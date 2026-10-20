# PRD — Fontes de IA múltiplas e Configurações repaginadas

- **Versão:** 0.3
- **Data:** 2026-10-20
- **Status:** v0.1 implementada (tasks 01–09 `CONCLUDED`); **v0.2 implementada**
  (tasks 10–13 `CONCLUDED`) — aba "Geral & HUD" e acabamento visual (§10),
  aguardando validação visual do usuário; **v0.3 especificada** — redesign da
  aba "Fontes de IA" (§11, task 14 `PENDING`)
- **Relacionados:** [`../business-rules.md`](../business-rules.md),
  [`memory.md`](memory.md), [`issues.md`](issues.md),
  [`architecture/ai-sources.md`](architecture/ai-sources.md),
  [`architecture/settings-redesign.md`](architecture/settings-redesign.md),
  [`tasks/_tasks.md`](tasks/_tasks.md), PRDs anteriores:
  [`../mvp/prd.md`](../mvp/prd.md), [`../assistant/prd.md`](../assistant/prd.md)

---

## 1. Objetivo

Duas entregas que andam juntas:

1. **Fontes de IA múltiplas** — o usuário passa a cadastrar **várias** fontes de
   IA (provedor + modelo + chave), a reordenar para escolher qual é a
   **primária**, e a contar com **fallback automático** (com *circuit breaker*)
   quando a primária falhar. **DeepSeek** entra como provedor de primeira
   classe.
2. **Tela de Configurações repaginada** — a tela atual (5 abas, um seletor de
   provider, um campo de chave, um botão "Salvar" por aba) vira uma casca nova:
   cabeçalho próprio, barra de abas com ícones, conteúdo em *cards* e um
   **rodapé fixo** com `Salvar Alterações` / `Restaurar Padrões`.

Além disso, o modelo de dados fica **preparado** para a PRD seguinte: vincular
uma fonte específica a um agente ou a uma tela (rota por escopo). O vínculo em
si **não** tem UI nesta entrega.

## 2. Problema

Hoje o GhostWhisper tem **uma** fonte de IA. `Settings` guarda `provider`,
`model` e `base_url`; o `SecretStore` guarda **uma** chave por *tipo* de
provider (`llm.openai`, `llm.gemini`, …); e o `LlmRouter` resolve sempre essa
única configuração. Disso decorrem três problemas reais:

- **Sem resiliência.** Se a OpenAI estiver com rate limit (429) ou a chave
  estiver inválida (401), a sugestão, o resumo e a tradução **falham** — não há
  para onde cair.
- **Sem comparação.** O usuário não pode deixar duas contas do mesmo provedor
  (ex.: uma pessoal e uma da empresa) nem trocar de modelo sem apagar a
  configuração anterior.
- **Uma chave por tipo trava o uso.** Duas fontes OpenAI (modelos diferentes)
  disputariam a mesma entrada do keyring.

A tela de Configurações reflete a mesma limitação: um `<select>` de provider, um
`<input>` de modelo, um bloco de chave — e nenhum sinal de saúde (latência,
conexão, fallback).

## 3. Público-alvo e casos de uso

Mesmas personas do MVP ([`../mvp/prd.md`](../mvp/prd.md) §3), com foco em:

- **CU-01 — Nunca ficar sem sugestão:** a fonte primária falha (429/401/timeout)
  e o GhostWhisper responde pela próxima fonte da lista, sem interromper a
  sessão.
- **CU-02 — Escolher a melhor fonte por momento:** reunião com a OpenAI no
  limite de uso → o usuário promove o Gemini a primário (um clique) e volta a
  ordem depois.
- **CU-03 — Várias contas do mesmo provedor:** cadastrar "OpenAI · Pessoal"
  (`gpt-4o-mini`) e "OpenAI · Empresa" (`gpt-4.1`) com **chaves diferentes**,
  cada uma no keyring.
- **CU-04 — Entender o estado:** ver na própria tela qual fonte está conectada,
  qual está em fallback e qual está falhando, com a latência medida.
- **CU-05 — Usar o DeepSeek:** cadastrar o DeepSeek com a chave dele, sem
  precisar de `base_url` manual.
- **CU-06 — Configurar sem medo:** um único "Salvar Alterações" no rodapé;
  "Restaurar Padrões" com confirmação explícita.

## 4. Escopo

### 4.1 Entregue

- Cadastro, edição, exclusão e **reordenação** de fontes de IA; a ordem da
  lista é a ordem de prioridade e a **primeira é a primária**.
