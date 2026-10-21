# Arquitetura — Gravação da sessão (tela + áudio)

- **PRD:** [`../prd.md`](../prd.md)
- **BRs:** BR-065 … BR-070
- **Status:** Proposto
- **Data:** 2026-10-21

---

## 1. Princípio

A gravação é um **adaptador atrás de uma porta**, não um caso especial do
pipeline de áudio. Ela não compartilha o PCM do ASR (que é 16 kHz, com ganho,
gate e AEC), não compartilha threads com o Whisper e **não pode** derrubar a
sessão se falhar (RNF-07).

```
                 ┌──────────────────────────────────────────┐
                 │            SessionService                │
                 │  start / pause / resume / stop           │
                 └───────────────┬──────────────────────────┘
                                 │ (application)
                 ┌───────────────▼──────────────────────────┐
                 │           RecordingService               │
                 │  estado, settings congeladas, eventos    │
                 └───────┬───────────────────────┬──────────┘
                         │                       │
              ┌──────────▼─────────┐   ┌─────────▼──────────┐
              │  ScreenRecorder    │   │  RecordingRepository│
              │  (port)            │   │  (port)             │
              └──────────┬─────────┘   └─────────┬──────────┘
                         │                       │
        ┌────────────────▼──────────┐   ┌────────▼─────────────────┐
        │ FfmpegScreenRecorder      │   │ LocalRecordingsRepository│
        │ (spawn + segmentos)       │   │ InMemoryRecordings…      │
        └───────────────────────────┘   └──────────────────────────┘
```

`domain`/`application` não conhecem `ffmpeg`, `MKV` nem `x11grab`: só o
`RecordingArtifact` e o estado.

## 2. Camadas e arquivos

| Camada | Arquivo | Conteúdo |
|---|---|---|
| `domain` | `domain/recording.rs` | `Recording { id, session_id, kind, status, path, size_bytes, duration_ms, width, height, fps, started_at_ms, finished_at_ms, error }`; `RecordingKind` (`Video`/`Audio`); `RecordingStatus` (`Recording`/`Paused`/`Finalizing`/`Ready`/`Failed`/`Discarded`/`Interrupted`); `RecordingSpec` (config congelada) |
| `application` | `application/recording_service.rs` | orquestra ciclo de vida, congela `RecordingSpec`, emite eventos, converte falha em estado |
| `ports` | `ports/screen_recorder.rs` | trait `ScreenRecorder` + `RecordingProgress` |
| `ports` | `ports/recordings_repository.rs` | persistência do artefato |
| `adapters` | `adapters/recording/ffmpeg_screen_recorder.rs` | spawn, segmentos, concat, progresso |
| `adapters` | `adapters/recording/ffmpeg_args.rs` | **builders puros** (testáveis sem processo) |
| `adapters` | `adapters/recording/ffmpeg_path.rs` | resolução do binário em degraus |
| `adapters` | `adapters/recording/x11_windows.rs` | `_NET_ACTIVE_WINDOW`, lista de janelas (`WM_CLASS`, título, geometria) |
| `adapters` | `adapters/recording/window_watch.rs` | `ConfigureNotify`/`DestroyNotify`/`UnmapNotify` da janela alvo |
| `adapters` | `adapters/persistence/sqlite/local_recordings_repository.rs` | tabela `recordings` |
| `adapters` | `adapters/persistence/memory/in_memory_recordings_repository.rs` | dobra de teste |
| `ipc` | `ipc/recording_commands.rs` | `recording_start/pause/resume/stop`, `recording_state`, `recording_open_folder`, `recording_discard`, `session_end_context` |
| `ipc` | `ipc/session_end_commands.rs` | payload do modal (§7) |
| frontend | `src/core/domain/recording.ts` | tipos + normalização de settings |
| frontend | `src/core/application/useRecording.ts` | estado via evento + ações |
| frontend | `src/adapters/ipc/recording.ts` | comandos |
| frontend | `src/features/overlay/RecordingButton.tsx` | botão (§6) |
| frontend | `src/features/session-end/SessionEndWindow.tsx` | modal (§7) |
| frontend | `src/features/settings/RecordingSettings.tsx` | aba (§8) |

