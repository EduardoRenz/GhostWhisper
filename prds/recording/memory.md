# Memory — PRD Gravação da sessão

Histórico cronológico deste PRD (entradas `## YYYY-MM-DD — Título`, com linha
`IDs:`; convenções em [`../../.agents/rules.md`](../../.agents/rules.md)).

---

## 2026-10-21 — PRD criada (gravar a sessão em vídeo)

- **IDs:** BR-065, BR-066, BR-067, BR-068, BR-069, BR-070, BR-071,
  ISS-049, ISS-050, ISS-051, ISS-052, ISS-053 ·
  **Refs:** [`prd.md`](prd.md),
  [`architecture/recording.md`](architecture/recording.md),
  [`tasks/_tasks.md`](tasks/_tasks.md)

### Pedido

O usuário pediu para avaliar a complexidade de **gravar a tela em MP4 com todos
os áudios** e, em seguida, fechou o escopo: o objetivo é **consultar a reunião
depois** (ler o que estava na tela), em **Full HD no máximo**, com **arquivo
compacto** e **sem perda de performance**. Mandou dois mockups: o **botão de
câmera** no notch (último, vermelho quando grava, com pausa/retomada) e o modal
**"Sessão Finalizada"**.

### Medições que fecharam os padrões (2026-10-21)

Máquina: **i5-6400, 4 núcleos, 1920×1080, X11**, ffmpeg 7.0.2, x264.

| Config | CPU (1 núcleo) | % da máquina |
|---|---|---|
| só captura 10 fps | 2,8% | 0,7% |
| 1080p 5 fps `ultrafast` CRF30 | 8,0% | 2,0% |
| 1080p 8 fps `veryfast` CRF27 | ~19% | ~4,8% |
| 1080p 10 fps `ultrafast` CRF30 | 15,7% | 3,9% |
| 1080p 10 fps `veryfast` CRF26 | 23,8% | 6,0% |
| 1080p 30 fps `ultrafast` CRF30 | 47,6% | 11,9% |
| 720p 10 fps (escala) | 10,7% | 2,7% |

Conclusões: **FPS é o custo** (~13 ms/frame a 1080p); **reduzir resolução não
economiza** e piora a leitura; `mpdecimate` não acompanha (6× mais lento que o
tempo real); `-window_id` funciona (1888×1006 a 8 fps, ~20% de um núcleo).
Tamanho: tela estática 5–15 MB/h; reunião típica estimada em 40–200 MB/h;
áudio AAC 96k = 43 MB/h.

### Decisões fechadas com o usuário

| Tema | Decisão |
|---|---|
| Vínculo | Gravação **é** da sessão; não existe sem ela (BR-065) |
| Botão | Último item de `NotchTabs`, depois de Sugestões; vermelho quando grava |
| Pausa | Pausa a gravação sem parar a sessão; pausar a sessão pausa a gravação |
| Fim da sessão | Modal próprio com duração, mídia, itens, abrir local, baixar, descartar, nova sessão |
| Settings | **Nova** aba "Gravação" (6ª), com defaults da máquina de referência |
| Objetivo da qualidade | Ler a tela, não cinema: nativo · 8 fps · CRF 27 (BR-066) |
| Crash | Segmentos MKV + remux `-c copy` no fim, com progresso (BR-067) |
| Áudio | Mic + sistema **mixados**, cru (sem AEC/ganho/gate do ASR) (BR-069) |
| Alvo | **Janela da reunião** por padrão; o overlay fica visível e fora do vídeo (BR-070) |
| Privacidade | Local-only, indicador vermelho, nada em log (BR-071) |

### Por que "janela" e não "tela inteira"

O Linux **não** tem API de exclusão de janela da captura (BR-006; diferente de
`WDA_EXCLUDEFROMCAPTURE`/`NSWindowSharingNone`). Capturar a **janela** é o único
caminho em X11 que mantém o overlay visível **e** fora do arquivo. O usuário
pediu exatamente isso: usar o app normalmente, sem ele aparecer na gravação.
Trade-off registrado em ISS-052 (popups e troca de janela ficam de fora) e
ISS-053 (resize).

### Alternativas descartadas

- **Esconder o overlay durante a gravação:** o usuário pediu o contrário.
- **Compositor próprio (XComposite manual):** reimplementar o compositor para
  excluir uma janela é frágil e caro.
