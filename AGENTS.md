# AGENTS.md — GhostWhisper

Guia para agentes e contribuidores. Regras completas em
[`.agents/rules.md`](.agents/rules.md); especificação em [`prds/`](prds/).

## O que é
Copiloto de reunião **open-source, Linux-first e local-only** (sem conta, sem
telemetria, sem backend). Ouve o áudio, transcreve, traduz e sugere respostas numa
janela overlay discreta.

## Stack
**Tauri v2 (Rust)** + **React/Vite/TS** · áudio `cpal` + PipeWire/PulseAudio ·
VAD Silero + ASR `whisper-rs` · LLM via HTTP (BYO key) · SQLite (`rusqlite`, FTS5)
· segredos no keyring.

## Premissas de arquitetura (obrigatórias)
1. **Clean/Hexagonal:** `domain → application → ports → adapters`, com inversão de
   dependência. `domain`/`application` nunca conhecem `adapters`.
2. **Injeção de dependência** por construtor; composition root só em
   `src-tauri/src/lib.rs`; `AppState` guarda `Arc<dyn Porta>`.
3. **SOLID** em todas as camadas; portas pequenas e específicas.
4. **Altamente testável:** nada de I/O em `domain`/`application`; fakes para toda
   porta; cobertura alta no núcleo.
5. **Nomenclatura:** sem prefixo `I`. Abstração primeiro, implementação com
   prefixo de contexto:
   - `TranscriptionsRepository` → `LocalTranscriptionsRepository`
   - `ChatBotProvider` → `OpenAiChatBotProvider`, `GeminiChatBotProvider`
   - `SpeechToText` → `WhisperSpeechToText`
   - Test doubles: `InMemoryTranscriptionsRepository`, `FakeChatBotProvider`

## Estrutura (resumo)
```
src-tauri/src/{domain,application,ports,adapters,ipc,lib.rs,state.rs}
src/{core,adapters,features}
prds/{business-rules.md,memory.md,mvp/{prd.md,architecture,tasks}}
```

## Fluxo de trabalho (spec-driven)
1. Descreva a mudança em `prds/mvp/` (PRD/arquitetura/task) antes de codar.
2. Implemente em uma única camada por vez, dependendo só de ports.
3. Escreva/exécute testes (`cargo test`, `pnpm test`).
4. Atualize a task (`CONCLUDED`) e `prds/memory.md`.

## Comandos
```bash
pnpm install
pnpm tauri dev        # rodar em desenvolvimento
pnpm tauri build      # gerar binários (.deb/AppImage)
cargo test            # testes do core
cargo clippy          # lint Rust
pnpm test             # testes do frontend
```

## Regras rápidas
- Não criar backend, login ou telemetria (BR-002, BR-004).
- Não copiar código/assets/marca de terceiros (BR-008).
- Não instanciar adaptadores fora de `lib.rs`.
- Nunca logar segredos ou áudio.
- Toda porta tem uma dobra de teste.