Composition root: `lib.rs` instancia **um** `FfmpegScreenRecorder` e o injeta no
`RecordingService`; `AppState.recording: Arc<dyn RecordingController>` (R2/R3).

## 3. Porta

```rust
/// Capacidades do ambiente (gate do botão).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordingCapabilities {
    pub available: bool,
    pub encoder: String,       // "vaapi" | "x264"
    pub reason: Option<String>,// preenchido quando `available == false`
}

/// Início de uma gravação (config já congelada pela aplicação).
pub struct RecordingStart {
    pub session_id: String,
    pub spec: RecordingSpec,
    pub dir: PathBuf,
}

pub trait ScreenRecorder: Send + Sync {
    fn capabilities(&self) -> RecordingCapabilities;
    fn start(&self, request: RecordingStart) -> Result<()>;
    fn pause(&self) -> Result<()>;
    fn resume(&self) -> Result<()>;
    /// Fecha o segmento e inicia a finalização **em background**;
    /// o resultado chega por `RecordingProgress`.
    fn stop(&self) -> Result<()>;
    fn state(&self) -> RecordingState;
}
```

`RecordingProgress` (evento):
```rust
pub enum RecordingProgress {
    State(RecordingState),      // recording | paused | finalizing | ready | failed
    Percent { percent: u8, phase: String },
    Ready(RecordingArtifact),
    Failed { message: String },
}
```

A porta é **pequena e específica** (BR-012); quem só precisa saber se está
gravando usa `state()`.

## 4. Máquina de estados

```
idle ──start──▶ recording ──pause──▶ paused
                    │                   │
                    │◀────resume────────┘
                    │
                 stop (usuário) / pause de sessão / stop de sessão
                    ▼
               finalizing ──ok──▶ ready
                    │
                    └──erro──▶ failed ──▶ (segmentos preservados)
```

- `interrupted` é atribuído no boot para linhas `recording`/`paused` órfãs
  (mesmo padrão de sessão órfã do BR-019).
- `discarded` é terminal e não apaga a sessão.
- O `RecordingService` escuta o `SessionController`: `pause()` da sessão →
  `pause()` da gravação; `resume()` → `resume()` **se** a gravação estava
  ativa; `stop()` → `stop()`.

## 5. Captura com `ffmpeg`

### 5.1 Resolução do binário (`ffmpeg_path.rs`)

1. `GHOSTWHISPER_FFMPEG` (caminho explícito, para dev/teste);
2. sidecar empacotado (`resources/ffmpeg/ffmpeg`, ao lado do executável);
3. `ffmpeg` no `PATH`.

Se nenhum existir, `capabilities().available = false` com
`reason: "ffmpeg não encontrado"` e o botão fica desabilitado (CA-01.3).

### 5.2 Segmento (vídeo)

Cada `start`/`resume` abre **um** processo e um arquivo `.mkv`:

```
ffmpeg -hide_banner -nostdin -y -loglevel error -progress pipe:1 -nostats \
  -f x11grab -framerate 8 -window_id 0x2800004 -i :0.0 \
  -f pulse -i <mic> \
  -f pulse -i <monitor> \
  -filter_complex "[1:a]aresample=48000,aformat=channel_layouts=mono[m];\
                   [2:a]aresample=48000,aformat=channel_layouts=stereo[s];\
                   [m][s]amix=inputs=2:normalize=0:duration=longest,\
                   alimiter=limit=0.95[a]" \
  -map 0:v -map "[a]" \
  -c:v libx264 -preset veryfast -crf 27 -g 80 -threads 2 -pix_fmt yuv420p \
  -c:a aac -b:a 96k -ar 48000 \
  -f matroska parts/part-0001.mkv
```