- **Output virtual dedicado:** só em wlroots; GNOME/KDE não expõem
  (`wayland-spike.md` §2).
- **KDE Plasma 6.6+ "Hide from Screencast":** único caminho real de exclusão por
  compositor, mas é Wayland + portal, fora da v1 (BR-007).
- **`mpdecimate` para dedup:** não acompanha em tempo real a 1080p.

### Pendências

- **ISS-049** bloqueia o empacotamento: sidecar GPL vs dependência de sistema vs
  build LGPL com VAAPI (Q-01).
- **ISS-050**: segundo cliente no microfone em ALSA puro.
- **ISS-051**: Wayland nativo sem gravação.
- **ISS-053**: resize gera dimensões diferentes entre segmentos (reencode no
  fim).

---

## 2026-10-21 — Implementação da v1 (backend + UI)

- **IDs:** BR-065…BR-071, ISS-054 ·
  **Refs:** [`prd.md`](prd.md),
  [`architecture/recording.md`](architecture/recording.md),
  [`tasks/_tasks.md`](tasks/_tasks.md)

### Pedido

O usuário pediu para **começar a trabalhar na feature e fazê-la** depois de
aprovar a PRD (botão de câmera, modal de fim de sessão e aba de configurações).

### O que foi implementado

| Camada | Arquivos |
|---|---|
| `domain` | `domain/recording.rs` (Recording, RecordingSpec, RecordingTarget, estados, normalização) |
| `application` | `application/recording_service.rs` (ciclo de vida, spec congelada, finalização em thread) |
| `ports` | `screen_recorder.rs`, `recordings_repository.rs`, `recording.rs`, `screen_targets.rs`, `pending_session_end.rs` |
| `adapters` | `recording/{ffmpeg_path,ffmpeg_args,ffmpeg_screen_recorder,finalizer,x11_targets}.rs`; `persistence/sqlite/local_recordings_repository.rs`; `persistence/memory/in_memory_recordings_repository.rs`; `pending/in_memory_pending_session_end.rs` |
| schema | tabela `recordings` + índice; `schema_version` → 3 |
| `ipc` | `recording_commands.rs`, `session_end_commands.rs`; `session_stop` devolve `SessionEndDto`; `session_delete` descarta a gravação |
| frontend | `core/domain/recording.ts`, `useRecording`, `useSessionEnd`, `RecordingButton`, `SessionEndWindow`, `RecordingSettings`, aba "Gravação", ícones, CSS |
| config | janela `session-end` (560×400) + capability; `allow-start-dragging`; `allow-reveal-item-in-dir` |

### Decisões de implementação

- **`parec` como fallback de áudio (ISS-054).** O `ffmpeg` estático do usuário
  não tem entrada `pulse`; o áudio é capturado em PCM cru e remuxado no fim.
  Com um `ffmpeg` de distro (com Pulse), o caminho de um processo só é usado.
- **Alvo = janela ativa; se o overlay tiver o foco, a janela do topo.** Assim o
  clique no botão não faz a gravação cair para a tela inteira (BR-070).
- **Finalização assíncrona** com `-c copy` e progresso por `-progress pipe:1`;
  o modal mostra a barra. Falha preserva os segmentos.
- **Descarte antes do stop** na exclusão de sessão, para não finalizar um arquivo
  que será apagado; o finalizador ignora resultado de gravação descartada.

### Verificação

- `cargo test --lib`: **586** ✓ (novos testes de domínio, args, repositório e
  serviço com fakes).
- Teste manual `#[ignore]` `records_and_finalizes_a_real_screen`: grava 2 s da
  tela, captura mic + sistema via `parec` e gera MP4 ✓.
- Validação em shell do fallback: 3 s → H.264 1888×1006 @ 8 fps + AAC estéreo
  48 kHz, duração 3,0 s.
- `cargo clippy --lib --all-targets`: limpo.
- `pnpm exec tsc --noEmit`: limpo · `pnpm test`: **382** ✓ · `pnpm build` ✓.

### Pendências (tasks 03, 07 e 10)

