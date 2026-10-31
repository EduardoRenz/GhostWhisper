# 27 — Wayland: gravação de tela via ScreenCast + GStreamer

- **Status:** CONCLUDED
- **Depende de:** 26 (Wayland/XWayland e portais), 25 (áudio sem `pactl`)
- **Habilita:** gravar reuniões no Ubuntu 26.04/GNOME 50 (Wayland-only)
- **Issues:** ISS-089 · **BR:** BR-006, BR-007, BR-065, BR-067, BR-070

## Objetivo

Restaurar a **gravação da sessão** no Wayland. O caminho X11 continua idêntico
(`ffmpeg`/`x11grab`, janela ativa, follow e troca de alvo); no Wayland a fonte é
escolhida no diálogo do **XDG ScreenCast** (uma janela ou um monitor) e o vídeo
vai para segmentos MKV via **PipeWire + GStreamer**.

## Causa

- O `DISPLAY` do XWayland não contém as janelas Wayland nativas: o `x11grab`
  gravaria um desktop vazio/incompleto.
- O `ffmpeg` do host não tem a entrada `pulse` e não há `parec`; o áudio precisa
  sair do GStreamer (`pulsesrc`) no mesmo pipeline.
- O portal `ScreenCast` é a única API do compositor para receber um nó PipeWire;
  ele exige o app id reverso-DNS já registrado na task 26.

## Entregáveis

- **`PortalScreenRecorder`** (`adapters/recording/portal_screen_recorder.rs`):
  - sessão `Screencast` com `Monitor | Window`, fonte única e cursor embutido;
  - um `OpenPipeWireRemote` **por segmento** (fd novo após pausa);
  - `gst-launch-1.0` em processo próprio, com o fd do portal herdado via
    `pre_exec`/`dup2` (sem `CLOEXEC` no filho);
  - vídeo H.264 (`pipewiresrc` → `x264enc` → `matroskamux`) e áudio AAC
    (`pulsesrc` mic + monitor → `audiomixer` → `avenc_aac`);
  - áudio-only sem portal (`pulsesrc` → `flacenc`);
  - `pause` encerra o `gst-launch` com SIGINT + `-e` e mantém a sessão do portal;
    `resume` abre outro segmento; `stop` fecha a sessão (some o indicador do
    GNOME);
  - thread própria para D-Bus/runtime; a porta `ScreenRecorder` continua síncrona;
  - ambiente do `gst-launch`/`gst-inspect` **sanitizado**: o `AppRun` do
    AppImage exporta `LD_LIBRARY_PATH`/`GST_PLUGIN_SYSTEM_PATH[_1_0]` para o
    AppDir e, sem limpar isso, o GStreamer do sistema não acha
    `pipewiresrc`/`x264enc`.
- **`gstreamer_args`**: builder puro dos argumentos (muxer/mixer declarados antes
  das ramificações), com testes sem executar processo.
- **`PortalScreenTargets`**: alvo único que representa o seletor do GNOME.
- **Capacidades**: `GStreamerMissing`; `target_mode = "portal"` e
  `follow_supported = false` no Wayland. A UI esconde o alvo interno e o
  "seguir a janela ativa" quando o modo é portal.
- **Composition root** (`lib.rs`): seleciona portal/GStreamer no Wayland e
  mantém `FfmpegScreenRecorder`/`X11ScreenTargets` no X11. A finalização continua
  no `FfmpegRecordingFinalizer` para os dois caminhos.
- **UI/l10n**: o seletor mostra "Escolher janela ou tela no GNOME" e avisa que a
  fonte não muda durante a gravação; o card de alvo explica o portal. Chaves em
  **pt** e **en**.
- **Dependências**: feature `screencast` do `ashpd` e `libc` (dup2/sinais).

## Critérios de aceite

- [x] Gravar pela UI do GNOME com **vídeo + áudio** e finalizar em MP4.
- [x] Pausar e retomar gera dois segmentos com fds novos e o arquivo final tem a
      duração dos dois trechos.
- [x] Áudio-only grava FLAC via GStreamer (sem portal).
- [x] X11 mantém o caminho antigo (testes e código sem mudança de comportamento).
- [x] `pause`/`stop` não deixam `gst-launch` órfão.
- [x] `cargo test`, clippy, fmt, `pnpm test` e `tsc`.
- [x] AppImage completo (Vulkan + Laya + modelo) reconstruído e validado.

## Verificação (2026-10-30)

- Teste real ignorado `records_and_finalizes_a_real_portal_stream`: seleção da
  tela no GNOME 50.1, 5 s + pausa + 4 s, **2 segmentos**, MP4 final com
  **9,0 s**, H.264 **1920×1080** e AAC **48 kHz estéreo** (ffprobe); amostra em
  `/tmp/opencode/portal-recording-test.mp4`.
- `records_audio_only_with_gstreamer`: 3 s de microfone, FLAC finalizado.
- Testes unitários do builder, das capacidades e do alvo do portal.
- Ambiente do **AppImage**: rodando o binário de teste com
  `LD_LIBRARY_PATH`/`GST_PLUGIN_SYSTEM_PATH_1_0` apontando para o AppDir (como o
  `AppRun` faz), o áudio-only continua passando e as capabilities ficam
  disponíveis — o `gst-launch`/`gst-inspect` do sistema roda com o ambiente
  limpo.
- Rust **911 ✓** (14 ignorados) · clippy/fmt ✓ · frontend **546 ✓** · tsc ✓.
- AppImage final **830,52 MiB** (870.869.496 bytes), sha256
  `7c5531037f152b1a3190960df93a0ff8ef368befe0f6e7c6b9ef7d0ed770b1dd`,
  em `src-tauri/target/release/bundle/appimage/GhostWhisper_0.3.1_amd64.AppImage`.
  Montou e iniciou no Ubuntu 26.04 (Vulkan + Laya + app id do portal); o
  ambiente GStreamer do `AppRun` foi simulado no teste de áudio e as
  capabilities ficaram disponíveis.
- **Validação manual do usuário (2026-10-31):** três gravações pela UI do
  AppImage, incluindo uma com **pausa/retomada** (2 segmentos); todas
  finalizaram com status `ready` (1,9–3,5 MB; 9,9–20,3 s).

## Limites conhecidos

- Não há **follow automático** da janela ativa no Wayland: o portal fixa a fonte
  escolhida no `start`.
- O seletor do GNOME aparece a cada gravação (`PersistMode::DoNot`); reaproveitar
  a fonte exigiria um restore token persistido, fora do escopo.
- Capturar a **tela inteira** inclui o overlay; só a captura de **janela** o
  deixa fora (BR-006).
- Se a janela escolhida fechar ou o compartilhamento for encerrado no GNOME, o
  `gst-launch` termina e o segmento fica válido; sem `window-watch` no portal, o
  app só fecha/contabiliza o trecho no próximo `pause`/`stop`.
- O host precisa de GStreamer com `pipewiresrc`, `x264enc`, `matroskamux` e
  `pulsesrc`; sem isso a UI mostra `gstreamer_missing`.