- **`-window_id`** no lugar da geometria fixa quando o alvo é uma janela
  (padrão — §5.3); com `RecordingTarget::Screen`, trocar por
  `-video_size 1920x1080 -i :0.0`.

- **MKV** porque é válido mesmo truncado (cluster a cluster): um crash deixa o
  arquivo reproduzível até o último ponto.
- **`-threads 2`** fixo: o teto de CPU (RNF-01) vale mais que o encode mais
  rápido; `-preset veryfast` já entrega texto nítido (medido, §8).
- **`-g 80`** = keyframe a cada 10 s a 8 fps, para *scrubbing*.
- **VAAPI** (quando `capabilities().encoder == "vaapi"`): trocar as linhas de
  vídeo por `-vaapi_device /dev/dri/renderD128`, `-vf
  format=nv12,hwupload`, `-c:v h264_vaapi -qp 26` e **remover `-threads`**.
- **Somente áudio:** o mesmo comando sem `-f x11grab`/`-map 0:v`, saída
  `-c:a flac -f flac parts/part-0001.flac`.
- **Sem microfone** (sessão só com áudio do sistema): o grafo cai para um único
  input, sem `amix`.

### 5.3 Alvo da captura (janela vs tela)

**Por que janela e não root:** o Linux não tem API de exclusão de janela da
captura (BR-006; diferente de `WDA_EXCLUDEFROMCAPTURE`/`NSWindowSharingNone`).
Capturar a janela é o único caminho em X11 que mantém o overlay do GhostWhisper
**visível na tela** e **fora do arquivo** (BR-070).

```
RecordingTarget::Window(id)  →  -f x11grab -window_id <id> -i :0.0
RecordingTarget::Screen      →  -f x11grab -video_size <WxH> -i :0.0
```

- **Escolha no início:** `_NET_ACTIVE_WINDOW` via `x11rb`; se a ativa for o
  próprio GhostWhisper (ou não houver), cai para `Screen` com aviso no
  *tooltip* (CA-06.3).
- **Seletor:** `x11_windows::list_windows` — janelas top-level com `WM_CLASS`,
  título e geometria; ignora `override-redirect` e janelas 1×1. A UI mostra
  app + título + **"Tela inteira"** (que avisa incluir o overlay).
- **Congelado por sessão** (CA-06.4): trocar de alvo no meio quebraria o
  `-c copy`; é outra sessão.
- **Ciclo de vida da janela:** `window_watch` assina `ConfigureNotify`
  (redimensionou), `DestroyNotify`/`UnmapNotify` (fechou/minimizou).
  - resize → fecha o segmento e abre outro na nova geometria (ISS-053);
  - fechou/minimizou → `stop` com `status = interrupted`, segmentos
    preservados (CA-06.5).
- **Dimensões divergentes entre segmentos:** a finalização detecta e reencoda
  com `scale`+`pad` para o tamanho do primeiro, sob a fase
  `Otimizando gravação…` (CA-06.6).
- **Monitores (RandR 1.5):** `x11_monitors` usa `GetMonitors` e devolve nome,
  geometria e primário; sem RandR 1.5 cai para a tela virtual. O seletor mostra
  **"Todos os monitores"** (tela virtual) e, com mais de um, **cada monitor**.
  Um monitor vira `RecordingTarget::Screen { x, y, .. }`, e o `ffmpeg` recebe
  `-i :0.0+X,Y` com `-video_size` do monitor. O **print de tela** usa o monitor
  **sob o cursor** (`monitor_under_pointer`), com fallback para o primário.
- **Limitação assumida:** popups/diálogos que são janelas separadas ficam de
  fora, e trocar de janela não acompanha (ISS-052). "Tela inteira" é a válvula
  de escape.

Medido nesta máquina (2026-10-21): janela de 1888×1006 a 8 fps = **~20% de um
núcleo**, mesmo custo por frame da root — o ganho é de conteúdo, não de CPU.

