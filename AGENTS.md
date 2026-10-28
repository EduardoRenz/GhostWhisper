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
src/core/l10n/{locales,labels.ts,catalog.ts}   # pt/en, chaves snake_case
prds/{business-rules.md, <prd>/{prd.md,memory.md,issues.md,architecture,tasks}}
```

## Fluxo de trabalho (spec-driven)
1. Descreva a mudança em `prds/mvp/` (PRD/arquitetura/task) antes de codar.
2. Implemente em uma única camada por vez, dependendo só de ports.
3. Escreva/exécute testes (`cargo test`, `pnpm test`).
4. Atualize a task (`CONCLUDED`) e o `memory.md` **do PRD**
   ([`prds/mvp/memory.md`](prds/mvp/memory.md)).
5. Referencie docs/IDs (`BR-015`, `ISS-011`, `12-task`) em vez de duplicar
   conteúdo (R14, BR-016); histórico por PRD, formato buscável (R15, BR-017).

## Comandos
```bash
git submodule update --init   # vendor/laya; o Cargo resolve mesmo sem a feature
pnpm install
pnpm tauri dev        # rodar em desenvolvimento
pnpm tauri build      # gerar binários (.deb/AppImage)
pnpm tauri:build:laya # AppImage com Vulkan + engine/modelo Laya (POC)
cargo test            # testes do core
cargo clippy          # lint Rust
pnpm test             # testes do frontend
```

## Versionamento
- A versão é única nos três manifestos: `package.json`,
  `src-tauri/tauri.conf.json` e `src-tauri/Cargo.toml`. O `Cargo.lock` acompanha.
- Bump: `./scripts/bump-version.sh X.Y.Z` (atualiza os três, o README e roda
  `cargo check` para o lock). Não edite só um deles.
- O nome do AppImage/deb usa a versão dos manifestos; faça o bump **antes** do
  `pnpm build:final` da release.
- O build remove AppImages de versões antigas (`scripts/prune-appimages.sh`):
  só o artefato da versão atual fica nos diretórios de bundle.
- Próxima release planejada: **0.2.0**.

## Regras rápidas
- **Perssua é a referência de produto:** ao decidir *como* resolver algo, priorize
  a solução que o Perssua já usa (produto maduro); transpomos só a tecnologia
  (Electron→Tauri, JS→Rust), não o comportamento (BR-015, R11–R13).
- Não criar backend, login ou telemetria (BR-002, BR-004).
- Não copiar código/assets/marca de terceiros (BR-008).
- Não instanciar adaptadores fora de `lib.rs`.
- Nunca logar segredos ou áudio.
- Toda porta tem uma dobra de teste.
- **Todo bug corrigido ganha teste anti-regressão** que falharia no código
  anterior; causas de WM/timing devem ser isoladas em lógica pura testável
  (R16/R17).
- **Documentação é fonte única:** cite arquivos e IDs (`BR-015`, `ISS-011`,
  `12-task`) em vez de duplicar conteúdo (R14, BR-016).
- **Sessão reportada vira regressão:** resultado inesperado → extraia a conversa
  da sessão e crie teste de integração/regressão com o prompt/Laya até acertar;
  proibido palavra-chave/idioma (R23/R24).
- **Classificação sem palavra-chave/idioma:** decisões do classificador
  (`question`, `comment`, `already_answered`, `no_action`) usam o modelo
  multilíngue/sinais estruturais; listas de gatilhos por idioma são proibidas,
  inclusive como guarda (R23). Se o modelo abstém, não sugerir.
- **Frontend sempre via l10n:** nenhum texto visível hardcoded. Use
  `t("chave_em_snake_case")` (`src/core/l10n`) com entrada em **pt** e **en**;
  dados do usuário/LLM não são traduzidos (R18–R22, BR-086).
- **Idioma da interface:** escolha do usuário em Configurações → Geral & HUD
  (primeira aba). A detecção do sistema roda **uma vez** na primeira
  inicialização e nunca sobrescreve a escolha (BR-085, R21).
- **Histórico por PRD:** `memory.md` vive em `prds/<prd>/memory.md`; entradas
  `## YYYY-MM-DD — Título` cronológicas com linha `IDs:` (R15, BR-017).
