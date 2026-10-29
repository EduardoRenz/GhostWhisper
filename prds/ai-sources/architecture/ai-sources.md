# Arquitetura — Fontes de IA múltiplas, fallback e circuit breaker

Especificação técnica do núcleo de fontes de IA. Comportamento de referência
no **Perssua 0.27.5** (BR-015) — o que existe lá e o que é desvio está no
[`../prd.md`](../prd.md) §6.

---

## 1. Componentes

```
domain/ai_source.rs                     # AiSource, AiSourceHealth, validação
domain/ai_route.rs                      # AiRouteScope, AiRoute, resolve_route()
domain/settings.rs                      # Settings.ai_sources / ai_routes
domain/assistant.rs                     # AssistantMode.source_id
domain/llm.rs                           # ProviderConfig (inalterado)

ports/ai_source_service.rs              # CRUD + chaves + saúde
ports/llm_controller.rs                 # complete / complete_for
ports/llm_provider_factory.rs           # (inalterado)
ports/event_publisher.rs                # emit_llm_source_used

application/ai_source_health.rs         # registro de saúde + circuit breaker
application/ai_source_service.rs        # implementação da porta
application/ai_source_migration.rs      # settings/chaves legadas → fontes
application/llm_router.rs               # cadeia de fallback
application/llm_timeouts.rs             # prazos por escopo (15 s / 60 s)

adapters/ai/deepseek_chat_bot_provider.rs
adapters/ai/default_chat_bot_provider_factory.rs
adapters/ai/openai_compatible.rs

ipc/ai_source_commands.rs
```

`domain` e `application` não conhecem adaptadores (R1); nada de `new` de
adaptador fora de `lib.rs` (R2); toda porta nova tem dobra de teste (R8).

## 2. Modelo de dados

### 2.1 `AiSource` (`domain/ai_source.rs`)

```rust
pub struct AiSource {
    pub id: String,            // uuid estável — permite rota por agente/tela
    pub kind: ProviderKind,    // openai | gemini | openrouter | deepseek | custom
    pub label: String,         // "OpenAI · Empresa" (nome de exibição)
    pub model: String,
    pub base_url: Option<String>,
    pub enabled: bool,         // participa do roteamento?
}
```

- `id` é **estável** e nunca é reaproveitado: a rota por escopo (BR-048) e as
  chaves do keyring apontam para ele.
- A **ordem do `Vec`** em `Settings.ai_sources` é a prioridade. `ai_sources[0]`
  é a primária. "Tornar primário" = mover para o índice 0; "reordenar" = mover
  na lista. A UI usa as funções puras `moveSource`/`moveSourceBy`/
  `makePrimarySource` (`src/core/domain/settings.ts`) e a **mesma** lista volta
  sem `onChange` quando nada muda (ISS-086).
- `secret_key()` → `llm.source.{id}` (uma chave por fonte, não por tipo).
- `effective_base_url()` → `base_url` explícito ou o padrão do provedor.

### 2.2 `Settings`

```rust
pub ai_sources: Vec<AiSource>,   // ordenada; [0] é a primária
pub ai_routes: Vec<AiRoute>,      // vazio por padrão (BR-048)
```

`provider`, `model` e `base_url` viram **campos legados** de leitura:

```rust
#[serde(default, rename = "provider", skip_serializing)]
pub legacy_provider: Option<ProviderKind>,
#[serde(default, rename = "model", skip_serializing)]
pub legacy_model: Option<String>,
#[serde(default, rename = "base_url", skip_serializing)]
pub legacy_base_url: Option<String>,
```

Eliminá-los do `struct` faria o `serde` ignorar as chaves antigas e **perder** a
configuração do usuário. Mantê-los como entrada de migração (e nunca
serializá-los) deixa `sanitized()` ser a **única** fonte de verdade depois do
primeiro load.

### 2.3 Migração (T02)

`application/ai_source_migration.rs` roda no boot (`lib.rs`):

1. Se `ai_sources` está vazia e há `legacy_provider` → cria **uma** `AiSource`
   (id novo, `kind` legado, modelo e `base_url` legados) e salva.
2. Para cada fonte migrada, se `llm.source.{id}` não existe no keyring e
   `llm.{kind}` existe → **copia** o valor. A chave legada **não** é apagada
   (o usuário pode ter outra instalação; apagar seria destrutivo).

Idempotente: rodar de novo não faz nada.

### 2.4 `AiRouteScope` / `AiRoute` (`domain/ai_route.rs`)

```rust
pub enum AiRouteScope {
    Default,                          // a primária
    Capability(AiCapability),         // Summary | Suggestions | Translation
    Agent(String),                    // mode_id de um agente
}

pub struct AiRoute { pub scope: AiRouteScope, pub source_id: String }
```