### 5.4 Sincronia

- Todos os inputs com `-use_wallclock_as_timestamps 1` e `-fflags +genpts`.
- Vídeo em CFR (`-vsync cfr` implícito no `x11grab` + `-r`).
- Áudio reamostrado para 48 kHz e alinhado pelo `amix` com
  `duration=longest`.
- O desvio residual entre mic e monitor é **fixo** (latência de abertura de
  dispositivo), não cresce: tolerância de 150 ms/h (RNF-04).
- Start de cada segmento registra `Instant::now()` para a duração usada na
  barra de progresso (não depende de `ffprobe`).

### 5.5 Finalização (`concat` + remux)

```
# parts.txt
file 'part-0001.mkv'
file 'part-0002.mkv'

ffmpeg -hide_banner -nostdin -y -loglevel error -progress pipe:1 -nostats \
  -f concat -safe 0 -i parts.txt -c copy \
  -movflags +faststart \
  <pasta>/<sessão>.mp4
```

- `-c copy`: **sem re-encode**. Custo é I/O; ~2–5 s por hora de vídeo.
- Progresso: `-progress pipe:1` emite `out_time_us=`; o percentual é
  `out_time_us / soma_das_durações`. A barra só aparece se passar de **800 ms**
  (CA-03.6).
- Ao terminar: grava `path`, `size_bytes`, `duration_ms` e `status = ready`;
  apaga `parts/` se `keep_segments == false`.
- Falha: `status = failed`, **mantém** `parts/`, e o modal aponta
  "Abrir local" para a pasta dos segmentos (CA-03.7).

### 5.6 Parâmetros congelados

`RecordingSpec` é copiado de `Settings` no `start` e nunca relido. É o que
garante que todos os segmentos tenham o mesmo codec, resolução, fps e taxa de
áudio — condição para o `-c copy` funcionar (CA-02.7).

## 6. Botão de câmera

- Novo item **dentro** de `NotchTabs`, depois de `Sugestões`, com divisor
  (`overlay__tabs-divider`) — o mockup mostra a câmera como último segmento da
  pílula.
- Props: `recording: RecordingState`, `busy`, `onToggle`.
- Cores por token (`--oh-danger`), sem cor hardcoded; o pulso usa
  `@keyframes` de opacidade (respeita `prefers-reduced-motion`).
- `aria-pressed` reflete `recording`; `aria-label` muda com o estado.
- `data-interactive` no botão (o overlay é click-through fora dos controles).

## 7. Modal "Sessão Finalizada"

### 7.1 Janela

Nova entrada em `tauri.conf.json`: `session-end` (560×400, `decorations:false`,
`transparent`, `alwaysOnTop`, `skipTaskbar`, `center`, `visible:false`).
Adicionar a `capabilities/default.json`.

O padrão é o da janela `capture`: o overlay chama `session_end_open(payload)`;
o backend guarda o payload em `AppState.pending_session_end` e mostra a janela;
ao montar, a UI chama `session_end_context()` e depois `session_end_clear()`
(mesmo ciclo de `capture_pending`/`capture_clear`).

### 7.2 Payload

```rust
pub struct SessionEndDto {
    pub session_id: String,
    pub subject: Option<String>,
    pub started_at_ms: i64,
    pub ended_at_ms: i64,
    pub duration_ms: i64,
    pub transcriptions: u32,
    pub suggestions: u32,
    pub recording: Option<RecordingSummaryDto>,
}

pub struct RecordingSummaryDto {
    pub kind: String,        // "video" | "audio"
    pub status: String,
    pub path: Option<String>,
    pub size_bytes: u64,
    pub duration_ms: i64,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub error: Option<String>,
}
```

### 7.3 Ações

