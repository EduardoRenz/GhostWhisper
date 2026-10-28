# Arquitetura — Modo de Sugestão (gate por blocos + classificador)

Especificação técnica do PRD [`../prd.md`](../prd.md). Cobre o domínio de blocos,
a porta do classificador, o gate no agendador, os eventos/IPC, a UI e a POC Laya.
Contrato de produto em **BR-092**…**BR-101**.

> **Revisão 2026-10-26 (taxonomia por intenção):** as cues antigas
> `hesitation`/`lost_thread`/`invitation`/`silence`/`no_suggestion` foram
> substituídas por `question`, `comment`, `already_answered`, `no_action` e
> `insufficient_context`. Não existe mais sonda de bloco aberto. O
> classificador recebe o contexto do agente e as últimas 5 perguntas
> respondidas (BR-094/BR-101). Seções históricas abaixo foram ajustadas onde
> o contrato mudou.

---

## 1. Princípios e restrições

- **Hexagonal (R1–R6):** `domain` puro; `application` orquestra; `ports` define
  contratos; `adapters` implementam; `ipc` traduz; `lib.rs` faz o wiring.
- **Sem I/O no núcleo (R7):** blocos, cues e decisão são funções puras; tempo via
  `Clock`; classificação via porta.
- **Toda porta tem dobra de teste (R8):** `FakeSuggestionCueClassifier`,
  `InMemoryTranscriptionRepository`, `FakeClock`.
- **Dependência opt-in:** Laya entra como o crate `laya-onnx` atrás da feature
  `laya`; o modelo ONNX fica fora do bundle. O build padrão não o inclui.
- **Modo automático intocado:** o caminho atual do `spawn_suggestion_scheduler`
  continua idêntico; o classificador é um segundo caminho.
- **Privacidade:** nenhum texto de transcrição/token em log; só metadados.

---

## 2. Modelo de domínio