`resolve_route(routes, scope) -> Option<&str>` devolve a fonte do **escopo mais
específico** que casar: `Agent` > `Capability` > `Default`. Com `ai_routes`
vazio devolve `None` → usa a primária. É puro e testado.

`AssistantMode.source_id: Option<String>` guarda o vínculo por agente; a UI de
vínculo é da PRD futura (BR-048).

## 3. Portas

### 3.1 `AiSourceService` (`ports/ai_source_service.rs`)

```rust
pub trait AiSourceService: Send + Sync {
    fn list(&self) -> Result<Vec<AiSource>>;
    fn save(&self, sources: Vec<AiSource>) -> Result<()>;
    fn health(&self) -> Vec<AiSourceHealth>;
    fn set_key(&self, source_id: &str, key: &str) -> Result<()>;
    fn has_key(&self, source_id: &str) -> Result<bool>;
    fn clear_key(&self, source_id: &str) -> Result<()>;
    fn key_preview(&self, source_id: &str) -> Result<String>;
    fn validate(&self, source_id: &str) -> Result<String>;
    fn restore_defaults(&self) -> Result<Vec<AiSource>>;
}
```

`save` recebe a lista **inteira** (a ordem é a prioridade) — não há um comando
por operação, o que evita estado parcial no arquivo de settings.

`validate` testa **uma** fonte e devolve a resposta do provider (mesmo texto de
`LlmController::validate` hoje).

### 3.2 `LlmController` (evolução)

```rust
fn complete_for(&self, scope: &AiRouteScope, request: &CompletionRequest) -> Result<Completion>;
```

Implementação padrão: delega a `complete`. O `LlmRouter` resolve a rota do
escopo e começa a cadeia por ela; sem rota, começa pela primária.

### 3.3 `EventPublisher`

```rust
fn emit_llm_source_used(&self, usage: &LlmSourceUsage);
```

```rust
pub struct LlmSourceUsage {
    pub source_id: String,
    pub label: String,      // "DeepSeek" (a fonte)
    pub provider: String,   // "deepseek"
    pub model: String,      // "deepseek-reasoner"  (BR-051)
    pub scope: String,      // "agent:meeting" | "capability:summary" | "default"
    pub is_fallback: bool,
    pub latency_ms: u64,
}
```

Evento Tauri `llm_source_used`: a tela de Configurações usa para atualizar o
badge da fonte em tempo real e a **UI mostra o modelo que respondeu** (BR-051) —
o `scope` permite que cada tela mostre o modelo da **sua** tarefa.

## 4. Circuit breaker (`application/ai_source_health.rs`)

Registro em memória (`Arc<Mutex<HashMap<String, AiSourceHealth>>>`), injetado por
construtor — sem global mutável (R10).

```rust
pub enum AiSourceState { Unknown, Ready, Degraded, Failed }

pub struct AiSourceHealth {
    pub source_id: String,
    pub state: AiSourceState,
    pub consecutive_failures: u32,
    pub latency_ms: Option<u64>,
    pub last_error: Option<String>,
    pub is_fallback: bool,
    /// `Some(ms)` enquanto o circuito está aberto (o contador usa o `Clock`).
    pub open_until_ms: Option<i64>,
}
```

### 4.1 Política

| Evento | Ação |
|---|---|
| sucesso | `consecutive_failures = 0`, estado `Ready`, circuito fechado |
| falha **de autenticação** (401/403) | circuito **abre na hora** (`open_until_ms = now + 60 s`) |
| qualquer outra falha | `consecutive_failures += 1`; com o limiar em **1**, o circuito abre por **60 s** na primeira |
| circuito aberto e cooldown expirado | estado **meio-aberto**: a próxima chamada passa e é a tentativa de recuperação |
| meio-aberto + sucesso | circuito fecha (`Ready`) |
| meio-aberto + falha | circuito reabre por 60 s |

Enquanto o circuito está aberto, a fonte é **pulada** (sem gastar timeout de
rede) e o `state` é `Failed`. Com o limiar em 1 o estado `Degraded` fica
reservado (só aparece se `FAILURE_THRESHOLD` subir).

### 4.1.1 Prazos (`application::llm_timeouts`)

| Escopo | Prazo | Motivo |
|---|---|---|
| `Default`, agente, `Capability::Suggestions` | **15 s** | latência crítica: a falha precisa chegar ao breaker antes de o usuário desistir |
| `Capability::Summary`, `Capability::Translation` | **60 s** | textos maiores; não bloqueiam a conversa |