| Ação | Implementação |
|---|---|
| **Abrir local** | `tauri-plugin-opener` → `reveal_item_in_dir(path)`; sem arquivo mas com segmentos → abre a pasta |
| **Baixar Sessão** | reusa `pickMarkdownPath` + `saveSessionMarkdown` (`ipc/export_commands.rs`) |
| **Descartar gravação** | `recording_discard` (apaga arquivo e `parts/`, `status = discarded`); confirmação em dois toques |
| **Nova Sessão** | `session_start` com os canais da sessão encerrada, fecha o modal |
| **X / Esc** | apenas fecha |

Só aparece no `stop` explícito do usuário (CA-03.8): o `session_stop` do IPC
devolve `SessionEndDto`; `session_delete` e a limpeza de órfãs **não** abrem.

## 8. Orçamento de desempenho (medido)

Máquina de referência: **i5-6400, 4 núcleos, 1920×1080, X11**, `ffmpeg 7.0.2`,
x264. 10 s de captura real por linha; CPU em % de **um** núcleo.

| Config | CPU (1 núcleo) | % da máquina |
|---|---|---|
| só captura, 10 fps | 2,8% | 0,7% |
| 1080p 5 fps `ultrafast` CRF 30 | 8,0% | 2,0% |
| **1080p 8 fps `veryfast` CRF 27 (padrão)** | **~19%** | **~4,8%** |
| 1080p 10 fps `ultrafast` CRF 30 | 15,7% | 3,9% |
| 1080p 10 fps `veryfast` CRF 26 | 23,8% | 6,0% |
| 1080p 30 fps `ultrafast` CRF 30 | 47,6% | 11,9% |
| janela 1888×1006 8 fps `veryfast` | ~20% | ~5,0% |
| 720p 10 fps (escala) | 10,7% | 2,7% |

Conclusões que viraram requisito:

1. **FPS é a variável de custo** (13 ms por frame a 1080p). 30 fps dobra o custo
   sem ganho para "ler a tela" → padrão 8 fps (RNF-01).
2. **Reduzir resolução não economiza** (o custo é por frame) e **destrói texto**
   → padrão "nativa" (CA-04.3).
3. `ultrafast` desliga CABAC/deblocking e serrilha texto; `veryfast` CRF 27 é o
   ponto de legibilidade por custo.
4. `mpdecimate` (dedup de frames) **não** é viável: medido 6× mais lento que o
   tempo real a 1080p. Dedup futuro só por *damage events* do X11.

Tamanho (medido + projeção): tela estática **5–15 MB/h**; slides com cursor
**~5 MB/h**; reunião típica **40–200 MB/h**; áudio AAC 96k soma **43 MB/h**.
Teto de RNF-03: 250 MB/h.

## 9. Schema

`recordings` (**várias execuções por sessão** — continuar uma sessão antiga
inicia uma gravação nova, sem concatenar com a anterior):

```sql
CREATE TABLE IF NOT EXISTS recordings (
  id             TEXT PRIMARY KEY,
  session_id     TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
  kind           TEXT NOT NULL,              -- video | audio
  status         TEXT NOT NULL,              -- recording|paused|finalizing|ready|failed|discarded|interrupted
  path           TEXT,
  segments_dir   TEXT,
  size_bytes     INTEGER NOT NULL DEFAULT 0,
  duration_ms    INTEGER NOT NULL DEFAULT 0,
  width          INTEGER NOT NULL DEFAULT 0,
  height         INTEGER NOT NULL DEFAULT 0,
  fps            INTEGER NOT NULL DEFAULT 0,
  started_at_ms  INTEGER NOT NULL,
  finished_at_ms INTEGER,
  error          TEXT
);

CREATE INDEX IF NOT EXISTS idx_recordings_status ON recordings(status);
CREATE INDEX IF NOT EXISTS idx_recordings_session
  ON recordings(session_id, started_at_ms);
```

- `schema_version` → **4** (`INSERT OR IGNORE`, idempotente como o resto).
- `get_by_session` devolve a execução **mais recente** (o modal mostra a última);
  `list_by_session` devolve todas (descarte/exclusão).
