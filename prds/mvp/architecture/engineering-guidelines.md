# Arquitetura — Diretrizes de Engenharia (Hexagonal)

Detalha como as regras de [`.agents/rules.md`](../../../.agents/rules.md)
(BR-011..BR-014) se aplicam ao GhostWhisper.

---

## 1. Camadas no core Rust

| Camada | Pasta | Papel | Depende de |
|--------|-------|-------|------------|
| Domínio | `src-tauri/src/domain/` | Entidades e regras puras | — |
| Aplicação | `src-tauri/src/application/` | Casos de uso | `domain`, `ports` |
| Portas | `src-tauri/src/ports/` | Traits (contratos) | `domain` |
| Adaptadores | `src-tauri/src/adapters/` | Implementações concretas | `ports`, `domain` |
| IPC | `src-tauri/src/ipc/` | Entrada (commands/events) | `application`, `ports` |
| Composition root | `src-tauri/src/lib.rs` | Monta tudo (DI) | todas |

## 2. Fluxo de dependência

```
ipc ─▶ application ─▶ ports ◀─ adapters
                        ▲
                        └── domain (contratos referenciam entidades)
```

- `application` recebe `Arc<dyn Porta>` por construtor.
- `lib.rs` decide qual adaptador concreto injetar (por config/plataforma) — é o
  **único** lugar com `match platform { ... }` de implementação.
- Testes de `application` injetam `InMemory*`/`Fake*`.

## 3. Catálogo de portas (MVP)

| Porta | Responsabilidade | Adaptadores |
|-------|------------------|-------------|
| `SessionRepository` | Persistir sessões | `LocalSessionRepository`, `InMemorySessionRepository` |
| `TranscriptionsRepository` | Persistir segmentos | `LocalTranscriptionsRepository`, `InMemoryTranscriptionsRepository` |
| `MessageRepository` | Persistir mensagens | `LocalMessageRepository`, `InMemoryMessageRepository` |
| `ChatBotProvider` | Gerar respostas/insights | `OpenAiChatBotProvider`, `GeminiChatBotProvider`, `OpenRouterChatBotProvider`, `CustomChatBotProvider`, `FakeChatBotProvider` |
| `SpeechToText` | Transcrever áudio | `WhisperSpeechToText`, `FakeSpeechToText` |
| `VoiceActivityDetector` | Detectar fala | `SileroVoiceActivityDetector` |
| `AudioSource` | Fornecer PCM | `CpalMicrophoneSource`, `PulseSystemAudioSource`, `FakeAudioSource` |
| `SecretStore` | Guardar chaves | `KeyringSecretStore`, `InMemorySecretStore` |
| `SettingsStore` | Configurações | `FileSettingsStore`, `InMemorySettingsStore` |
| `OverlayWindowController` | Controlar overlay | `TauriOverlayWindowController` |
| `GlobalShortcutRegistry` | Atalhos | `TauriGlobalShortcutRegistry` |
| `EventPublisher` | Emitir eventos p/ UI | `TauriEventPublisher`, `RecordingEventPublisher` |
| `Clock` | Tempo | `SystemClock`, `FixedClock` |

## 4. Exemplo: porta e adaptadores

```rust
// src-tauri/src/ports/chat_bot_provider.rs
#[async_trait]
pub trait ChatBotProvider: Send + Sync {
    async fn complete(&self, req: CompletionRequest) -> Result<Completion, ProviderError>;
}

// src-tauri/src/adapters/ai/openai_chat_bot_provider.rs
pub struct OpenAiChatBotProvider { client: reqwest::Client, model: String }
#[async_trait]
impl ChatBotProvider for OpenAiChatBotProvider { /* ... */ }
```

`application::insight_service` depende de `Arc<dyn ChatBotProvider>` — nunca de
`OpenAiChatBotProvider`.

## 5. Casos de uso (application) do MVP

- `SessionService`: iniciar/encerrar sessão; coordena fontes e persistência.
- `TranscriptionService`: consome `AudioSource` + `VoiceActivityDetector` +
  `SpeechToText`, persiste e publica eventos.
- `InsightService`: monta prompt com contexto e chama `ChatBotProvider`.
- `TranslationService`: tradução por segmento via `ChatBotProvider`.

## 6. Frontend (React) — espelho leve

| Pasta | Papel |
|-------|-------|
| `src/core/domain/` | Tipos/entidades do domínio (sem React) |
| `src/core/ports/` | Interfaces de repositório/serviço |
| `src/core/application/` | Hooks/use-cases (ex.: `useLiveTranscription`) |
| `src/adapters/` | Implementações via IPC (`IpcTranscriptionsRepository`) |
| `src/features/` | Componentes/UI (overlay, transcript, settings) |

Regra: componentes de UI dependem de `core/ports`; o adaptador IPC é injetado por
um provider React (composition root do frontend).

## 7. Estratégia de testes

| Alvo | Ferramenta | Fakes |
|------|-----------|-------|
| `domain` | `cargo test` (unit) | — |
| `application` | `cargo test` + `mockall` | `InMemory*`, `Fake*` |
| `adapters` (HTTP) | `wiremock` | — |
| `adapters` (DB) | SQLite in-memory | — |
| `adapters` (áudio) | WAVs de fixture | — |
| Frontend | `vitest` + Testing Library | `IpcPort` fake |

Cobertura mínima: `domain`/`application` > 80%.

## 8. IPC: main thread vs thread de bloqueio (ISS-020)

O IPC do Tauri chega na **main thread** (o loop GTK, no Linux) e um comando
`#[tauri::command]` **síncrono é chamado inline ali** — qualquer demora congela a
interface inteira: sem repaint, sem clique, sem arrastar. (`tauri-macros`: comando
`(async)` usa `respond_async_serialized` → `async_runtime::spawn`; sem o atributo,
`body_blocking` chama a função na própria thread do IPC.)

Regra:

- **`#[tauri::command(async)]`** em todo comando que possa demorar: áudio (abrir
  stream, listar dispositivos, `pactl`), rede (LLM, validação de chave — `ureq` com
  timeout de **60 s**), banco (leitura concorrente com o ASR), keyring (DBus),
  arquivos (export) e o **ciclo de sessão** (pausar/parar espera o passe final do
  turno, até 20 s: `FLUSH_WAIT`).
- **Síncrono** só para o que é instantâneo (getters de estado, `ping`) ou que
  **exige a main thread** (criação de janela em `settings_open`; operações de janela
  em `overlay_*`, que o Tauri despacha para a main thread de qualquer forma).
- O corpo de um comando `(async)` é síncrono e bloqueia **um worker** do runtime; se
  algum dia houver comandos longos concorrentes, usar `async_runtime::spawn_blocking`
  (pool dedicada) em vez de `async fn` direto.

No frontend, a regra equivalente: **estado de alta frequência não pode subir para o
`Overlay`** (o nível de áudio chega a ~10 eventos/s por canal e re-renderizaria a
conversa inteira) e nenhuma mutação que não mova regiões clicáveis pode republicar o
input-shape do X11 (ver `useInteractiveRegions`).