Arquivo novo: `src-tauri/src/domain/suggestion_gate.rs` (ou
`suggestion_cue.rs`; escolher um nome e registrar no `domain/mod.rs`).

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SuggestionMode {
    Auto,
    Classifier,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClassifierSensitivity {
    Low,      // threshold 0.85
    Balanced, // threshold 0.70 (default)
    High,     // threshold 0.55
}

impl ClassifierSensitivity {
    pub fn threshold(self) -> f32 {
        match self {
            Self::Low => 0.85,
            Self::Balanced => 0.70,
            Self::High => 0.55,
        }
    }

    /// Preset de cues positivas (o avançado pode restringir).
    pub fn default_allowed_cues(self) -> &'static [SuggestionCue] {
        match self {
            Self::Low => &[SuggestionCue::Question, SuggestionCue::Hesitation],
            Self::Balanced => &[
                SuggestionCue::Question,
                SuggestionCue::Hesitation,
                SuggestionCue::LostThread,
                SuggestionCue::Silence,
            ],
            Self::High => &[
                SuggestionCue::Question,
                SuggestionCue::Hesitation,
                SuggestionCue::LostThread,
                SuggestionCue::Invitation,
                SuggestionCue::Silence,
            ],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuggestionCue {
    Question,
    Hesitation,
    LostThread,
    Invitation,
    Silence,
    NoSuggestion,
    InsufficientContext,
}

impl SuggestionCue {
    /// Cues que podem gerar sugestão (as demais nunca geram).
    pub fn can_suggest(self) -> bool {
        !matches!(self, Self::NoSuggestion | Self::InsufficientContext)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BlockTurn {
    pub speaker: Speaker,
    pub text: String,
    pub start_ms: i64,
    pub end_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConversationBlock {
    /// Id determinístico: `blk_{session}_{start}_{end}_{last_segment_id}`.
    pub id: String,
    pub session_id: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub turns: Vec<BlockTurn>,
    pub me_chars: usize,
    pub they_chars: usize,
    pub is_open: bool,
}

impl ConversationBlock {
    pub fn has_both_speakers(&self) -> bool {
        self.turns.iter().any(|t| t.speaker == Speaker::Me)
            && self.turns.iter().any(|t| t.speaker == Speaker::They)
    }

    /// Bloco elegível para classificação geral (fechado + dois falantes).
    pub fn is_classifiable(&self) -> bool {
        !self.is_open && self.has_both_speakers()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockConfig {
    pub gap_ms: i64,          // default 3_000
    pub max_block_ms: i64,    // default 45_000
    pub max_turns: usize,     // default 12
    pub context_blocks: usize,// default 3
    pub min_chars: usize,     // default 8 por falante
}
```

### 2.1 Construtor de blocos (puro)

```rust
pub fn build_blocks(
    session_id: &str,
    segments: &[TranscriptSegment],
    config: &BlockConfig,
    now_ms: i64,
) -> Vec<ConversationBlock>
```

Regras:

1. Filtrar `is_final` e texto não vazio; ordenar por `(start_ms, end_ms)`.
2. Abrir bloco no primeiro segmento.
3. Fechar o bloco quando:
   - `next.start_ms - current.end_ms >= gap_ms`; **ou**
   - `current.end_ms - block.start_ms >= max_block_ms`; **ou**
   - `block.turns.len() >= max_turns`.
4. O último bloco fica `is_open = true` até o gap de silêncio ser observado;
   `now_ms` permite fechar por silêncio mesmo sem novo segmento.
5. `me_chars`/`they_chars` somam apenas texto (trim).
6. `id` determinístico a partir de `session_id`, `start_ms`, `end_ms` e do
   `last_segment_id`; o mesmo conjunto de segmentos gera o mesmo id.
7. Blocos com menos de `min_chars` de um dos falantes não são classificáveis
   (mas permanecem para contexto/evidência).

### 2.2 Heurística automática (backend `heuristic`)

```rust
pub fn heuristic_cue(
    block: &ConversationBlock,
    config: &HeuristicConfig,
    prior_questions: &[PriorQuestion],
) -> SuggestionCue
```

Ordem de precedência:

1. **`insufficient_context`** — menos de 3 palavras no total.
2. **`no_action`** — small talk (cumprimento/agradecimento) ou fala sem
   pergunta/convite.
3. **`already_answered`** — a última pergunta de THEY já foi respondida no
   próprio bloco (ME falou depois) ou repete uma pergunta do histórico. A
   comparação é lexical e independente de idioma (R23): Jaccard do texto
   completo (≥ 0,60), maior Jaccard entre janelas deslizantes (24 tokens, passo
   6) e contenção (`|A∩B|/min(|A|,|B|)` ≥ 0,75, com ≥ 6 tokens em comum e ≥ 8
   no lado menor) para cauda/fragmento. O histórico guarda a pergunta do bloco
   inteiro (até 600 caracteres) e o prompt mostra até 180 por item
   (ISS-081/task 18; a janela fixa da ISS-076 foi removida).
4. **`question`** — regra **legada** do backend “Regras locais”: `?`, palavra
   interrogativa ou pedido. **Não adicione novas palavras/idiomas** (R23); esse
   backend será removido (ISS-078/task 16).
5. **`comment`** — regra legada por convite explícito.
6. **`no_action`** — caso contrário.

**Debounce estrutural (ISS-083/task 19):** um bloco fechado só é classificado
depois de **3 s** de silêncio e com o canal **THEY** parado
(`TranscriptActivity`, por sessão/canal). Se THEY começar a falar, o gate segura;
quando a pergunta real finaliza, classifica esse bloco. ME não segura.

**Caminho efetivo (Laya, R23):** o modelo multilíngue decide
`question`/`comment`/`no_action`/`insufficient_context` lendo a **conversa
inteira** (sem separar contexto/alvo) com prompt genérico (sem exemplos/palavras
de idioma). O alvo é construído por `target_across_blocks`, que junta blocos
contíguos divididos por tempo (pergunta longa em vários turnos); um ME não
significativo **depois** do alvo não impede, um ME antes (resposta) impede
(ISS-080/task 17). O contexto do agente é só **nome/categoria/contexto** — o
`suggestion_prompt` de geração não vai ao classificador. Só `already_answered`
usa a similaridade lexical (Jaccard) do histórico, que não codifica idioma. Laya
indisponível/abstenção ⇒ não sugerir. As inclusões de palavras-chave foram
revertidas (ISS-077/task 15).

---

## 3. Porta do classificador

Arquivo novo: `src-tauri/src/ports/suggestion_cue_classifier.rs`.

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassificationRequest {
    pub session_id: String,
    pub block: ConversationBlock,
    /// Blocos anteriores (mais antigo → mais novo), no máximo
    /// `context_blocks`.
    pub context: Vec<ConversationBlock>,
    pub locale: String,
    pub mode_id: String,
    /// `true` quando é a sonda de bloco aberto (hesitação).
    pub is_open_probe: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ClassificationOutcome {
    pub block_id: String,
    pub session_id: String,
    pub cue: SuggestionCue,
    pub should_suggest: bool,
    pub confidence: f32,       // 0.0..=1.0
    /// Trecho curto (≤ 160 chars) da fala que pesou na decisão.
    pub evidence: String,
    /// Foco curto para o gerador (≤ 240 chars); opcional.
    pub focus: Option<String>,
    pub backend: String,       // "heuristic" | "laya" | ...
    pub model: Option<String>, // "multilingual", ...
    pub latency_ms: u64,
    pub block_start_ms: i64,
    pub block_end_ms: i64,
    pub turn_count: usize,
    pub classified_at_ms: i64,
    /// `true` quando a decisão veio de uma abstenção/baixa confiança.
    pub abstained: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum ClassifierError {
    #[error("classificador desligado")]
    Disabled,
    #[error("classificador indisponível")]
    Unavailable,
    #[error("timeout do classificador")]
    Timeout,
    #[error("resposta inválida do classificador")]
    InvalidResponse,
    #[error("erro do classificador: {0}")]
    Other(String),
}

pub trait SuggestionCueClassifier: Send + Sync {
    fn classify(&self, request: &ClassificationRequest) -> Result<ClassificationOutcome>;
    /// Identidade para a UI/settings; não é segredo.
    fn backend_id(&self) -> &'static str;
}
```

- O trait é **síncrono**: o agendador já roda em thread própria; o adaptador
  HTTP usa timeout de `ureq`.
- O adaptador **não** recebe `SuggestionMode`; o gate decide se chama.
- `confidence` e `should_suggest` são normalizados no adaptador; o gate aplica
  o limiar e a allowlist de cues.

### 3.1 Fake

`FakeSuggestionCueClassifier` (testes):

- fila de resultados/erros;
- registra requests (`Mutex<Vec<ClassificationRequest>>`);
- `backend_id()` configurável;
- permite `delay` simulado? Não; o núcleo não dorme. Para timeout, o fake
  retorna `ClassifierError::Timeout`.

---

## 4. Serviço de gate

Arquivo novo: `src-tauri/src/application/suggestion_gate_service.rs`.
Porta de entrada nova: `src-tauri/src/ports/suggestion_gate.rs`.

```rust
pub trait SuggestionGate: Send + Sync {
    /// Classifica o bloco mais recente (ou o bloco aberto, se `force_open`).
    fn classify_latest(
        &self,
        session_id: &str,
        force_open: bool,
    ) -> Result<ClassificationOutcome>;

    /// Última classificação da sessão (memória; não chama o classificador).
    fn last(&self, session_id: &str) -> Result<Option<ClassificationOutcome>>;

    /// Decide se a classificação deve gerar uma sugestão agora.
    fn should_generate(
        &self,
        outcome: &ClassificationOutcome,
        now_ms: i64,
    ) -> Result<GateDecision>;
}

pub enum GateDecision {
    Generate { focus: Option<String> },
    Skip { reason: GateSkipReason },
}

pub enum GateSkipReason {
    Disabled,
    NoSession,
    CueNotAllowed,
    BelowConfidence,
    Cooldown { remaining_ms: i64 },
    AlreadyGenerated,
    Generating,
    EmptyFocus,
}
```

Responsabilidades:

- carregar settings e sessão;
- ler falas recentes via
  `TranscriptionsRepository::list_recent_by_session(session_id, since_ms, limit)`
  (novo método; SQL filtra `is_final = 1`, `end_ms >= ?`, ordena por
  `start_ms`, limita);
- montar blocos;
- selecionar o bloco mais recente (fechado + dois falantes) ou o aberto
  (`force_open`);
- montar `ClassificationRequest` com os `context_blocks` anteriores;
- chamar `SuggestionCueClassifier`;
- normalizar (`confidence.clamp(0,1)`, `evidence` truncada, `should_suggest`
  coerente com a cue);
- guardar a última por sessão (`Mutex<HashMap<String, ClassificationOutcome>>`);
- publicar `suggestion_classification`;
- aplicar cooldown/dedupe no `should_generate`.

### 4.1 Decisão pura

Para testar sem I/O, extrair:

```rust
pub struct GateTick {
    pub enabled: bool,
    pub has_session: bool,
    pub outcome: ClassificationOutcome,
    pub cue_allowed: bool,
    pub confidence_min: f32,
    pub cooldown_ms: i64,
    pub elapsed_since_last_suggestion_ms: i64,
    pub last_generated_block_id: Option<String>,
    pub generating: bool,
}

pub fn decide(tick: &GateTick) -> GateDecision
```

Regras (todas precisam valer para `Generate`):

1. `enabled && has_session`;
2. `!generating`;
3. `outcome.should_suggest`;
4. `outcome.cue.can_suggest()` e `cue_allowed`;
5. `outcome.confidence >= confidence_min`;
6. `cooldown` vencido;
7. `outcome.block_id != last_generated_block_id`.

### 4.2 Decisão pendente

- Se 3–7 passarem mas o cooldown não venceu, o gate guarda
  `pending: Option<(ClassificationOutcome, focus)>` e retorna
  `Skip { Cooldown }`.
- No próximo tick, se o cooldown venceu e o bloco ainda é o mais recente,
  gera **sem reclassificar**.
- Um bloco novo substitui a pendência; se o novo for negativo, limpa.
- `last_generated_block_id` é atualizado após a geração (sucesso ou `skipped`).

---

## 5. Agendador

`src-tauri/src/lib.rs` (`spawn_suggestion_scheduler`) passa a ter três ramos.

### 5.1 `off`

- `enabled = false`: não classifica, não gera.
- Publica estado ocioso (`SuggestionClassifierStateEvent { state: Paused }` ou
  cadence `enabled=false`).
- Mantém “Sugerir agora” funcionando (via IPC, fora do scheduler).

### 5.2 `auto`

- Caminho atual **sem alteração**:
  - `should_trigger` / `cadence` existentes;
  - `suggestion_interval_secs` como cooldown;
  - `SuggestionRequest::ephemeral(...).with_fast_first(true)`.
- Não importa nem chama `SuggestionCueClassifier`.
- Teste de regressão: auto nunca chama o fake.

### 5.3 `classifier`

Pseudo-fluxo por tick (1 s):

```
settings = load()
if !settings.suggestion_auto_refresh: publish paused; continue
session = current_session()
if none: publish no_session; continue
segments = list_recent_by_session(session, now-180s, 40)
blocks = build_blocks(...)
closed = newest closed + both speakers
if closed and closed.id != last_classified_block_id:
    outcome = gate.classify_latest(session, false)
    last_classified_block_id = closed.id
    // classificação publicada dentro do serviço
    if open probe enabled and no closed event:
        ...
decision = gate.should_generate(outcome, now_ms)
if decision == Generate:
    publish generating=true
    suggestions.generate(SuggestionRequest { trigger: Classifier, cue, focus, ... })
    last_generated_block_id = outcome.block_id
    publish generating=false + fresh state
```

Histórico e contexto:

- o gate carrega no mínimo 8 blocos anteriores (janela de 5 min/120 segmentos);
- mantém por sessão os títulos das perguntas que geraram sugestão;
- envia até `suggestion_answered_questions` (padrão 5) no
  `ClassificationRequest.prior_questions` (usado pelo Jaccard; o prompt também
  recebe a lista, mas a decisão `already_answered` é da guarda lexical);
- o alvo junta blocos contíguos (`target_across_blocks`) e o Laya lê o contexto
  + bloco atual inteiros (sem `[CONTEXTO]/[ALVO]`), para uma pergunta longa não
  ser classificada por um fragmento;
- o Laya injeta os títulos no prompt; o heurístico compara similaridade.

O scheduler **não** deve classificar a cada tick: usa `last_classified_block_id`
e o `block_id` determinístico.

---

## 6. Eventos e IPC

### 6.1 Evento novo

`src-tauri/src/ports/event_publisher.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SuggestionClassifierState {
    Paused,
    NoSession,
    Listening,
    Analyzing,
    Ready,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SuggestionClassificationEvent {
    pub session_id: String,
    pub state: SuggestionClassifierState,
    pub outcome: Option<ClassificationOutcome>,
    pub error: Option<String>,
    pub generating: bool,
}
```

- `EventPublisher::emit_suggestion_classification(&event)`.
- `TauriEventPublisher` emite `suggestion_classification`.
- `NullEventPublisher`/test doubles: default no-op.
- A UI assina `suggestion_classification` e hidrata com
  `suggestion_classification_last`.

Evento de download do modelo:

```rust
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SuggestionModelDownloadEvent {
    pub model_id: String,
    pub model_version: String,
    pub phase: String,        // downloading | verifying | installing | ready | failed
    pub file: Option<String>,
    pub received_bytes: u64,
    pub total_bytes: Option<u64>,
    pub error: Option<String>,
}
```

- `EventPublisher::emit_suggestion_model_download`.
- Tauri emite `suggestion_model_download`; a UI atualiza o dialog.

### 6.2 Comandos

`src-tauri/src/ipc/suggestion_commands.rs`:

```rust
#[tauri::command(async)]
pub fn suggestion_classification_last(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Option<ClassificationOutcome>>;

#[tauri::command(async)]
pub fn suggestion_classify_now(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<ClassificationOutcome>;
```

- `suggestion_classify_now` **não** gera sugestão; só classifica e publica
  (botão “Classificar agora” / painel de teste).
- Model manager:
  ```rust
  classifier_model_catalog() -> Result<Vec<ClassifierModelManifest>>;
  classifier_model_installed() -> Result<Vec<InstalledClassifierModel>>;
  classifier_model_install(model_version: String) -> Result<()>;
  classifier_model_activate(model_version: String) -> Result<()>;
  classifier_model_remove(model_version: String) -> Result<()>;
  ```
  O `install` baixa o manifesto do catálogo e os `files[]`; progresso via
  evento, não via retorno bloqueante.
- Registrar em `lib.rs` no `invoke_handler`.

### 6.3 Tipos TS

`src/core/domain/suggestion.ts` (ou `assistant.ts`):

```ts
export type SuggestionMode = "auto" | "classifier";
export type SuggestionCue =
  | "question"
  | "comment"
  | "already_answered"
  | "no_action"
  | "insufficient_context";

export interface SuggestionClassification {
  block_id: string;
  session_id: string;
  cue: SuggestionCue;
  should_suggest: boolean;
  confidence: number;
  evidence: string;
  focus: string | null;
  backend: string;
  model: string | null;
  latency_ms: number;
  block_start_ms: number;
  block_end_ms: number;
  turn_count: number;
  classified_at_ms: number;
  abstained: boolean;
}
```

- `SuggestionTrigger` ganha `"classifier"`.
- `Suggestion` ganha `cue?: SuggestionCue | null`.

---

## 7. Settings

### 7.1 Rust (`domain/settings.rs`)

```rust
#[serde(default)]
pub suggestion_mode: SuggestionMode, // default Auto

#[serde(default = "default_enabled")]
pub suggestion_show_classification: bool, // true

#[serde(default = "default_classifier_backend")]
pub suggestion_classifier_backend: String, // "heuristic" | "laya"

/// Sensibilidade: deriva o limiar (0.85/0.70/0.55) e o preset de cues.
#[serde(default)]
pub suggestion_classifier_sensitivity: ClassifierSensitivity, // Balanced

/// Modelo do model manager; `model_version` vazio = versão ativa.
#[serde(default = "default_classifier_model_id")]
pub suggestion_classifier_model_id: String, // "laya-multilingual"

#[serde(default)]
pub suggestion_classifier_model_version: String,

/// Override de dev (diretório ONNX). Vazio = model manager/LAYA_ONNX_DIR.
#[serde(default)]
pub suggestion_classifier_model_path: String,

/// Nunca baixar sozinho.
#[serde(default)]
pub suggestion_classifier_auto_download: bool, // false

/// Catálogo de modelos (lista de manifestos/versões).
#[serde(default = "default_classifier_catalog_url")]
pub suggestion_classifier_model_catalog_url: String, // oficial

/// Checkpoint do Laya: só `multilingual` (English não é exposto).
#[serde(default = "default_classifier_checkpoint")]
pub suggestion_classifier_checkpoint: String,

#[serde(default = "default_classifier_cooldown")]
pub suggestion_classifier_cooldown_secs: u32, // 15

#[serde(default = "default_classifier_timeout")]
pub suggestion_classifier_timeout_ms: u32, // 3000

#[serde(default = "default_classifier_fallback")]
pub suggestion_classifier_fallback: String, // "skip"

#[serde(default = "default_block_gap")]
pub suggestion_block_gap_ms: u32, // 3000

#[serde(default = "default_block_max")]
pub suggestion_block_max_ms: u32, // 45000

#[serde(default = "default_block_turns")]
pub suggestion_block_max_turns: u32, // 12

#[serde(default = "default_context_blocks")]
pub suggestion_context_blocks: u32, // 8 (faixa 8–20)

#[serde(default = "default_answered_questions")]
pub suggestion_answered_questions: u32, // 5 (faixa 0–20)

#[serde(default = "default_allowed_cues")]
pub suggestion_allowed_cues: Vec<SuggestionCue>,
```

- `sanitize()`: clamps de todos os campos; caminho vazio → usa env; checkpoint
  desconhecido → `multilingual`; cues desconhecidas são descartadas;
  `already_answered`/`no_action`/`insufficient_context` nunca entram na
  allowlist.
- `suggestion_auto_refresh` continua no struct; é o interruptor mestre.
- Sem token; a única URL é o catálogo de modelos. O Laya roda in-process via
  `laya-onnx`; o modelo é local.
- A feature Cargo `laya` é necessária para o backend `laya`; sem ela, o valor
  `laya` é rejeitado/sanitizado para `heuristic` com aviso na UI.

### 7.2 TS (`core/domain/settings.ts`)

- Espelhar os campos com defaults idênticos.
- `defaultSettings()` atualizado.
- Teste de paridade Rust/TS? O projeto não tem; manter os valores em testes
  separados.

### 7.3 Migração

- JSON antigo sem `suggestion_mode` → `auto` (serde default).
- Nenhuma remoção de campo; nenhuma migração de banco.
- Settings com `suggestion_auto_refresh=false` → desligado, independente do
  modo; ao religar, o modo salvo vale.

---

## 8. UI

### 8.1 Componentes

```
src/features/suggestions/
  SuggestionsWindow.tsx          # escolhe header conforme o modo
  AutoSuggestControl.tsx         # inalterado (auto)
  ClassifierSuggestControl.tsx   # NOVO: status do classificador
  LastClassificationStrip.tsx    # NOVO: faixa persistente
  SuggestionCard.tsx             # + selo de cue (opcional)
  ClassificationDetails.tsx      # NOVO: detalhes expandidos (ou dentro do strip)

src/core/application/
  useSuggestionClassification.ts # NOVO: evento + last + classifyNow
  useSuggestionCadence.ts        # inalterado (auto)
```

### 8.2 Header

`SuggestionsWindow`:

```tsx
const mode = settings.suggestion_mode;
const paused = !settings.suggestion_auto_refresh;

{mode === "auto" ? (
  <AutoSuggestControl cadence={cadence} paused={paused} onToggle={toggleAuto} />
) : (
  <ClassifierSuggestControl
    paused={paused}
    sessionActive={Boolean(state.session_id)}
    classification={classification}
    state={classifierState}
    onToggle={toggleAuto}
    onClassifyNow={classifyNow}
  />
)}
```

- `ClassifierSuggestControl` estados:
  - `paused` → “Pausado”;
  - `!sessionActive` → “Sem escuta”;
  - `generating` → “Gerando…”;
  - `state === "analyzing"` → “Analisando…”;
  - `state === "listening"` → “Ouvindo bloco…”;
  - `classification?.should_suggest` → “Vai sugerir: {cue}”;
  - `classification` → “{cue}: nada a sugerir”;
  - `state === "error"` → “Falha”;
  - default → “Aguardando bloco”.
- Sem anel/cronômetro.
- `aria-live="polite"` no pill de estado.
- “Sugerir agora” permanece igual; tooltip “Ignora o classificador”.

### 8.3 Faixa de última classificação

`LastClassificationStrip`:

- Props: `classification: SuggestionClassification | null`,
  `mode`, `state`, `error`, `onClassifyNow`, `onRetry`, `onOpenSettings`.
- Colapsada (1 linha):
  `Última classificação  [PERGUNTA]  0,86  ·  há 8s  ·  Laya  ·  212 ms`
- Expandida:
  - cue + explicação curta (label);
  - decisão (Sim/Não) e motivo (`Abaixo do limiar`, `Cooldown 4s`,
    `Categoria desligada`, `Já sugerido`);
  - evidência (quote curta);
  - bloco `00:12–00:28 · 5 turnos`;
  - backend/modelo/latência;
  - botões “Classificar agora” e “Tentar de novo” (erro);
  - link “Ajustar nas Configurações”.
- Estados vazios:
  - sem classificação e modo classificador: “Nenhuma classificação ainda…”;
  - modo automático: “Modo automático: o classificador está desligado.”;
  - erro: “Classificador indisponível…”.
- Sempre renderizada se `suggestion_show_classification`; não empurra o cartão
  para fora (uma linha + expansão).
- Não usa `t` para dados (cue é traduzido por chave; evidência é dado).

### 8.4 Cartão da sugestão

- Se `suggestion.trigger === "classifier"`, mostrar um selo discreto com a cue
  (ex.: “Pergunta”) ao lado do título.
- Não mudar o layout de resposta rápida/completa.

### 8.5 l10n

Chaves novas (pt/en, R18–R22):

```
settings_tab_sources               # valor: "Fontes de IA & Modelos"
suggestion_activation_title
suggestion_activation_badge
suggestion_activation_subtitle
suggestion_activation_enabled
suggestion_activation_active
suggestion_activation_inactive
suggestion_mode_classifier_title
suggestion_mode_classifier_subtitle
suggestion_mode_classifier_description
suggestion_mode_auto_title
suggestion_mode_auto_subtitle
suggestion_mode_auto_description
suggestion_model_selected
suggestion_model_laya
suggestion_model_heuristic
suggestion_model_manage
suggestion_model_status_installed
suggestion_model_status_missing
suggestion_model_status_downloading
suggestion_model_status_corrupt
suggestion_model_status_feature_off
suggestion_model_download
suggestion_model_version
suggestion_model_activate
suggestion_model_delete
suggestion_sensitivity
suggestion_sensitivity_low
suggestion_sensitivity_balanced
suggestion_sensitivity_high
suggestion_cadence
suggestion_blind_trigger
suggestion_advanced
suggestion_classifier_backend
suggestion_classifier_backend_heuristic
suggestion_classifier_backend_laya
suggestion_classifier_model_path
suggestion_classifier_checkpoint
suggestion_classifier_model_ready
suggestion_classifier_model_missing
suggestion_classifier_feature_disabled
suggestion_classifier_effective_threshold
suggestion_classifier_cooldown
suggestion_classifier_timeout
suggestion_classifier_fallback
suggestion_classifier_fallback_skip
suggestion_classifier_fallback_auto
suggestion_block_gap
suggestion_block_max_duration
suggestion_block_max_turns
suggestion_block_context
suggestion_cues_title
suggestion_cue_question
suggestion_cue_comment
suggestion_cue_already_answered
suggestion_cue_no_action
suggestion_cue_insufficient_context
suggestion_last_classification
suggestion_last_classification_empty
suggestion_last_classification_auto
suggestion_last_classification_error
suggestion_classification_details
suggestion_classification_evidence
suggestion_classification_block
suggestion_classification_latency
suggestion_classification_confidence
suggestion_classify_now
suggestion_classifier_listening
suggestion_classifier_analyzing
suggestion_classifier_ready
suggestion_classifier_generating
suggestion_classifier_paused
suggestion_classifier_no_session
suggestion_classifier_error
suggestion_classifier_will_suggest
suggestion_classifier_nothing
suggestion_test_classifier
suggestion_test_hint
suggestion_privacy_note
```

---

## 9. Configurações — bloco no topo de “Fontes de IA & Modelos”

Sem aba “Sugestões” no MVP. O modo vive no topo da aba de fontes/modelos
(`SettingsPage` → `tab === "sources"`), acima da lista de fontes de IA.

### 9.1 Componentes

```
src/features/settings/
  SuggestionsActivationBlock.tsx   # NOVO: os dois cards + toggle mestre
  SuggestionsAdvancedDialog.tsx    # NOVO: avançado + teste
  ClassifierModelManagerDialog.tsx # NOVO: download/versões/checksum
  AiSourcesSettings.tsx            # recebe o bloco no topo (ou SettingsPage)
  CadenceSettings.tsx              # remove o card de sugestões
```

- O bloco é renderizado **antes** da lista de fontes; não substitui a lista.
- A aba `sources` é renomeada de “Fontes de IA” para “Fontes de IA & Modelos”
  (l10n key `settings_tab_sources`).

### 9.2 Layout do bloco (mockup)

```
┌──────────────────────────────────────────────────────────────────────┐
│ (✦) Modo de ativação de sugestões  [Estratégia de trigger]  [ON/OFF] │
│                                       Disparo inteligente em tempo real│
├──────────────────────────────┬───────────────────────────────────────┤
│ (◉) Modelo de classificação  │ (○) Sugestão por tempo                │
│     Laya · Classificador     │     Intervalo periódico contínuo       │
│     contextual               │                                       │
│ Descrição: dispara quando    │ Descrição: cadência fixa, sem          │
│ detecta fala/dúvida relevante│ classificação de contexto              │
│                              │                                       │
│ Modelo selecionado [Laya ▼]  │ Cadência de ciclo [8s][15s][30s]      │
│ Status: ● instalado · 1,2 GB │                                       │
│ Sensibilidade [Baixa|Bal.|Alta]│ Disparo cego · sem filtro semântico │
│ [Avançado ⚙]                 │                                       │
└──────────────────────────────┴───────────────────────────────────────┘
```

- Clicar no card seleciona `suggestion_mode` (`classifier` | `auto`).
- O toggle mestre (`suggestion_auto_refresh`) fica no header do bloco; off
  desabilita os dois cards sem perder o modo.
- O card selecionado ganha borda/glow de accent e badge **“Ativa”**; o outro
  fica neutro (sem badge ou “Inativa”).
- O status do modelo é curto; “Baixar” abre o model manager.
- “Avançado ⚙” abre o dialog com o restante.

### 9.3 Cards

**Modelo de classificação (classifier)**

- Título/subtítulo: “Modelo de classificação” / “Laya · Classificador
  contextual”.
- Descrição: “Dispara sugestões e intervenções automaticamente apenas quando
  detecta momentos oportunos de fala ou dúvidas relevantes.”
- **Modelo selecionado**: `Laya (Padrão)` | `Regras locais`.
  - `Laya (Padrão)` = backend `laya` + checkpoint `multilingual`.
  - `Regras locais` = backend `heuristic` (offline, sem modelo).
  - O checkpoint English **não** é exposto (o multilingual cobre pt/en e ocupa
    menos espaço).
  - “Gerenciar modelos…” abre o dialog de download/versões.
- **Status**: instalado/versão/tamanho, não instalado, baixando, corrompido,
  feature desligada.
- **Sensibilidade**: `Baixa` (0,85) | `Balanceada` (0,70) | `Alta` (0,55).
  - O valor é `suggestion_classifier_sensitivity`; o limiar é derivado.
  - O preset de cues é o da §3.6 do PRD; o “Avançado” pode restringir.
- **Avançado** (dialog): cooldown, timeout, fallback, blocos, categorias,
  sonda, mostrar última classificação, teste.

**Sugestão por tempo (auto)**

- Título/subtítulo: “Sugestão por tempo” / “Intervalo periódico contínuo”.
- Descrição: “Gera sugestões automaticamente a cada intervalo pré-definido de
  segundos ou minutos sem depender de classificação de contexto.”
- **Cadência de ciclo**: `8s` | `15s` | `30s` (mapeia `suggestion_interval_secs`).
- Nota: “Disparo cego · sem filtro semântico”.
- O intervalo de sugestões sai de `CadenceSettings` (Geral & HUD); o resumo
  automático continua lá.

### 9.4 Dialog “Avançado”

```
Avançado do classificador
├─ Cooldown [15s]
├─ Timeout [3000ms]
├─ Em erro [Não sugerir | Usar automático]
├─ Blocos
│  ├─ Silêncio para fechar [3s]
│  ├─ Duração máxima [45s]
│  ├─ Turnos máximos [12]
│  └─ Contexto [3 blocos]
├─ Categorias que geram sugestão
│  └─ [x] Pergunta [x] Hesitação [x] Perda do fio [ ] Convite [x] Silêncio
├─ [x] Sondar bloco aberto (4s)
├─ [x] Mostrar última classificação
└─ Teste
   ├─ [Usar último bloco] [Colar exemplo] [Testar]
   ├─ Resultado: cue, confiança, latência, backend
   └─ Aviso de privacidade
```

- O dialog usa o `draft` de settings; “Salvar alterações” do rodapé persiste.
- O teste chama `suggestion_classify_now` com `block_text` opcional; não gera
  sugestão.
- Sem token; a única URL é o catálogo de modelos; a inferência é local via
  `laya-onnx`.

### 9.5 Model manager dialog

```
Modelo do classificador
├─ Recomendado: Laya Multilingual (1,2 GB, Apache-2.0)
├─ Versões instaladas
│  ├─ 2026.10.0 · ativo · [Apagar]
│  └─ 2026.09.0 · [Ativar] [Apagar]
├─ [Baixar/Atualizar] progresso + cancelar
├─ checksum: verificado / falhou
└─ [Abrir pasta] (dev)
```

- O download é sob confirmação; progresso via evento
  `suggestion_model_download`.
- “Ativar” troca o ponteiro `current`; “Apagar” remove versões não ativas.
- O botão “Baixar” só aparece quando o modelo recomendado não está instalado.

### 9.6 Estados e acessibilidade

- Modo off: cards desabilitados, header mostra “Desligado”; clicar em um card
  pergunta se quer ligar.
- Sem sessão: o bloco continua configurável (é settings).
- `aria-pressed`/`role="radio"` nos cards; foco visível; labels traduzidos.
- O status do modelo é `aria-live="polite"`.
- Nenhum texto hardcoded; chaves novas em pt/en.

### 9.7 Model manager (contrato técnico)

Novo módulo `src-tauri/src/domain/classifier_model.rs`:

```rust
pub struct ClassifierModelFile {
    pub path: String,       // "model.onnx", "tokenizer/tokenizer.json"
    pub url: String,
    pub size_bytes: u64,
    pub sha256: String,     // hex minúsculo (64 chars)
}

pub struct ClassifierModelManifest {
    pub schema: u32,               // 1
    pub model_id: String,          // "laya-multilingual"
    pub version: String,           // "0.1.0"
    pub checkpoint: String,        // "multilingual"
    pub layout: String,            // "onnx"
    pub label: Option<String>,
    pub size_bytes: u64,           // soma dos files[] (0 = soma calculada)
    pub license: String,           // "Apache-2.0"
    pub files: Vec<ClassifierModelFile>,
    pub calibration: ClassifierModelCalibration,
    pub metrics: ClassifierModelMetrics,
    pub created_at: String,
}

pub struct InstalledClassifierModel {
    pub model_id: String,
    pub version: String,
    pub checkpoint: String,
    pub path: String,
    pub size_bytes: u64,
    pub active: bool,
    pub verified: bool,
    pub installed_at_ms: i64,
}
```

O `manifest.json` (gravado pelo manager dentro da versão) é a fonte de
verdade do caminho/versão; `files[]` **não** inclui o manifesto. Caminhos são
relativos e validados (sem `..`/absoluto); `sha256` precisa ser hex minúsculo.

Porta `src-tauri/src/ports/classifier_model_manager.rs`:

```rust
pub trait ClassifierModelManager: Send + Sync {
    fn catalog(&self) -> Result<ClassifierModelCatalog, ClassifierModelError>;
    fn installed(&self) -> Result<Vec<InstalledClassifierModel>, ClassifierModelError>;
    fn active(&self) -> Option<InstalledClassifierModel>;
    fn active_path(&self) -> Option<PathBuf>;
    fn manifest(&self, model_id: &str, version: &str) -> Result<ClassifierModelManifest, …>;
    /// Única operação que baixa; instala sem ativar.
    fn install(&self, model_id: &str, version: &str) -> Result<InstalledClassifierModel, …>;
    fn activate(&self, model_id: &str, version: &str) -> Result<InstalledClassifierModel, …>;
    fn remove(&self, model_id: &str, version: &str) -> Result<(), …>;
}
```

`catalog()` devolve o `ClassifierModelCatalog` (o primeiro modelo é o
recomendado); `install` compõe com `activate` no comando IPC.

Adaptador `LocalClassifierModelManager`:

- catálogo estático (`catalog.json`) com manifestos/versões; `GET` só quando o
  usuário abre “Gerenciar modelos” ou clica “Verificar atualizações”;
- diretório base: `app_data/models/laya`;
- download **arquivo a arquivo** (`manifest.files[]`) via `ureq` com `Range`
  opcional; sem tar/zip;
- baixa para `<version>.part/<path>`, verifica o SHA-256 de cada arquivo e só
  então renomeia `<version>.part/` → `<version>/`;
- grava `manifest.json` **dentro** da versão (`<id>-<versão>/manifest.json`);
- ponteiro `current` (arquivo texto) gravado por `current.part` + rename;
- evento `suggestion_model_download` com `phase` (`downloading`, `verifying`,
  `installing`, `ready`, `failed`) + `bytes_done`/`bytes_total`/`file`/
  `file_index`/`file_count`;
- erros tipados (`Unavailable`, `UnknownModel`, `NotInstalled`,
  `InvalidManifest`, `ChecksumMismatch`, `NoSpace`, `Cancelled`, `ActiveModel`,
  `Io`).

Configuração:

- `suggestion_classifier_model_id` (`laya-multilingual` default);
- `suggestion_classifier_model_version` (vazio = ativo);
- `suggestion_classifier_model_path` (override de dev; vazio = model manager);
- `suggestion_classifier_model_catalog_url` (padrão oficial);
- `suggestion_classifier_auto_download` **false** (nunca baixar sozinho).

O backend `laya` resolve o diretório ativo no boot; se não houver, fica
`Unavailable` e o heurístico assume. O `manifest.json` é a fonte de verdade para
o caminho/versão; um fine-tuned só precisa de um novo manifesto.

---

## 10. POC Laya (SDK Rust `laya-onnx`)

### 10.1 Modelo e artefato

- Checkpoint: **`convaiinnovations/laya-multilingual`** (mmBERT-base, 322M,
  1024 tokens, 100+ idiomas). É o default para pt/en.
- O modelo raiz `convaiinnovations/laya` é English-only e **não é exposto**;
  só o multilingual entra na UI (cobre pt/en e ocupa menos espaço).
- O crate roda **ONNX**; o checkpoint PyTorch precisa ser exportado uma vez:
  `python laya-rust/tools/export_onnx.py --model multilingual`.
  O artefato ONNX ainda não está publicado no Hugging Face.
- O diretório exportado tem `model.onnx`, `model.onnx.data`,
  `rl_agent_config.json` e `tokenizer/`.
- Tamanho ~1,2 GB; **não** entra no bundle. No caminho de usuário final, o
  model manager baixa/instala/ativa; `suggestion_classifier_model_path` e
  `LAYA_ONNX_DIR` ficam como override de dev.

### 10.2 Estrutura da POC

```
poc/laya-classifier/
  README.md                 # export ONNX (uma vez) + como rodar o eval
  fixtures/blocks.jsonl     # ≥ 24 blocos rotulados (pt/en)

src-tauri/examples/laya_eval.rs   # eval Rust, feature-gated
```

- Sem `docker-compose`, sem `laya-serve`, sem token HTTP.
- O eval é um `example` do próprio crate `src-tauri`; usa o adaptador e o
  mapeamento reais (sem duplicar a lógica).
- Rodar: `cargo run --example laya_eval --features laya`.
- Python só aparece no passo de exportação do ONNX.

### 10.3 Adaptador Rust

`src-tauri/src/adapters/ai/laya_suggestion_cue_classifier.rs`:

```rust
pub struct LayaSuggestionCueClassifier {
    tx: std::sync::mpsc::Sender<LayaRequest>,
    timeout: Duration,
    min_confidence: f32,
    checkpoint: LayaCheckpoint,
    model_path: PathBuf,
}

struct LayaRequest {
    state: serde_json::Value,
    reply: std::sync::mpsc::Sender<Result<LayaResult, LayaError>>,
}
```

- A worker thread é dona do `LayaEngine` (carregado uma vez) e recebe requests.
- `classify` envia e espera com `recv_timeout`; timeout → `ClassifierError::Timeout`
  e o resultado atrasado é descartado.
- `LayaEngine` é `Send + Sync`; o worker evita bloquear o agendador e permite
  aplicar o timeout.
- Perguntas: **uma** `choice` (`cue`) com as 7 opções. **Não** usar `noul`.
- `QuestionSet`:
  ```rust
  let questions = QuestionSet::new().with(
      "cue",
      Question::choice("Qual é o próximo movimento do copiloto?", [
          ("question", "THEY fez uma pergunta direta ou pediu esclarecimento."),
          ("hesitation", "ME está travando, repetindo, se corrigindo ou sem terminar."),
          ("lost_thread", "ME perdeu o fio da conversa ou não sabe o que dizer."),
          ("invitation", "THEY fez um ponto que convida ME a responder."),
          ("silence", "ME parou de falar e a conversa ficou em silêncio."),
          ("no_suggestion", "Nada útil a acrescentar; ficar em silêncio."),
          ("insufficient_context", "Contexto insuficiente para decidir."),
      ])?,
  )?;
  ```
- `PredictOptions::new().min_confidence(MinConfidence::scalar(threshold)?)`.
- Parse:
  - `let answer = result.get("cue")`; `let choice = answer.as_choice()`;
  - `cue = choice.choice`;
  - `confidence = answer.answer_confidence()`;
  - `abstained = answer.abstention() == Some("abstained") ||
    answer.low_confidence()`;
  - `should_suggest = !abstained && confidence >= threshold &&
    cue.can_suggest()`;
- `evidence`: última fala de THEY (ou ME, se a cue for hesitação/silêncio),
  truncada em 160 chars.
- `focus`: rótulo curto por cue (ex.: “responder à pergunta”, “continuar a
  frase”, “retomar o fio”, “puxar a próxima fala”); não é texto do modelo.
- `model`/`backend_id`: `"laya"` + `"multilingual"`.
- Erros: `LayaError::MissingFile`/`DirectoryNotFound` → `Unavailable`;
  `InvalidQuestion`/`NoMarkers` → `InvalidResponse`; timeout → `Timeout`.
- Logs: `block_id`, `cue`, `confidence`, `latency_ms`, `checkpoint`; **nunca**
  o texto.

### 10.4 Cargo e feature

O crate não está no crates.io e o repositório upstream não tem `Cargo.toml` na
raiz. Decisão: **submódulo git** `vendor/laya` (Apache-2.0) + dependência por
caminho, sem conta/publicação.

No `src-tauri/Cargo.toml`:

```toml
[features]
laya = ["dep:laya-onnx", "dep:ort"]

[dependencies]
laya-onnx = { path = "../vendor/laya/laya-rust", optional = true, default-features = false }
ort = { version = "=2.0.0-rc.13", optional = true }  # pin exigido pelo SDK
```

- `git submodule update --init` antes do build.
- `default-features = false` desliga o download HTTP do SDK; o modelo é local.
- `ort` pinado em `=2.0.0-rc.13`: o SDK usa a API antiga (`Session::builder()`
  sem `Environment`); o resolver puxaria rc.15.
- O `ort` ainda usa o default `download-binaries`; no release, vendorizar o
  ONNX Runtime ou usar `load-dynamic`.
- **MSRV:** `rust-version = "1.88"` (edition 2024 do SDK). O bump ativou lints
  de MSRV no Clippy; os avisos pré-existentes foram corrigidos.
- O build padrão não compila o `laya-onnx`; o backend Laya só existe com
  `--features laya`.

> **Falta o artefato** (ISS-059): o ONNX não está no HF; exportar uma vez com
> `python3 vendor/laya/laya-rust/tools/export_onnx.py --model multilingual`.
> Ver [`../../poc/laya-classifier/README.md`](../../../poc/laya-classifier/README.md).

### 10.5 Seleção do backend

`src-tauri/src/lib.rs`:

```rust
#[cfg(feature = "laya")]
fn laya_backend(settings: &Settings) -> Arc<dyn SuggestionCueClassifier> { ... }

let classifier: Arc<dyn SuggestionCueClassifier> = match settings
    .suggestion_classifier_backend
    .as_str()
{
    #[cfg(feature = "laya")]
    "laya" => laya_backend(&settings),
    _ => Arc::new(HeuristicSuggestionCueClassifier::new(...)),
};
```

- Implementação real: `SettingsSuggestionCueClassifier` lê
  `suggestion_classifier_backend` **a cada bloco** e delega. Sem engine/modelo,
  `"laya"` cai no heurístico com um aviso único; o `outcome.backend` mostra qual
  respondeu.
- O gate recebe `reconfigure(config_from_settings(&settings))` a cada tick do
  scheduler: sensibilidade/cues/cooldown salvos valem sem reiniciar.
- O caminho do modelo é resolvido no boot (override de dev ou `active()` do
  manager); o engine em si carrega **na primeira classificação** (lazy), para
  não atrasar o boot. Depois de instalar/ativar um modelo, reiniciar o app para
  o caminho ser resolvido.
- Trocar de backend não exige reinício quando o engine já foi resolvido no
  boot; sem modelo no boot, `"laya"` cai no heurístico até reiniciar.

### 10.6 Avaliação

- As métricas vivem em `domain/classifier_eval.rs` (puras): acurácia de
  `should_suggest`, FP/FN, P/R/F1 por cue, matriz de confusão e p50/p95.
- O harness lê `poc/laya-classifier/fixtures/blocks.jsonl` (28 blocos, 4 por
  cue). Hoje ele roda o **heurístico** como baseline em um teste `#[ignore]`
  (`cargo test --lib eval_heuristic_baseline_on_fixtures -- --ignored --nocapture`).
- Desvio: o plano era `examples/laya_eval.rs`, mas os módulos do `lib.rs` são
  privados; o harness virou teste interno. Quando o engine existir, o eval do
  Laya usa o mesmo `evaluate`/fixtures.
- Gate: acurácia ≥ 0.80, FP ≤ 0.15, p95 ≤ 1 s CPU / 300 ms GPU.
- O model card do `laya-multilingual` avisa: **não calibrado** (ECE ~0.31) e
  fraco em English vs. o checkpoint English. Calibrar o limiar nos fixtures; o
  English não é exposto no produto (só multilingual).
- Manter `choice` com poucas opções (7) — o limite seguro do model card é ~20.

---

## 11. Testes

### 11.1 Rust

- `domain::suggestion_gate::tests`:
  - fecha por gap, duração, turnos;
  - último bloco aberto;
  - id determinístico;
  - `has_both_speakers` / `is_classifiable`;
  - heurístico: pergunta, hesitação (pt/en), silêncio, nada, insuficiente.
- `application::suggestion_gate_service::tests`:
  - classifica bloco fechado novo;
  - não classifica o mesmo bloco duas vezes;
  - sonda aberta com força;
  - última classificação por sessão;
  - decisão: cue desligada, confiança baixa, cooldown, duplicado, gerando;
  - erro/timeout → skip + estado de erro;
  - fallback `auto` (se implementado);
  - reset ao trocar de sessão.
- `ipc::suggestion_commands::tests`:
  - `last` vazio/algum;
  - `classify_now` não gera sugestão.
- `adapters::ai::heuristic_...`:
  - fixtures de texto.
- `adapters::ai::laya_...`:
  - mapeamento puro `choice` → `SuggestionCue` + confiança + abstenção;
  - modelo ausente/inválido → `Unavailable`;
  - timeout do worker → `Timeout` (sem bloquear o scheduler);
  - teste de integração com ONNX real é `#[ignore]` e exige `LAYA_ONNX_DIR`.
- `adapters::models::local_classifier_model_manager`:
  - instala/ativa/remove;
  - checksum inválido → `ChecksumMismatch`;
  - `.part` não vira `current`;
  - download sem confirmação não começa;
  - rollback para versão anterior.
- Scheduler:
  - auto nunca chama o fake;
  - classifier chama uma vez por bloco;
  - off não chama;
  - manual bypassa.

### 11.2 TypeScript

- `ClassifierSuggestControl.test.tsx`: todos os estados.
- `SuggestionsWindow.test.tsx`: header auto vs classifier; “Sugerir agora”.
- `LastClassificationStrip.test.tsx`: vazio, auto, sucesso, negativa, erro,
  expansão, `classifyNow`.
- `useSuggestionClassification.test.tsx`: hidrata, evento, reset de sessão,
  erro.
- `SuggestionsActivationBlock.test.tsx`: cards, toggle, sensibilidade,
  cadência, modelo/status.
- `SuggestionsAdvancedDialog.test.tsx`: campos, validação, teste.
- `ClassifierModelManagerDialog.test.tsx`: baixar/verificar/ativar/apagar.
- l10n: catálogo pt/en com as chaves novas.

### 11.3 Manual (Linux)

1. Sessão real com microfone + áudio do sistema.
2. Conversa com pergunta de THEY → `question` → sugestão.
3. ME hesita (“é… como eu falo…”) → `hesitation` → sugestão.
4. Silêncio após ME → `silence` → sugestão no tempo configurado.
5. Conversa fiada → `no_suggestion` → faixa negativa, sem LLM.
6. Trocar para automático → header volta ao anel; sem chamadas ao classificador.
7. Abrir/fechar a janela → última classificação hidrata.
8. Baixar/verificar/ativar modelo pelo card → status muda; apagar → heurístico.
9. Erro do Laya (caminho inválido/feature off) → faixa de erro; “Sugerir
   agora” continua.
10. Ver logs: nenhum texto de transcrição.

---

## 12. Compatibilidade e migração

- `suggestion_mode` ausente → `Auto`.
- `suggestion_auto_refresh=false` → desligado; modo preservado.
- `suggestion_interval_secs` antigo (10/20/60) é preservado; a UI mostra um
  chip “Custom” além dos presets 8/15/30. Nenhuma migração forçada.
- `suggestion_classifier_*` ausente → defaults seguros; nenhum modelo baixado.
- Campos novos com `serde(default)`; nenhum `deny_unknown_fields`.
- TS `defaultSettings()` e testes de settings atualizados.
- Nenhuma migração de SQLite.
- Nenhum evento antigo removido; `suggestion_cadence` continua existindo para
  o modo auto.
- `SuggestionTrigger` ganha `classifier`; a UI antiga que não conhece o valor
  deve tratar como `auto`/manual (fallback) — versões são empacotadas juntas,
  então não há risco de mismatch no release.

---

## 13. Mapa de arquivos

| Arquivo | Ação |
|---|---|
| `src-tauri/src/domain/suggestion_gate.rs` | novo |
| `src-tauri/src/domain/mod.rs` | registrar módulo |
| `src-tauri/src/ports/suggestion_cue_classifier.rs` | novo |
| `src-tauri/src/ports/suggestion_gate.rs` | novo |
| `src-tauri/src/ports/mod.rs` | registrar |
| `src-tauri/src/ports/repositories.rs` | `list_recent_by_session` |
| `src-tauri/src/ports/event_publisher.rs` | evento de classificação |
| `src-tauri/src/application/suggestion_gate_service.rs` | novo |
| `src-tauri/src/application/mod.rs` | registrar |
| `src-tauri/src/adapters/ai/heuristic_suggestion_cue_classifier.rs` | novo |
| `src-tauri/src/adapters/ai/laya_suggestion_cue_classifier.rs` | novo (POC) |
| `src-tauri/src/adapters/ai/mod.rs` | registrar |
| `src-tauri/src/adapters/persistence/sqlite/local_transcriptions_repository.rs` | método novo |
| `src-tauri/src/adapters/persistence/memory/in_memory_transcriptions_repository.rs` | método novo |
| `src-tauri/src/domain/settings.rs` | campos + sanitize |
| `src-tauri/src/ipc/suggestion_commands.rs` | comandos |
| `src-tauri/src/lib.rs` | wiring + scheduler |
| `src-tauri/src/state.rs` | `Arc<dyn SuggestionGate>` |
| `src-tauri/Cargo.toml` | feature `laya` + `laya-onnx` opcional |
| `src-tauri/examples/laya_eval.rs` | eval Rust (fixtures) |
| `poc/laya-classifier/fixtures/blocks.jsonl` | fixtures da POC |
| `poc/laya-classifier/README.md` | passo de export ONNX + eval |
| `prds/suggestion-mode/architecture/fine-tuning.md` | contrato de fine-tuning |
| `src/core/domain/settings.ts` | campos |
| `src/core/domain/assistant.ts` / `suggestion.ts` | tipos |
| `src/core/application/useSuggestionClassification.ts` | novo |
| `src/features/suggestions/ClassifierSuggestControl.tsx` | novo |
| `src/features/suggestions/LastClassificationStrip.tsx` | novo |
| `src/features/suggestions/SuggestionsWindow.tsx` | header/condicional |
| `src/features/suggestions/SuggestionCard.tsx` | selo de cue |
| `src/features/settings/SuggestionsActivationBlock.tsx` | novo (topo de sources) |
| `src/features/settings/SuggestionsAdvancedDialog.tsx` | novo |
| `src/features/settings/ClassifierModelManagerDialog.tsx` | novo |
| `src/features/settings/SettingsPage.tsx` | montar bloco em `sources` |
| `src/features/settings/AiSourcesSettings.tsx` | receber o bloco no topo |
| `src/features/settings/CadenceSettings.tsx` | remover card de sugestão |
| `src-tauri/src/domain/classifier_model.rs` | manifesto/instalação |
| `src-tauri/src/ports/classifier_model_manager.rs` | porta do model manager |
| `src-tauri/src/adapters/models/local_classifier_model_manager.rs` | download/checksum |
| `src/core/l10n/locales/{pt,en}.ts` | chaves |
| `poc/laya-classifier/**` | novo (POC) |

---

## 14. Riscos técnicos e decisões abertas

1. **Leitura de transcrições por tick:** `list_recent_by_session` é a solução
   limpa; se o SQLite não tiver índice adequado, adicionar índice em
   `transcriptions(session_id, end_ms)` (migração).
2. **Recriar o adaptador ao trocar backend:** o scheduler lê settings a cada
   tick; recriar o `Arc<dyn SuggestionCueClassifier>` no boot é simples, mas
   exige reiniciar o app. Alternativa: `ClassifierFactory` + revisão de
   settings. Task 06 decide; POC aceita reiniciar.
3. **Sampling da sonda aberta:** o scheduler precisa saber se ME está falando.
   O `AppState` já tem o estado de transcrição/atividade; passar um booleano
   para o gate ou usar `latest_final_end_ms` + `now`. Definir na task 10.
4. **Bloqueio do LLM de geração:** a geração é síncrona no scheduler; o
   classificador também. Em modo classificador, duas chamadas sequenciais
   (classificador + gerador) podem atrasar o tick. O classificador é rápido;
   se o gerador demorar, o scheduler fica ocupado e não classifica novos
   blocos. Aceitável no POC; evolução: thread separada para geração.
5. **Cooldown e decisão pendente:** guardar em memória por sessão; limpar ao
   trocar de sessão.
6. **Privacidade do Laya:** inferência 100% local depois de instalado; a única
   rede é o download explícito do artefato. Nenhum texto de conversa é enviado.
7. **Persistência da última classificação:** memória no POC; se o app reiniciar,
   a faixa começa vazia. Documentar como não-objetivo.
8. **MSRV/ORT:** `laya-onnx` exige Rust ≥ 1.88 e o `ort` pode baixar binário no
   build. Resolver na task 06 (bump de MSRV + vendorização do ORT).
9. **Hospedagem do ONNX:** o artefato de 1,2 GB precisa de release/CDN com
   `manifest.json` + SHA-256; o export local é só dev.
10. **Download/instalação:** `.part`, checksum, instalação atômica e rollback;
    uma falha não pode deixar `current` apontando para modelo incompleto.
11. **Fine-tuning:** o contrato de artefato/manifesto precisa ser estável antes
    de treinar; mudanças na pergunta `cue` exigem nova `model_version` e eval.
