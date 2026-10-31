<p align="center">
  <img src="assets/icons/icon-source.png" width="144" alt="GhostWhisper logo" />
</p>

<h1 align="center">GhostWhisper</h1>

<p align="center">
  <strong>Open-source, Linux-first, local-only meeting copilot.</strong><br />
  Listens to your meeting, transcribes, translates and suggests replies — in a discreet overlay.
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-2ea44f?style=flat-square" alt="License: MIT" /></a>
  <img src="https://img.shields.io/badge/Platform-Linux-0078D6?style=flat-square&logo=linux&logoColor=white" alt="Platform: Linux" />
  <img src="https://img.shields.io/badge/Tauri-v2-24C8DB?style=flat-square&logo=tauri&logoColor=white" alt="Tauri v2" />
  <img src="https://img.shields.io/badge/Rust-1.77%2B-000000?style=flat-square&logo=rust&logoColor=white" alt="Rust 1.77+" />
  <img src="https://img.shields.io/badge/React-19-61DAFB?style=flat-square&logo=react&logoColor=06111F" alt="React 19" />
  <img src="https://img.shields.io/badge/Whisper-Vulkan%20%2F%20CPU-A41E22?style=flat-square&logo=vulkan&logoColor=white" alt="Whisper: Vulkan or CPU" />
  <img src="https://img.shields.io/badge/Local--only-no%20telemetry-0F766E?style=flat-square" alt="Local-only, no telemetry" />
</p>

---

GhostWhisper is an open-source meeting copilot built from scratch to work well
on Linux. No account, no telemetry and no subscription: bring your own API key
(BYO key) or run local models.

## Why it exists

Most similar apps offer poor Linux support: unstable system-audio capture, a
window/permission quirks and key features locked
behind a paywall. GhostWhisper focuses on getting the core experience right on
Linux and staying fully open.

## Key features

- **Real-time transcription** — microphone and/or system audio (PipeWire /
  PulseAudio sink monitor), with independent Whisper contexts per channel.
- **Local speech-to-text** — `whisper.cpp` through `whisper-rs`, Silero VAD,
  Vulkan GPU acceleration with automatic CPU fallback.
- **Translation and AI suggestions** — direct LLM calls (BYO key) with multiple
  prioritized sources, automatic fallback and per-source circuit breaker.
- **Agents and insights** — Default, Meeting, Technical Interview, LeetCode and
  System Design presets, plus custom agents and saved prompts.
- **Sessions and history** — local SQLite storage (FTS5 search), live session
  journal, incremental summaries and Markdown export.
- **Session recording** — record the meeting window or the full screen together
  with session audio (microphone + system) into a compact MP4; recording belongs
  to the session and can be paused, resumed and discarded. On Wayland you pick a
  window or monitor in the GNOME portal dialog; the source stays fixed for that
  recording.
- **Discreet overlay** — transparent HUD, tray icon, configurable global
  shortcuts and a stealth mode that hides the window from the taskbar.
- **Local-only** — audio, transcripts and settings never leave your machine;
  only the LLM requests you configure go out.

## Tech stack

| Layer | Technology |
|-------|------------|
| App shell | **Tauri v2** (Rust 1.77+) |
| Frontend | React 19 + Vite 6 + TypeScript 5 |
| Audio | `cpal` 0.15 (ALSA/PulseAudio/PipeWire) + `libpulse-binding` 2 |
| System audio | PipeWire / PulseAudio sink monitor |
| VAD + ASR | `whisper-rs` 0.16 (whisper.cpp) + Silero VAD |
| Acceleration | Vulkan backend (default release), CPU fallback |
| LLM | HTTP (OpenAI, Gemini, OpenRouter, DeepSeek, custom OpenAI-compatible) |
| Persistence | SQLite via `rusqlite` 0.32 (bundled, FTS5) |
| Recording | X11: FFmpeg `x11grab` (x264/VAAPI). Wayland: XDG ScreenCast + PipeWire + GStreamer (`pipewiresrc`/`x264enc`) |
| Secrets | Secret Service (libsecret) via `keyring` 3 |
| Overlay | Transparent Tauri window; X11/XWayland click-through |

## Requirements

- Linux with X11/XWayland. On Wayland the overlay uses XWayland and global shortcuts/screenshot use XDG portals (GNOME screenshot UI on capture). Screen recording uses the ScreenCast portal plus GStreamer (see below).
- Rust stable (1.77+) and `cargo`.
- Node.js 20+ and `pnpm` (or `npm`).
- Tauri v2 system dependencies:
  - Debian/Ubuntu:
    `libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev libasound2-dev libpulse-dev libvulkan-dev glslc`
  - Fedora:
    `webkit2gtk4.1-devel openssl-devel alsa-lib-devel pulseaudio-libs-devel vulkan-loader-devel glslc`