- **Chave por fonte** no keyring (`llm.source.{id}`), com versão mascarada,
  "Validar chave", "Trocar chave" e "Remover chave" — **inclusive para uma
  fonte que ainda está no rascunho** (ISS-039).
- **O modelo é digitado pelo usuário**, sem catálogo sugerido (BR-049).
- **Fallback automático** em todas as chamadas de IA (sugestões, resumo
  incremental, tradução ao vivo) com **circuit breaker** por fonte.
- **DeepSeek** como provedor de primeira classe.
- Sinal de saúde por fonte na tela (conectada + latência / degradada / em
  fallback / falhou).
- **Log de debug** com os dados do modelo usado na tarefa atual, com o nível
  de raciocínio quando existir (BR-050).
- **O modelo que respondeu é visível na UI** (BR-051): pill no card da
  sugestão e na linha do resumo, com a marca de fallback.
- Tela de Configurações repaginada com salvamento global.
- Rotas por escopo **no modelo de dados** (fonte vinculada por agente ou por
  capacidade), sem UI de vínculo.

### 4.2 Fora do escopo (PRD futura)

- UI para vincular uma fonte a um agente ou a uma tela.
- **Anthropic**: a API `/v1/messages` não é compatível com OpenAI e exige um
  adapter nativo (formato de request/response e de streaming diferentes).
  Entra em PRD própria.
- Endpoint local como fonte de LLM (`custom` continua existindo para
  Ollama/LM Studio/vLLM, com `base_url` manual).
- Múltiplas chaves para a **mesma** fonte (uma chave por fonte, só).
- Sincronização das configurações entre máquinas (BR-002).

## 5. Regras de produto

As regras canônicas vivem em [`../business-rules.md`](../business-rules.md):

- **BR-044** — Fontes de IA múltiplas e ordenadas; a primeira é a primária.
- **BR-045** — Fallback automático com circuit breaker em todas as chamadas de IA.
- **BR-046** — DeepSeek como fonte de primeira classe.
- **BR-047** — Configurações repaginadas com salvamento global.
- **BR-048** — Rotas de IA por escopo (modelo pronto, UI depois).
- **BR-049** — O modelo é digitado pelo usuário (sem catálogo).
- **BR-050** — Log de debug do modelo em uso (com nível de raciocínio).
- **BR-051** — O modelo que respondeu é visível na interface.
- **BR-062** — Aba "Geral & HUD": opacidade, ancoragem e densidade do notch (§10).

A revisão 0.3 (§11) é visual e reusa **BR-044, BR-045, BR-047 e BR-049**; não
cria regra de produto nova.

Regras pré-existentes que continuam valendo: BR-002 (sem backend), BR-003 (BYO
key no keyring), BR-005 (áudio local), BR-008 (reconstruir comportamento, não
código), BR-011/BR-013 (hexagonal + testabilidade), BR-015 (Perssua como
referência), BR-016 (documentação como fonte única).

## 6. Referência: o que o Perssua faz (BR-015, R12)

Extraído do `Perssua-0.27.5-x86_64.AppImage` → `resources/app.asar`
(`electron/settingsStore.js`, `electron/modelMigrations.js`,
`build/static/js/main.f180d025.js`, `~/.config/Perssua/settings.json`):

| Comportamento | Perssua | GhostWhisper (esta PRD) |
|---|---|---|
| Fonte ativa | `current_provider` (uma só) | lista `ai_sources` ordenada; a primeira é a primária |
| Modelo por provedor | `openai_model`, `gemini_model`, `openrouter_model`, `custom_api_model` | `AiSource.model` (um por fonte cadastrada) |
| Chave | `*_api_key` no `settings.json` (texto puro) | `llm.source.{id}` no keyring (BR-003) |
| Fonte de credencial | `openrouter_credential_source` (`trial`/`personal`) | — (não há trial no GhostWhisper) |
| Rota por capacidade | `analysis_model` + `analysis_model_source`, `translation_model` + `translation_model_source` | `Settings.ai_routes` + `AiRouteScope` (BR-048) |
| Reordenação / fallback / circuit breaker | **não existe** | **BR-044 / BR-045** (desvio) |
| DeepSeek de primeira classe | **não existe** (aparece só em `electron/preload.js` como provider de web search) | **BR-046** (desvio) |

### Desvios conscientes (R12)