- Cada execução tem pasta e arquivo próprios:
  `<pasta>/<sessão>/<início_ms>/` e `<pasta>/<sessão>/<início_ms>.<ext>` — o
  arquivo anterior **não** é sobrescrito.
- Migração da v3: a tabela é reconstruída uma vez (`connection.rs`) para remover
  o `UNIQUE(session_id)`.
- `path` é `NULL` enquanto não finaliza; `segments_dir` permite recuperar.
- `ON DELETE CASCADE`: excluir a sessão apaga as linhas (os arquivos são apagados
  pelo serviço, não pelo banco — CA-05.4).

## 10. Configurações

Campos novos em `domain/settings.rs` (todos com `#[serde(default)]`) e no
espelho `src/core/domain/settings.ts`:

```rust
pub recording_mode: String,           // "video" | "audio"          (default video)
pub recording_target: String,         // "window" | "screen"        (default window)
pub recording_auto_start: bool,       // default false
pub recording_fps: u32,               // default 8   (5..=30)
pub recording_resolution: String,     // "native" | "1280x720" | "854x480"
pub recording_quality: String,        // "compact" | "balanced" | "sharp"
pub recording_encoder: String,        // "auto" | "x264" | "vaapi"
pub recording_audio_kbps: u32,        // default 96  (64..=192)
pub recording_container: String,      // "mp4" | "mkv" (formato final)
pub recording_dir: Option<String>,    // default XDG videos
pub recording_keep_segments: bool,    // default false
```

Normalização no backend (função pura, como `clamp_mic_gain`):
`normalize_recording_settings(&mut Settings)` — FPS para o degrau mais próximo,
kbps clampado, strings fora do allowlist caem no padrão, caminho vazio →
`$XDG_VIDEOS_DIR/GhostWhisper` → `~/Vídeos/GhostWhisper`.

## 11. Erros e degradação

| Situação | Comportamento |
|---|---|
| `ffmpeg` ausente | botão desabilitado, `reason` no *tooltip*, sessão normal |
| sessão Wayland (`WAYLAND_DISPLAY`/`XDG_SESSION_TYPE=wayland`) | botão desabilitado: a root do XWayland não contém janelas Wayland nativas |
| sem `DISPLAY` | idem, `reason: "gravação disponível apenas em X11/XWayland (BR-007)"` |
| janela alvo fechada ou minimizada | `stop` com `status=interrupted`, segmentos preservados, modal mostra "Abrir local" |
| janela alvo redimensionada | fecha o segmento e abre outro na nova geometria (ISS-053) |
| encoder falha no meio | `status=failed`, sessão continua, toast discreto |
| disco < 250 MB | fecha o segmento, `failed` com motivo, mantém o que existe |
| `paused` no stop | ainda assim finaliza o que já foi gravado |
| duas sessões | impossível por construção (sessão única) |

Nenhum caminho grava áudio/frame em log (RNF-08).

## 12. Testes

- **Puros (sem processo):** `ffmpeg_args` — vídeo, áudio, sem mic, VAAPI;
  normalização de settings; `ffmpeg_path` (3 degraus, com `PATH` fake);
  `x11_windows` (parsers de `_NET_ACTIVE_WINDOW`/`WM_CLASS` com buffers fixos);
  `window_watch` (transições resize/close/minimize).
- **Application:** `RecordingService` com `FakeScreenRecorder` +
  `InMemoryRecordingsRepository` — start/pause/resume/stop, falha não derruba a
  sessão, settings congeladas, `interrupted` no boot.
- **Frontend:** `RecordingButton` (4 estados, clique, `aria-pressed`),
  `SessionEndWindow` (com/sem gravação, progresso, descartar em dois toques),
  `RecordingSettings` (defaults, faixas, estimativa).
- **Manual (ignorado no CI, como `captures_the_real_screen`):** gravação de 10 s
  real e verificação de que o MP4 abre com `ffprobe`.
- Nenhum teste unitário executa `ffmpeg` (RNF-10).
