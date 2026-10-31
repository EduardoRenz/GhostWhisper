# Arquitetura — GhostWhisper MVP

Como o [PRD](../prd.md) será implementado. Descreve a estrutura de pastas, os
arquivos (novos e modificados) e o papel de cada um.

> Princípios obrigatórios (hexagonal, SOLID, DI, testabilidade, nomenclatura):
> [`.agents/rules.md`](../../../.agents/rules.md) e
> [`engineering-guidelines.md`](engineering-guidelines.md) (BR-011..BR-014).

---

## 1. Visão de alto nível

```
┌──────────────────────────── Tauri App ─────────────────────────────┐
│                                                                    │
│  Frontend (React/TS)                Core (Rust, hexagonal)         │
│  ┌───────────────────┐   events    ┌───────────────────────────┐   │
│  │ Overlay window    │◀────────────│ ipc (entrada)             │   │
│  │ Settings window   │────────────▶│ application (use cases)   │   │
│  │ Session/History   │             │   └─ ports (traits) ◀─────┼── adapters
│  └───────────────────┘             │ domain (regras puras)     │   │
│                                    └───────────────────────────┘   │
└────────────────────────────────────────────────────────────────────┘
```

- O **core Rust** concentra áudio, VAD, ASR, LLM e persistência, atrás de portas.
- A **UI React** é apresentação/entrada; consome `ports` via adaptador IPC. Não
  acessa rede/disco direto. Segredos nunca chegam ao frontend.

## 2. Camadas e fronteiras

| Camada (`src-tauri/src/`) | Responsabilidade | Não faz | Depende de |
|---------------------------|------------------|---------|------------|
| `domain/` | Entidades/regras puras | I/O, framework | — |
| `application/` | Casos de uso/orquestração | I/O direto | `domain`, `ports` |
| `ports/` | Traits (contratos) | implementação | `domain` |
| `adapters/` | Audi, ASR, LLM, DB, OS, overlay | regra de negócio | `ports`, `domain` |
| `ipc/` | Commands/events Tauri | regra de negócio | `application`, `ports` |
| `lib.rs` | Composition root (DI) | regra de negócio | todas |

Detalhes em [`engineering-guidelines.md`](engineering-guidelines.md).

## 3. Estrutura de pastas (alvo do MVP)

```
GhostWhisper/
├── README.md / AGENTS.md / .agents/rules.md
├── package.json / pnpm-lock.yaml / vite.config.ts / tsconfig.json / index.html
├── prds/                                        # especificação
├── assets/
│   ├── ggml-silero-v5.1.2.bin                   # (novo) modelo VAD (MIT)
│   └── icons/
│
├── src/                                         # FRONTEND (React + TS)
│   ├── main.tsx / App.tsx / styles/globals.css
│   ├── core/                                    # "domain/application/ports" do frontend
│   │   ├── domain/{session.ts,transcription.ts,insight.ts}
│   │   ├── ports/{transcriptions_repository.ts,chat_bot_provider.ts,...}
│   │   └── application/{useLiveTranscription.ts,useSession.ts}
│   ├── adapters/
│   │   └── ipc/{ipcTranscriptionsRepository.ts,ipcChatBotProvider.ts,client.ts}
│   └── features/
│       ├── overlay/{Overlay.tsx,ActionBar.tsx}
│       ├── transcript/TranscriptView.tsx
│       ├── translation/TranslationView.tsx
│       ├── insights/InsightsPanel.tsx
│       ├── session/SessionControls.tsx
│       ├── history/HistoryPage.tsx
│       └── settings/SettingsPage.tsx
│
├── src-tauri/                                   # CORE (Rust, hexagonal)
│   ├── Cargo.toml / tauri.conf.json / build.rs
│   ├── capabilities/default.json
│   ├── icons/
│   └── src/
│       ├── main.rs                              # entrypoint (chama lib::run)
│       ├── lib.rs                               # COMPOSITION ROOT (wiring/DI)
│       ├── error.rs                             # erros comuns (thiserror)
│       ├── state.rs                             # AppState (Arc<dyn Porta>)
│       │
│       ├── domain/
│       │   ├── mod.rs
│       │   ├── session.rs                       # Session, SessionStatus
│       │   ├── transcription.rs                 # TranscriptSegment, Speaker
│       │   └── insight.rs                        # Insight, Translation
│       │
│       ├── application/
│       │   ├── mod.rs
│       │   ├── session_service.rs               # iniciar/encerrar sessão
│       │   ├── transcription_service.rs         # VAD->ASR->persistência/eventos
│       │   ├── insight_service.rs               # sugestões
│       │   └── translation_service.rs           # tradução ao vivo
│       │
│       ├── ports/
│       │   ├── mod.rs
│       │   ├── repositories.rs                  # Session/Transcriptions/Message
│       │   ├── chat_bot_provider.rs             # ChatBotProvider
│       │   ├── speech_to_text.rs                # SpeechToText
│       │   ├── voice_activity.rs                # VoiceActivityDetector
│       │   ├── audio_source.rs                  # AudioSource
│       │   ├── secret_store.rs                  # SecretStore
│       │   ├── settings_store.rs                # SettingsStore
│       │   ├── overlay_window.rs                # OverlayWindowController
│       │   ├── shortcut_registry.rs             # GlobalShortcutRegistry
│       │   ├── event_publisher.rs               # EventPublisher
│       │   └── clock.rs                         # Clock
│       │
│       ├── adapters/
│       │   ├── mod.rs
│       │   ├── persistence/
│       │   │   ├── schema.sql
│       │   │   ├── sqlite/
│       │   │   │   ├── connection.rs
│       │   │   │   ├── local_session_repository.rs
│       │   │   │   ├── local_transcriptions_repository.rs
│       │   │   │   └── local_message_repository.rs
│       │   │   └── memory/
│       │   │       ├── in_memory_session_repository.rs
│       │   │       └── in_memory_transcriptions_repository.rs
│       │   ├── ai/
│       │   │   ├── openai_chat_bot_provider.rs
│       │   │   ├── gemini_chat_bot_provider.rs
│       │   │   ├── openrouter_chat_bot_provider.rs
│       │   │   └── custom_chat_bot_provider.rs
│       │   ├── speech/
│       │   │   ├── whisper_speech_to_text.rs
│       │   │   ├── silero_voice_activity.rs
│       │   │   └── whisper_model_store.rs
│       │   ├── audio/
│       │   │   ├── cpal_microphone_source.rs
│       │   │   ├── pulse_system_audio_source.rs
│       │   │   ├── pactl_sink_helper.rs
│       │   │   └── pcm_resampler.rs
│       │   ├── secrets/keyring_secret_store.rs
│       │   ├── settings/file_settings_store.rs
│       │   ├── system/system_clock.rs
│       │   ├── overlay/{tauri_overlay_window.rs,cursor_click_through.rs}
│       │   ├── shortcuts/tauri_global_shortcut_registry.rs
│       │   └── events/tauri_event_publisher.rs
│       │
│       └── ipc/                                 # adapters de ENTRADA
│           ├── mod.rs
│           ├── session_commands.rs
│           ├── audio_commands.rs
│           ├── llm_commands.rs
│           └── settings_commands.rs
│
└── scripts/package-linux.sh
```

