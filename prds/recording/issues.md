# Issues — PRD Gravação da sessão

Registry de riscos, pendências e limitações observadas. Convenções em
[`../../.agents/rules.md`](../../.agents/rules.md) (`ISS-NNN` canônico por
arquivo de PRD; a numeração **continua** a dos PRDs anteriores). Próximo ID
livre: **ISS-054**.

| ID | Resumo | Severidade | Status |
|---|---|---|---|
| ISS-049 | `ffmpeg` não é dependência do app; licença do build estático (x264) é GPL | Alta | OPEN |
| ISS-050 | Dois clientes no microfone (cpal + ffmpeg) podem competir em ALSA puro | Média | OPEN |
| ISS-051 | Wayland nativo sem gravação (BR-007) | Média | OPEN |
| ISS-052 | Captura por janela perde popups/diálogos e não acompanha troca de janela | Média | OPEN |
| ISS-053 | Redimensionar a janela muda as dimensões dos segmentos | Média | OPEN |
| ISS-054 | `ffmpeg` estático sem entrada Pulse: áudio cai para `parec` em PCM cru | Média | MITIGATED |

---

## ISS-049 — `ffmpeg` não é dependência do app; licença do build estático é GPL

- **Severidade:** Alta · **Status:** OPEN
- **Sintoma:** hoje não há `ffmpeg` no `Cargo.toml` nem nos pacotes. O binário
  do usuário (`~/.local/bin/ffmpeg`, build estático John Van Sickle) **não** tem
  VAAPI, e é compilado com `--enable-libx264` (GPL).
- **Impacto:** sem `ffmpeg` a feature não funciona; empacotar o build estático
  adiciona ~80 MB ao AppImage e obriga o projeto a ser **GPL-compatível** — o
  licenciamento do GhostWhisper está "a definir (candidato MIT/Apache-2.0)".
- **Opções:**
  1. dependência do sistema (`ffmpeg` no `.deb`/`depends`), sem empacotar;
  2. sidecar estático GPL → projeto vira GPL-2.0+ (ou LGPL sem x264);
  3. build próprio **LGPL** com `--disable-gpl` + `h264_vaapi`/`openh264`
     (sem `libx264`), mantendo MIT/Apache;
  4. `ffmpeg-next`/`libav` linkado dinamicamente (mesmo problema de licença,
     pior build).
- **Recomendação:** (3) se a licença for MIT/Apache; (2) se o projeto adotar
  GPL. A decisão bloqueia o empacotamento (task 10), não o desenvolvimento.
- **Refs:** BR-066, BR-070 · [`../prd.md`](prd.md) Q-01/RNF-09.

## ISS-050 — Dois clientes no microfone (cpal + ffmpeg)

- **Severidade:** Média · **Status:** OPEN
- **Sintoma:** o ASR abre o microfone via `cpal`; a gravação abre outro stream
  no mesmo dispositivo via `ffmpeg -f pulse`.
- **Impacto:** em PipeWire/PulseAudio (o caminho comum) múltiplos leitores são
  suportados e não há problema. Em ALSA puro (`hw:`) o dispositivo pode ser
  exclusivo: o segundo cliente falha ou o primeiro é interrompido.
- **Mitigação:** detectar a falha e degradar para **áudio do sistema apenas**,
  com aviso no modal (`Gravação sem microfone: dispositivo em uso`), em vez de
  derrubar a sessão (RNF-07). Caminho definitivo: tap do PCM nativo do app
  (uma captura só), que exige o resample virar decorador — fora da v1.
- **Refs:** BR-069 · [`architecture/recording.md`](architecture/recording.md) §5.2.

## ISS-051 — Wayland nativo sem gravação

- **Severidade:** Média · **Status:** OPEN
- **Sintoma:** em Wayland puro não há `DISPLAY`; `x11grab` não existe. O
  `xdg-desktop-portal` + PipeWire permitiria capturar, mas exige um consumidor
  PipeWire (nova dependência) e consentimento por sessão.