O prazo viaja no `CompletionRequest.timeout` e é aplicado pelo adaptador HTTP
(`request_timeout`). Um prazo definido pelo chamador (`.with_timeout`) vence a
política do escopo (`.with_default_timeout`).

### 4.2 Classificação do erro

`GhostWhisperError` ganha uma variante tipada:

```rust
#[error("provider respondeu {code}: {detail}")]
ProviderStatus { code: u16, detail: String },
```

Assim o breaker classifica por **código**, sem parsear a mensagem (o
`openai_compatible` atual embute o código na string). `map_transport_error` e o
caminho de status do `chat_completion` passam a usar a variante. O `detail` é
truncado (300 caracteres) e **nunca** contém a chave.

## 5. Cadeia de fallback (`application/llm_router.rs`)

```
complete(request, scope)
  │
  ├─ sources = settings.ai_sources habilitadas, na ordem
  ├─ start  = resolve_route(settings.ai_routes, scope) → índice da fonte
  │            (ou 0 = primária)
  └─ para source em sources[start..], em ordem:
        ├─ se circuito aberto → registra "pulado" e segue
        ├─ key = secrets.get(source.secret_key())?  (ausente/vazia → segue)
        ├─ config = ProviderConfig { kind, model, base_url, api_key }
        ├─ provider = factory.create(&config)?
        ├─ t0 = clock.now_ms()
        ├─ completion = provider.complete(request)      ← ou complete_streaming
        ├─ sucesso → registra saúde (latência), emite evento, devolve
        └─ falha   → registra falha; se a falha for de status,
                     devolve o erro; senão tenta a próxima
```

Regras:

- A **primeira** fonte que responde ganha; o evento carrega `is_fallback =
  (fonte != start)`.
- Erro só é devolvido quando **todas** as candidatas falharam; a mensagem
  agrega as causas (sem segredos).
- `complete_streaming` segue a mesma cadeia. A troca de fonte **no meio** do
  streaming **não** acontece: se a fonte	start` já emitiu texto, o erro sobe
  (evitar texto duplicado na UI). Fontes que falham **antes** de emitir
  qualquer pedaço entram no fallback normal.
- `validate` (teste de conexão) **não** faz fallback: valida a fonte pedida.

## 6. DeepSeek (BR-046)

`adapters/ai/deepseek_chat_bot_provider.rs` reusa
`openai_compatible::chat_completion` / `chat_completion_streaming`:

| Item | Valor |
|---|---|
| `base_url` | `https://api.deepseek.com/v1` |
| modelo padrão | `deepseek-chat` |
| outro modelo | `deepseek-reasoner` |
| autenticação | `Authorization: Bearer <key>` (mesma do OpenAI) |
| `temperature` | `deepseek-reasoner` **não** aceita `temperature` fora do padrão |

O `temperature_supported` do `openai_compatible` passa a incluir
`deepseek-reasoner`.

`default_chat_bot_provider_factory` ganha o braço `ProviderKind::DeepSeek`.

## 7. IPC (`ipc/ai_source_commands.rs`)

| Comando | Assinatura |
|---|---|
| `ai_sources_list` | `() -> Vec<AiSource>` |
| `ai_sources_save` | `(sources: Vec<AiSource>) -> ()` |
| `ai_sources_health` | `() -> Vec<AiSourceHealth>` |
| `ai_source_set_key` | `(source_id: String, key: String) -> ()` |
| `ai_source_has_key` | `(source_id: String) -> bool` |
| `ai_source_clear_key` | `(source_id: String) -> ()` |
| `ai_source_key_preview` | `(source_id: String) -> String` |
| `ai_source_validate` | `(source_id: String) -> String` |
| `ai_sources_restore_defaults` | `() -> Vec<AiSource>` |

A chave **nunca** volta ao frontend: `key_preview` devolve a versão mascarada
(`••••••••·abcd`, `domain::settings::redact_secret`).

`AppState` ganha `ai_sources: Arc<dyn AiSourceService>`.

## 8. Dobras de teste

| Porta | Dobra |
|---|---|
| `AiSourceService` | `InMemoryAiSourceService` (memória) + `FakeAiSourceService` (roteiro de respostas) |
| `ChatBotProviderFactory` | `CountingFactory` / `FailingFactory` (falha N vezes, depois responde) |
| `Clock` | `FixedClock` (já existe) |
| `EventPublisher` | `RecordingEventPublisher` |

Cenários cobertos: fallback na 1ª falha; circuito abre **na primeira** falha e a
fonte aberta é pulada; meio-aberto fecha com sucesso; todas as fontes falham ⇒
erro agregado sem vazamento; fonte sem chave é pulada; streaming não troca de
fonte no meio; prazo curto no caminho rápido × prazo longo no resumo.