## 4. Papel de cada camada

- **`domain`** — `Session`, `TranscriptSegment` (com `Speaker` ME/THEY), `Insight`,
  `Translation`. Sem dependências externas.
- **`application`** — casos de uso que orquestram portas; onde a regra de negócio
  vive. Sem I/O.
- **`ports`** — traits. Catálogo completo em
  [`engineering-guidelines.md`](engineering-guidelines.md#3-catálogo-de-portas-mvp).
- **`adapters`** — implementações: `cpal`/Pulse (áudio), `whisper-rs`/Silero
  (ASR/VAD), `reqwest` (LLM), `rusqlite` (DB), `keyring` (segredos), Tauri
  (overlay/atalhos/eventos). Inclui dobras de teste (`memory/`).
- **`ipc`** — traduz comandos/eventos Tauri para chamadas de caso de uso.
- **`lib.rs`** — monta adaptadores concretos e injeta nos serviços; único ponto de
  `match platform`.
- **`state.rs`** — `AppState` com `Arc<dyn Porta>` compartilhados.

## 5. Frontend (React)

- `core/domain`, `core/ports`, `core/application` espelham a separação do core
  (tipos, interfaces e hooks/use-cases).
- `adapters/ipc/*` implementam as portas chamando `invoke`/eventos; é o único
  ponto que conhece o Tauri.
- `features/*` são a UI; dependem de `core/ports` e recebem adaptadores por um
  provider React (composition root do frontend).

## 6. Arquivos compartilhados modificados ao longo das tarefas

| Arquivo | Criado em | Modificado por |
|---------|-----------|----------------|
| `src-tauri/Cargo.toml` | T00 | T01–T09 |
| `src-tauri/tauri.conf.json` | T00 | T01, T09, T10 |
| `src-tauri/src/lib.rs` | T00 | T01–T09 (wiring/DI) |
| `src-tauri/src/state.rs` | T00 | T02–T07 |
| `src-tauri/src/ipc/mod.rs` | T00 | T02–T09 |
| `src/adapters/ipc/client.ts` | T00 | T02–T08 |
| `src/App.tsx` | T06 | T07–T08 |

## 7. Integração com Tauri v2

- **Plugins:** `tauri-plugin-global-shortcut`, `tauri-plugin-store` (settings não
  sensíveis), `tauri-plugin-opener`.
- **Permissões:** `capabilities/default.json` mínimo (sem rede no frontend).
- **DI:** adaptadores Tauri (janela, atalhos, eventos) são criados no composition
  root e injetados via portas.

## 8. Decisões técnicas e alternativas

- **Tauri + Rust:** binário menor e core nativo para áudio/ASR (decisão do usuário).
- **`libpulse-binding`** para áudio do sistema (PipeWire expõe Pulse); alternativa
  futura `pipewire-rs`.
- **`whisper-rs`** para ASR/VAD (bindings maduros).
- **Wayland (2026-10-30):** o overlay continua em **XWayland** (`GDK_BACKEND=x11`);
  atalhos, screenshot e gravação usam os portais XDG
  (`GlobalShortcuts`/`Screenshot`/`ScreenCast`). `gtk-layer-shell` não entrou;
  o caminho X11 segue igual (BR-007/ISS-088/ISS-089).