1. **Lista ordenada + fallback + circuit breaker.** O Perssua tem uma fonte
   ativa só e nenhum mecanismo de degradação. O desvio tem motivo técnico
   explícito: sem fallback, um 429 da OpenAI derruba sugestão, resumo e
   tradução — e o GhostWhisper é usado **durante** a reunião, quando não há
   tempo para abrir as configurações e trocar a chave.
2. **DeepSeek de primeira classe.** No Perssua o DeepSeek entraria pelo
   `custom` (endpoint OpenAI-compatível) e exigiria `base_url` digitada à mão.
   Aqui ele é um provedor do catálogo, com `base_url` e modelo padrão.
3. **Chave no keyring, não no `settings.json`.** O Perssua grava
   `openai_api_key` no arquivo de settings em texto puro. Isso **viola BR-003**;
   a transposição troca a tecnologia mantendo o comportamento *visível* (a chave
   nunca é devolvida à UI em claro).

## 7. Arquitetura (resumo)

Detalhe em [`architecture/ai-sources.md`](architecture/ai-sources.md) e
[`architecture/settings-redesign.md`](architecture/settings-redesign.md).

```
domain/ai_source.rs        AiSource, AiSourceHealth, validação
domain/ai_route.rs         AiRouteScope, AiRoute, resolve_route()
domain/settings.rs         Settings.ai_sources / ai_routes; ProviderKind::DeepSeek
domain/assistant.rs        AssistantMode.source_id

ports/ai_source_service.rs CRUD + chaves + saúde das fontes
ports/llm_controller.rs    complete_for(scope, …)
ports/event_publisher.rs   emit_llm_source_used

application/ai_source_health.rs    registro de saúde + circuit breaker
application/ai_source_service.rs   implementação da porta
application/ai_source_migration.rs settings/chaves legadas → fontes
application/llm_router.rs           cadeia de fallback

adapters/ai/deepseek_chat_bot_provider.rs
ipc/ai_source_commands.rs

src/features/settings/AiSourcesSettings.tsx
src/features/settings/ProviderCard.tsx
src/features/settings/AddProviderForm.tsx
```

## 8. Critérios de sucesso

- Cadastrar 3 fontes (OpenAI, Gemini, DeepSeek), reordenar e ter a terceira como
  primária em menos de 30 s.
- Com a primária returning 429, a sugestão seguinte responde pela segunda
  fonte **sem erro na UI**.
- Uma fonte com 401 não é retentada a cada requisição (circuito abre).
- Duas fontes do mesmo provedor usam **chaves diferentes** no keyring.
- Settings antigas (com `provider`/`model`) continuam funcionando após o
  update, com a chave legada preservada.
- A tela de Configurações tem um único "Salvar Alterações" no rodapé.

## 9. Riscos

| Risco | Mitigação |
|---|---|
| Fallback silencioso mascarar uma configuração ruim | Evento `llm_source_used` com `is_fallback` + badge "Fallback Ativo" na tela |
| Latência acumulada quando todas as fontes caem | Circuit breaker + circuit aberto pula a fonte sem gastar timeout |
| Vazamento de chave em log/erro | Segredos só no keyring; erro de provider tipado e truncado (`ProviderStatus`) |
| Regressão em quem já tinha `provider`/`model` | Migração idempotente no boot + teste de settings legadas |
| Escopo grande demais | 9 tasks independentes, com a casca da tela (07) separada das fontes (08) |

## 10. Revisão 0.2 — Aba "Geral & HUD" e acabamento visual (2026-10-20)

O mockup do usuário reposiciona as configurações de **comportamento do HUD** numa
aba própria e pede que a casca da tela fique mais próxima da referência. O que já
existe (cards, abas com ícone, rodapé global — BR-047) **não muda de contrato**: a
v0.2 acrescenta a aba e ajusta a forma.

### 10.1 Escopo

| Item | Antes | Depois |
|---|---|---|
| Aba "Ajustes" | Dwell-hover + stealth | **"Geral & HUD"**: 4 cards do HUD + card de privacidade |
| Opacidade do overlay | — | **20 %–100 %** (padrão 75 %), aplicada à superfície do notch |
| Posição de ancoragem | posição livre lembrada | **Topo Central** (padrão) · **Canto Sup. Dir.** · **Flutuante Livre** |
| Densidade do notch | altura fixa (72 px) | **Compacto** · **Padrão** (padrão) · **Expandido** |
| Dwell-hover | na aba "Ajustes" | no card próprio da aba **Geral & HUD**, faixa **100 ms–1,5 s** |
| Abas | pílula preenchida | **ícone + rótulo com sublinhado** na aba ativa (mais próximo do mockup) |
| Conteúdo da aba | coluna única | **grade de 2 colunas** nos cards do HUD |