- **Impacto:** usuários Wayland veem o botão desabilitado com explicação; a
  transcrição continua normal via XWayland.
- **Mitigação:** mensagem clara (`gravação disponível apenas em X11/XWayland`)
  e caminho futuro documentado (portal ScreenCast + `pipewire-rs`).
- **Refs:** BR-007, BR-070 ·
  [`../../mvp/architecture/wayland-spike.md`](../mvp/architecture/wayland-spike.md) §2–§3.

## ISS-052 — Captura por janela perde popups e não acompanha troca de janela

- **Severidade:** Média · **Status:** OPEN
- **Sintoma:** `-window_id` grava o drawable de **uma** janela. Um diálogo do
  sistema, um popup do navegador ou uma segunda janela da reunião (vídeo
  destacado) não entram. Trocar de janela durante a reunião não muda o alvo.
- **Impacto:** em reuniões com conteúdo em várias janelas, o vídeo fica
  incompleto. É o preço de manter o overlay fora do arquivo (BR-070).
- **Mitigação:** oferecer **"Tela inteira"** como alvo explícito na configuração
  e no seletor de início; documentar no *tooltip* que só a janela escolhida é
  gravada. Alvo é congelado por sessão.
- **Refs:** BR-070 · [`../prd.md`](prd.md) US-06.

## ISS-053 — Redimensionar a janela muda as dimensões dos segmentos

- **Severidade:** Média · **Status:** OPEN
- **Sintoma:** `x11grab -window_id` fixa a geometria na abertura. Maximizar,
  restaurar ou arrastar para outro monitor entre segmentos pode gerar
  resoluções diferentes, e o `-c copy` do concat falha.
- **Impacto:** a finalização precisa reencodar (mais lento) ou recusar.
- **Mitigação:** observar `ConfigureNotify`; ao mudar o tamanho, fechar o
  segmento e abrir outro com a nova geometria. Na finalização, se as dimensões
  divergirem, reencodar com `scale`+`pad` para o tamanho do primeiro segmento,
  com fase própria na barra (`Otimizando gravação…`). Se a janela fechar ou for
  minimizada, encerrar a gravação com `interrupted` (não apagar nada).
- **Refs:** BR-067, BR-070 · [`architecture/recording.md`](architecture/recording.md) §5.3.


## ISS-054 — `ffmpeg` estático sem entrada Pulse: áudio cai para `parec`

- **Severidade:** Média · **Status:** MITIGATED
- **Sintoma:** o build estático do `ffmpeg` da máquina de referência (John Van
  Sickle, 7.0.2) lista **apenas** `lavfi` e `x11grab` em `-devices`: não tem
  `pulse` nem `alsa`. Com `-f pulse` o processo falha com
  `Unknown input format: 'pulse'` — a gravação sairia **sem áudio**.
- **Mitigação:** quando `ffmpeg -devices` não tem `pulse`, o adaptador captura o
  áudio em **PCM cru S16LE 48 kHz estéreo** com `parec` (dois processos: microfone
  e monitor) e remuxa na finalização (`fallback_mux_args`). O vídeo continua no
  `ffmpeg` (`-an`). Validado em 2026-10-21: 3 s → MP4 H.264 1888×1006 + AAC
  estéreo 48 kHz, duração 3,0 s.
- **Consequência:** o app exige **`parec`** (de `pipewire-pulse`/`pulseaudio-utils`)
  quando o `ffmpeg` não tem Pulse. Sem nenhum dos dois, a gravação fica
  indisponível com motivo explícito (RNF-09). A sincronia A/V passa a depender
  dos dois inícios (ffmpeg e parec) no mesmo segmento; o desvio medido é de
  dezenas de ms.
- **Caminho definitivo:** `ffmpeg` de distro com Pulse (ou sidecar com
  `--enable-libpulse`) — decide-se junto com a licença (ISS-049).
- **Refs:** BR-069 · [`../architecture/recording.md`](../architecture/recording.md) §5.2.