- PipeWire or PulseAudio with a sink monitor for system audio.
- **Wayland recording:** GStreamer with `pipewiresrc`, `x264enc`, `matroskamux`
  and `pulsesrc`, plus the GNOME ScreenCast portal. On Debian/Ubuntu:
  `gstreamer1.0-tools gstreamer1.0-pipewire gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-plugins-ugly gstreamer1.0-libav xdg-desktop-portal-gnome`.
  Without them the recording button explains that `gstreamer_missing`.

## Getting started

```bash
git clone --recurse-submodules https://github.com/EduardoRenz/GhostWhisper.git
cd GhostWhisper
pnpm install
pnpm tauri dev
```

> O submódulo `vendor/laya` (classificador Laya, Apache-2.0) e o artefato ONNX
> (~1,3 GB, não versionado) são **obrigatórios** para o build de release. Se você
> clonou sem `--recurse-submodules`, rode `git submodule update --init`; para
> gerar o artefato uma vez, veja [Build and packaging](#build-and-packaging).
> Todo AppImage de release é **Vulkan + Laya + modelo empacotado**.

On first launch, open **Settings** (gear icon) and add an AI source with your own
API key. Keys are stored in the system keyring, never in plain text. The Whisper
model is downloaded only on the first session that uses it.

> **VS Code snap terminal:** the snap environment exports `GTK_PATH`,
> `GIO_MODULE_DIR`, `LOCPATH` and `XDG_DATA_DIRS` pointing into `/snap`, which
> breaks the app with `undefined symbol: __libc_pthread_init`. Use
> `pnpm tauri:dev:clean` or run from a native terminal.

## Build and packaging

O **único** build de release do GhostWhisper é o AppImage completo: **Vulkan +
engine Laya + modelo ONNX empacotado**. Não existe build leve/CPU de release; o
binário escolhe a GPU em runtime e cai para CPU sozinho quando não há Vulkan.

```bash
pnpm tauri:build
# equivalente: ./scripts/tauri-build-linux.sh
```

> `pnpm tauri build` também é redirecionado para esse build completo (o CLI não
> consegue gerar um bundle parcial).

O script exige o submódulo `vendor/laya` e o artefato ONNX (não versionado):

```bash
git submodule update --init
python3 vendor/laya/laya-rust/tools/export_onnx.py --model multilingual \
  --out src-tauri/resources/laya
```

Gera **um AppImage** em
`src-tauri/target/release/bundle/appimage/GhostWhisper_0.3.1_amd64.AppImage`
(~830 MiB), com o modelo embutido — não precisa baixar nada no primeiro uso.

O build:
- confere o artefato e corrige as permissões que o snapshot do Hugging Face
  deixa somente-leitura;
- monta o SDK Vulkan (`scripts/vulkan-env.sh`, sem sudo) e embute
  `libvulkan.so.1`;
- publica em `src-tauri/target/release/bundle` e remove AppImages antigos;
- imprime o caminho e o SHA-256.

> **Laya is a POC:** the engine and the multilingual model are bundled, but the
> zero-shot evaluation is below the quality gate. `Regras locais` remains the
> default; the UI warns when Laya is selected. Fine-tuning is the next step
> (task 12).

- The Vulkan binary picks a GPU at runtime and falls back to CPU by itself when
  no Vulkan device is available — one build works everywhere.
- `libvulkan.so.1` is embedded in the AppImage, so it runs even without the
  loader installed; GPU drivers/ICDs still come from the host.
- `scripts/vulkan-env.sh` assembles the SDK from the system (`libvulkan-dev`,
  `glslc`) or extracts downloaded `.deb` files into `.vulkan-sdk/` — no sudo.
- The build downloads the **current AppImage type2 runtime** into
  `~/.cache/ghostwhisper/` and embeds it; the old runtime cannot mount on
  Ubuntu 26.04 (`fusermount3` 3.18). If you still hit FUSE, run the AppImage
  with `APPIMAGE_EXTRACT_AND_RUN=1`.
- After an unclean shutdown with a GPU context active, the next launch runs on
  CPU and returns to GPU after a clean exit. The crash marker is armed only when
  the GPU context actually loads, so opening/closing the app without a session
  does not disable acceleration. The choice is also available in
  **Settings → Transcription & Audio → Acceleration** (`auto` / `CPU` / `GPU`).

Install a release:

```bash
chmod +x GhostWhisper_0.3.1_amd64.AppImage && ./GhostWhisper_0.3.1_amd64.AppImage
```

## Available scripts

| Command | Description |
|---------|-------------|
| `pnpm dev` | Vite dev server only |
| `pnpm tauri dev` | Full app in development (CPU build) |
| `pnpm tauri:dev:vulkan` | Development with Vulkan GPU acceleration |
| `pnpm tauri:dev:laya` | Development with Vulkan + Laya engine |
| `pnpm tauri:dev:clean` | Dev launcher that strips snap environment variables |
| `pnpm tauri:build` | **Único build de release**: AppImage Vulkan + Laya + modelo ONNX |
| `pnpm tauri build` | CLI redirecionado para o build único |
| `pnpm test` | Frontend tests (Vitest) |

## Project structure

```
GhostWhisper/
├── assets/icons/             # App icon source (also used by the README)
├── prds/                     # Spec-driven documentation and decisions
├── scripts/                  # Build, packaging and Vulkan helpers
├── src/                      # React frontend
│   ├── core/                 # Domain, l10n and adapters
│   ├── components/           # Shared UI
│   └── features/             # Overlay, settings, session, history, journal…
├── src-tauri/
│   └── src/
│       ├── domain/           # Pure domain types and rules
│       ├── application/      # Use cases
│       ├── ports/            # Traits (audio, ASR, LLM, repositories)
│       ├── adapters/         # Implementations (cpal, whisper, HTTP, SQLite)
│       └── ipc/              # Tauri commands
└── README.md
```

The Rust core follows a clean/hexagonal architecture: `domain` and `application`
never depend on adapters, and every port has a test double.

## Configuration

Everything is configured in the app and persisted locally:

| Setting | Description |
|---------|-------------|
| AI sources | Multiple providers with their own key and model; list order is priority order, with automatic fallback |
| Transcription | Local `whisper.cpp` model (`tiny`, `base`, `small`, `medium`) or cloud |
| Languages | Source and target: Auto, Portuguese, English; target Auto does not translate |
| Audio | Microphone, system sink monitor, or both |
| Shortcuts | Global shortcuts for overlay, session, stealth and suggestions |

Default shortcuts:

| Action | Shortcut |
|--------|----------|
| Show/hide overlay | `Ctrl+Shift+O` |
| Start/end session | `Ctrl+Shift+S` |
| Toggle stealth | `Ctrl+Shift+H` |
| Ask for a suggestion | `Ctrl+Shift+I` |

## Testing

```bash
pnpm test                                         # frontend (Vitest)
cargo test --manifest-path src-tauri/Cargo.toml   # Rust core
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```

The test suite uses in-memory repositories and fake providers; no network,
microphone or audio device is required.

## Troubleshooting

**Wayland** — on Wayland the overlay is an **XWayland** client (forced at boot)
so it stays on top and can be positioned/clicked through. Global shortcuts use
the XDG **GlobalShortcuts** portal (approve the GNOME dialog once); the screenshot
uses the XDG **Screenshot** portal in interactive mode, so GNOME shows its capture
UI and the user confirms the area. X11 keeps the old silent `GetImage` path.
Screen recording uses the XDG **ScreenCast** portal: GNOME asks for a window or
monitor on every recording and the source stays fixed until you stop (there is
no follow-active-window on Wayland). The app registers the reverse-DNS app id
`com.ghostwhisper.app` and a hidden `.desktop` in
`$XDG_DATA_HOME/applications`; that is required by xdg-desktop-portal ≥1.20.

**Screen-share invisibility** — no X11/Wayland API can hide a window from a
screen capture. Stealth mode removes the overlay from the taskbar and alt-tab
but not from a shared screen. Share a single window instead of the whole screen,
or move the overlay to another monitor. On Wayland, choose a **window** in the
ScreenCast dialog; the entire screen includes the overlay.

**Debug logging** — set `RUST_LOG=ghostwhisper_lib=debug` (not global `debug`,
which makes `keyring` dump credential attributes). Each LLM call logs the task
scope, provider, real model, fallback flag, latency and token usage.

## Privacy

- No account, no login, no telemetry, no backend.
- Audio and transcripts stay on your disk (SQLite).
- LLM requests go directly from your machine to the provider you configured.

## Contributing

1. Read [`AGENTS.md`](AGENTS.md) and the PRDs in [`prds/`](prds/).
2. Describe the change in the relevant PRD before writing code.
3. Keep the architecture boundaries and add tests for every port.
4. Run `pnpm test` and `cargo test` before opening a pull request.

## License

MIT — see [LICENSE](LICENSE).