### 10.2 Fora do escopo

- Novas abas do mockup ("Visão de Tela", "Privacidade", "Audio & Microfone",
  "Modelo de IA & Whisper"): a reorganização de conteúdo fica para outra PRD. A
  tela mantém as **5 abas** atuais, com "Ajustes" renomeada para "Geral & HUD".
- Prévia ao vivo do notch dentro das configurações.
- Opacidade por contexto ("assistente em segundo plano"): o valor vale sempre.

### 10.3 Modelo de dados

Três campos em `Settings` (Rust + espelho TS), com validação e sanitização:

```rust
pub overlay_opacity: f32,   // 0.2..=1.0, padrão 0.75
pub notch_anchor: String,   // top_center | top_right | free  (padrão top_center)
pub hud_density: String,    // compact | standard | expanded  (padrão standard)
```

`dwell_ms` passa a aceitar **100–1500 ms** (era 300–2000 ms); o padrão continua
500 ms.

### 10.4 Comportamento da janela

- **Ancoragem:** o notch é posicionado na **área útil** do monitor primário —
  centralizado (`x = (work.w − window.w) / 2`) ou à direita (`x = work.right −
  window.w − margem`), sempre com uma margem superior pequena. Em **Flutuante
  Livre**, vale a posição lembrada (`overlay_position`) e o arraste.
- **Arrastar o notch** passa a ancoragem para **Flutuante Livre** e persiste a
  posição (antes o arraste só desligava a reancoragem pela base).
- **Densidade:** define a **altura da janela** (62/72/84 px lógicos) e as
  variáveis CSS da barra (altura, padding e gap).
- **Opacidade:** vira `--hud-alpha` e compõe o fundo da barra
  (`rgba(<rgb do token>, var(--hud-alpha))`); 100 % = superfície sólida.

### 10.5 Critérios de sucesso

- Salvar a aba nova aplica opacidade/ancoragem/densidade **sem reiniciar** o app.
- Trocar a ancoragem move o notch imediatamente para o canto escolhido.
- Arrastar o notch em "Topo Central" o torna "Flutuante Livre".
- Settings antigas (sem os três campos) carregam com os padrões.

### 10.6 Tasks

[`tasks/10-task.md`](tasks/10-task.md) (domínio/settings) →
[`11-task.md`](tasks/11-task.md) (janela) →
[`12-task.md`](tasks/12-task.md) (UI da aba e estilo) →
[`13-task.md`](tasks/13-task.md) (overlay + docs/verificação).

## 11. Revisão 0.3 — Redesign da aba "Fontes de IA" (2026-10-20)

### 11.1 Objetivo

Aproximar a aba **"Fontes de IA"** da referência visual do mockup do usuário
(2026-10-20), usando o padrão **Obsidian HUD** já aplicado em "Geral & HUD"
(BR-035, BR-062). A entrega é de **frontend e CSS**: não altera o comportamento
de fontes, chaves, saúde, fallback, reordenação ou salvamento.

O resultado deve parecer a mesma família visual da casca atual — cabeçalho,
abas com ícone, cards e rodapé global — e não uma tela nova.

### 11.2 Escopo

- **Cabeçalho de seção** "Provedores de IA Ativos" fora do card, com ícone,
  subtítulo curto de prioridade e, à direita, a contagem de fontes + estado do
  keyring.
- **Card de fonte** reorganizado em três blocos:
  - cabeçalho com tile do provedor, nome, pill do modelo, pills de
    primário/fallback e status de saúde;
  - descrição curta do papel da fonte (primária, fallback ou reserva);
  - rodapé com a chave mascarada + origem no keyring e as ações principais.
- **Ações do card** com hierarquia: "Validar chave", "Trocar chave" e
  "Remover chave" como ações compactas; "Tornar primário" no cabeçalho;
  "Editar" e "Excluir fonte" como ações secundárias.
- **Formulário de adição** em card com grid de duas colunas, campo de chave
  opcional, helper de keyring e rodapé com "Cancelar" e "Adicionar".
- **Reuso** dos componentes e tokens existentes; nada de hex solto ou de um
  segundo sistema visual.
- **Testes** de estrutura, estados e fluxos já existentes, sem regressão de
  comportamento.

### 11.3 Fora do escopo

- Novos modos de autenticação, cartões de conta ou qualquer recurso que não
  exista no produto hoje. A aba não ganha cards fictícios.