- Seletor de janelas (clique secundário) — hoje usa a ativa/topo.
- Observar `ConfigureNotify` para redimensionamento de janela (ISS-053).
- Atalho **Ctrl+Shift+R** no catálogo de atalhos.
- Selo de gravação no Histórico (o export já inclui a linha `Gravação:`).
- Empacotamento do `ffmpeg`/`parec` condicionado à licença (ISS-049).
- Checagem de espaço em disco antes de iniciar (CA-02.9) e validação de
  permissão de escrita da pasta (CA-04.6).
- Recuperação guiada de gravação interrompida (CA-02.5): hoje o boot só marca
  `interrupted`; não há botão "recuperar". 

---

## 2026-10-21 — Ajustes do teste do usuário (v1.1)

- **IDs:** BR-065, BR-070 · **Refs:** [`prd.md`](prd.md),
  [`architecture/recording.md`](architecture/recording.md) §9

### Feedback recebido

1. Nas configurações → Gravação, a opção ativa não parecia selecionada.
2. Continuar uma sessão antiga pelo Histórico não iniciava a gravação.
3. O ícone da gravação repetia a câmera de **foto** do print de tela.

### Correções

- **Segmentado selecionado:** os 8 grupos da aba Gravação passaram a usar
  `settings__segment--active` (mesmo padrão do HUD): cor primária, borda e
  fundo ciano. Teste de regressão em `RecordingSettings.test.tsx`.
- **Continuar sessão inicia gravação nova:** `SessionService::continue_session`
  chama `on_session_started(id, canais, true)`; o botão de câmera passa a ter
  sessão e, com `recording_auto_start`, a gravação começa sozinha. Para não
  sobrescrever o vídeo anterior, o schema foi para a **v4**: `recordings` perde
  o `UNIQUE(session_id)`, ganha `idx_recordings_session` e cada execução usa
  pasta/arquivo próprios (`<sessão>/<início_ms>/` e `<início_ms>.<ext>`). O
  modal mostra a execução mais recente; "Descartar gravação" apaga todas as
  execuções da sessão.
- **Migração:** bancos da v3 são reconstruídos uma vez em `connection.rs`
  (teste `an_old_recordings_table_loses_the_unique_session_index`).
- **Ícone:** novo `VideoCameraIcon` (corpo + lente lateral) no botão e na aba
  Gravação; `CameraIcon` (foto) continua nos fluxos de print de tela.

### Verificação

- `cargo test --lib`: **589** ✓ · `cargo clippy --lib --all-targets`: limpo.
- `pnpm exec tsc --noEmit`: limpo · `pnpm test`: **382** ✓ · `pnpm build` ✓.

---

## 2026-10-21 — Seletor de janelas, histórico e atalho (v1.2)

- **IDs:** BR-065, BR-068, BR-070 · **Refs:** [`prd.md`](prd.md),
  [`tasks/03-task.md`](tasks/03-task.md), [`tasks/10-task.md`](tasks/10-task.md)

### O que entrou

- **Seletor de janelas (task 03):** nova janela `recording-picker`, aberta pelo
  **clique secundário** no botão de câmera. Lista a tela inteira e as janelas
  (`_NET_CLIENT_LIST_STACKING`, do topo para a base); a escolha inicia a
  gravação com alvo explícito (`recording_start(target)`). O seletor fica
  bloqueado com gravação em andamento (alvo é congelado).
- **Atalho (task 07):** `Ctrl+Shift+R` (`recording_toggle`) inicia/pausa/retoma,
  no grupo de atalhos "demorados" (a pausa espera o `ffmpeg` fechar o segmento).
- **Histórico (task 10):** `db_list_sessions` devolve `recording_kind`/`recording_path`;
  a lista mostra o selo **Vídeo/Áudio** e o detalhe ganhou **Abrir gravação**.
- **Nome de arquivo amigável:** `<título-da-sessão>-AAAAMMDD-HHMMSS.mp4`
  (slug do `subject`, fallback para o id).
- **Falha na finalização:** o modal mostra "Falhou (segmentos preservados)" e
  **Abrir local** passa a revelar a pasta dos segmentos.
- **Aviso de encoder:** pedir VAAPI num ambiente sem VAAPI mostra o aviso de
  fallback para x264.

### Verificação

- `cargo test --lib`: **590** ✓ · `cargo clippy --lib --all-targets`: limpo.
- `pnpm exec tsc --noEmit`: limpo · `pnpm test`: **386** ✓ · `pnpm build` ✓.

