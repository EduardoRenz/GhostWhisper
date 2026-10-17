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