- Novos provedores (ex.: Anthropic) ou novos adapters.
- Catálogo de modelos: o modelo continua **digitado pelo usuário** (BR-049).
- Reorganização das abas: a tela mantém as **5 abas** atuais (BR-047).
- Alterações em Rust, IPC, `Settings`, keyring, circuit breaker ou roteamento.
- Prévia ao vivo do notch dentro das configurações.
- Mudança no rodapé global, no cabeçalho da tela ou nas demais abas.

### 11.4 Estrutura visual

```
┌ Provedores de IA Ativos ────────────── 2 fontes · Keyring Seguro Ativo ┐
│                                                                        │
│  ▣  OpenAI   [gpt-4o-mini] [Primário]        ✓ Conectada · 142 ms     │
│     Fonte primária para sugestões, resumos e tradução.                 │
│  ────────────────────────────────────────────────────────────────────  │
│  🔑 ••••••••sFwA   🔒 Keyring do Sistema   [Validar] [Trocar] [Remover]│
└────────────────────────────────────────────────────────────────────────┘
┌ Provedores de IA Ativos (continuação) ─────────────────────────────────┐
│  ▣  DeepSeek [deepseek-chat] [Fallback ativo]   ○ Nunca usada          │
│     Assume se a fonte anterior falhar.        [Tornar primário]        │
│  ────────────────────────────────────────────────────────────────────  │
│  🔑 ••••••••k9P2   🔒 Keyring do Sistema   [Validar] [Trocar] [Remover]│
└────────────────────────────────────────────────────────────────────────┘
┌ ➕ Adicionar Nova Fonte de IA ───────────── Armazenamento seguro ──────┐
│  Provedor [Google Gemini ▾]        Modelo [gemini-2.0-flash        ]   │
│  Chave de API [sk-…                                        ]           │
│  🔒 A chave será criptografada e armazenada no keyring do sistema.    │
│                                            [Cancelar] [Adicionar]      │
└────────────────────────────────────────────────────────────────────────┘
```

O wireframe é ilustrativo: os dados vêm das settings atuais, não do mockup. A
lista nunca fica vazia (a última fonte não pode ser excluída).

### 11.5 Comportamento e estados

#### Cabeçalho da seção

- Título: **"Provedores de IA Ativos"**.
- Subtítulo: explica a prioridade em uma linha — "A primeira fonte responde; a
  próxima assume automaticamente se ela falhar."