### Ainda pendente

- Ciclo da janela (resize/fechar/minimizar) — ISS-053/CA-06.5/06.6.
- Recuperação guiada de gravação interrompida (CA-02.5).
- Espaço em disco (CA-02.9) e permissão da pasta (CA-04.6).
- Empacotamento `ffmpeg`/`parec` e licença (ISS-049).

---

## 2026-10-21 — Multi-monitor na gravação e no print (v1.3)

- **IDs:** BR-055, BR-070 · **Refs:** [`prd.md`](prd.md) US-06/CA-06.9,
  [`architecture/recording.md`](architecture/recording.md) §5.3

### Pedido

O usuário perguntou como escolher **entre dois monitores** na gravação e como o
**print de tela** se comporta nesse cenário.

### Antes

Os dois pegavam a **root window inteira**: a tela virtual somando todos os
monitores (ex.: 3840×1080). O print reduzia tudo para ≤1280 px e cada monitor
ficava com ~640 px — texto ilegível.

### Agora

- **`adapters/screen/x11_monitors.rs`**: RandR 1.5 (`GetMonitors`) devolve nome,
  geometria e primário; sem RandR 1.5 (ou com 0 monitores, como nesta máquina)
  cai para a tela virtual. `monitor_at` escolhe pelo ponto; `monitor_under_pointer`
  usa `QueryPointer`.
- **Seletor de gravação:** com mais de um monitor, lista **"Todos os monitores"**
  + **cada monitor** (nome e resolução) + janelas. Um monitor vira
  `RecordingTarget::Screen { x, y, .. }` e o `ffmpeg` recebe `-i :0.0+X,Y`.
- **Print de tela:** captura o **monitor sob o cursor**, com fallback para o
  primário/tela virtual (não reduz mais os dois monitores juntos).
- **Features:** `x11rb` ganhou `randr` no `Cargo.toml`.

### Verificação

- `cargo test --lib`: **594** ✓ (testes de `monitor_at`, `contains` e do offset
  `-i :0.0+1920,0`).
- `cargo clippy --lib --all-targets`: limpo.
- `pnpm exec tsc --noEmit`: limpo · `pnpm test`: **386** ✓ · `pnpm build` ✓.
- Nesta máquina o X server reporta 0 monitores RandR; o fallback (tela virtual
  1920×1080) foi exercitado. Num setup real de dois monitores, o `GetMonitors`
  lista os dois.

---

## 2026-10-21 — Guard de Wayland e portabilidade do pacote

- **IDs:** BR-007, BR-070, ISS-051 · **Refs:** [`prd.md`](prd.md) CA-06.8,
  [`architecture/recording.md`](architecture/recording.md) §11

### Guard de Wayland

`FfmpegScreenRecorder` passou a detectar `WAYLAND_DISPLAY` e
`XDG_SESSION_TYPE=wayland` **antes** de olhar o `DISPLAY`. Motivo: sob Wayland o
XWayland expõe um `DISPLAY`, mas a root do XWayland **não** contém as janelas
Wayland nativas — o `x11grab` gravaria um desktop vazio/incompleto. Agora a
gravação fica desabilitada com mensagem explícita, em vez de gerar um vídeo
quebrado. Teste puro `detects_wayland_sessions`.

### Portabilidade (resumo da avaliação)

- **GPU:** o binário é **Vulkan** (build `--features vulkan`). NVIDIA/AMD/Intel
  funcionam pelo ICD do driver; **CUDA/ROCm não** são usados (exigiriam builds
  próprios). Sem ICD, o probe ignora rasterizers de software e cai para CPU.
- **Wayland:** captura não confiável; agora recusada com explicação.
- **Áudio:** exige Pulse/PipeWire; gravação exige `ffmpeg` (x11grab) e, sem
  Pulse no ffmpeg, `parec` — nenhum dos dois está empacotado (ISS-049).
- **glibc:** AppImage gerado no Ubuntu 24.04 (glibc 2.39); distros mais antigas
  podem não abrir. Só `amd64`.
- **Decisões pendentes:** empacotar `ffmpeg`/`parec`, base de build mais antiga
  e eventual variante CUDA.