- Nota à direita: `N fontes cadastradas · Keyring Seguro Ativo` (ou "Chaves só
  nesta sessão" quando o keyring não for persistente).
- A contagem usa singular/plural corretamente.

#### Card de fonte

| Estado | UI |
|---|---|
| Primária | borda ciano suave + glow; pill **"Primário"**; tile ciano |
| Não primária | borda neutra; tile violeta suave; ação "Tornar primário" |
| Saudável | chip esmeralda com check: `Conectada · Latência 142 ms` |
| Instável | chip âmbar: `Instável` |
| Falhou | chip vermelho: `Falhou`, com `title` contendo o último erro |
| Nunca usada | chip neutro: `Nunca usada` |
| Fallback ativo | pill neutra **"Fallback ativo"** |
| Sem chave | linha da chave mostra "Sem chave configurada"; ação "Adicionar chave" |
| Operação em andamento | botões desabilitados (`busy`) |

- O modelo aparece como pill mono, truncado com reticências se necessário.
- A descrição é curta e derivada do papel da fonte, não um texto de marketing
  por provedor:
  - primária: "Fonte primária para sugestões, resumos e tradução.";
  - fallback: "Assume automaticamente se a fonte anterior falhar.";
  - reserva: "Fonte de reserva na ordem de prioridade."
- O arraste continua reordenando. O grip fica visível, mas discreto.
- "Editar" abre o editor inline de modelo/base URL; "Excluir fonte" continua
  condicionado a `canRemove`.
- "Trocar chave" abre o input inline; "Salvar chave" grava no keyring pelo IPC
  existente e **não** salva as settings.
- "Remover chave" limpa o keyring e mantém a fonte na lista.
- Os textos de status existentes são preservados para não quebrar testes e
  leitores de tela: `Conectada · Latência N ms`, `Instável`, `Falhou`,
  `Nunca usada`, `Primário`, `Fallback ativo`.

#### Formulário de adição

- Provedor e modelo em **duas colunas** no desktop; uma coluna abaixo de 760 px.
- O modelo continua livre (BR-049), com placeholder `nome-do-modelo`.
- Campo **"Chave de API" opcional**, do tipo `password`; se preenchido, a chave
  é gravada no keyring pelo `setSourceKey` existente quando a fonte é
  adicionada ao rascunho.
- A validação da chave continua no card ("Validar chave"); o formulário não
  valida antes de adicionar. O botão "Validar chave" do mockup fica de fora
  desta entrega: o IPC atual valida uma chave já gravada, e a fonte ainda é
  rascunho.
- O botão do rodapé diz **"Adicionar"**, não "Salvar Provedor": a fonte entra no
  rascunho e só é persistida pelo "Salvar Alterações" global.
- "Cancelar" limpa provedor, modelo, base URL e chave. "Adicionar" insere a
  fonte no rascunho e limpa o formulário.
- O rodapé global "Salvar Alterações" continua sendo o único caminho de
  persistência da lista.
- O `base_url` só aparece para o provedor `custom`, como hoje.

### 11.6 Regras de UI

- **Sem marca de terceiros:** ícones genéricos desenhados no projeto (BR-008);
  nada de logos de terceiros.
- **Tokens:** usar `var(--oh-*)` de `src/styles/tokens.css`; não repetir hex no
  CSS (BR-035).
- **Componentes reutilizáveis:** `Pill`, `IconButton`, `GripIcon`, `CheckIcon`,
  `BoltIcon`/`SparkleIcon`, e as classes `.settings__section-header`,
  `.settings__section-title`, `.settings__section-note`, `.settings__sources`,
  `.settings__card`, `.settings__field`, `.settings__hint`.
- **Novos modificadores** (`.settings__source-*`, `.settings__key-*`,
  `.settings__add-*`, `.settings__button--ghost`/`--compact`) estão detalhados
  em [`architecture/settings-redesign.md`](architecture/settings-redesign.md)
  §10.4; não alterar os estilos globais que as outras abas usam.
- **Acessibilidade:** todo botão de ícone tem `aria-label`/`title`; o status
  não depende só de cor (texto sempre presente); foco visível preservado.
- **Responsivo:** em 1020×720 a aba deve caber sem rolagem horizontal; em
  760×560 o rodapé do card empilha e as ações quebram em linhas. A rolagem
  vertical fica no `.settings__body`.

### 11.7 Critérios de sucesso

- A aba mostra o cabeçalho de seção, os cards e o formulário na hierarquia do
  wireframe, sem alterar o comportamento existente.
- Cadastrar, reordenar, promover, editar, validar, trocar e remover chave
  continuam funcionando como antes.
- Uma fonte sem chave mostra o estado vazio correto e permite adicioná-la.
- A chave digitada no formulário de adição vai para o keyring sem salvar as
  settings.
- As 5 abas continuam iguais; nenhum card ou seção nova fora do escopo de
  fontes.
- `pnpm test`, `tsc --noEmit` e `pnpm build` verdes.

### 11.8 Testes e verificação

Atualizar `src/features/settings/AiSourcesSettings.test.tsx` e criar
`src/features/settings/ProviderCard.test.tsx` para cobrir:

- cabeçalho com contagem e estado do keyring;
- card primário, fallback e sem chave;
- chip de saúde com latência e os estados instável/falhou/nunca usada;
- "Tornar primário", reordenação e exclusão condicionada;
- validação, troca e remoção de chave;
- adição de fonte com e sem chave, e limpeza pelo "Cancelar".

Verificação manual em `pnpm dev` com
`?window=settings&tab=sources`, em 1020×720 e 760×560.

### 11.9 Riscos

| Risco | Mitigação |
|---|---|
| Cards mais altos estourarem a janela de 720 px | Rodapé compacto, descrição em uma linha e rolagem no `.settings__body` |
| Mudanças de CSS afetarem as outras abas | Novas classes/modificadores; não alterar `.settings__button` nem `.settings__card` sem prefixo |
| Testes existentes dependerem da estrutura antiga | Atualizar seletores por papel/texto, preservando os rótulos atuais |
| Chave órfã ao adicionar e excluir antes de salvar | Mesmo comportamento atual do keyring; documentado, sem mudança de contrato |
| Fidelidade ao mockup levar a dados fictícios | A tela só renderiza fontes e estados reais; nada de dados inventados |

### 11.10 Tasks

[`tasks/14-task.md`](tasks/14-task.md). A arquitetura de UI (DOM, classes e
mapeamento de estados) está em
[`architecture/settings-redesign.md`](architecture/settings-redesign.md) §10.
