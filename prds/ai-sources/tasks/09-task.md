# 09 — Demais abas, rotas por escopo e docs

- **Status:** CONCLUDED
- **Depende de:** 08
- **Habilita:** —

## Objetivo

Adaptar as outras 4 abas ao layout de cards, fechar a preparação das rotas por
escopo (BR-048) e consolidar a documentação.

## Entregáveis

- `src/features/settings/SettingsPage.tsx`: as seções "Transcrição & Áudio",
  "Assistente", "Ajustes" e "Atalhos" no layout de cards (sem botão de
  salvar individual; a aba de atalhos mantém "Aplicar atalhos", que **registra**
  no SO).
- `src/features/settings/AssistantSettings.tsx`:agents em cards; o campo
  `source_id` fica **oculto** (a UI de vínculo é da PRD futura).
- Rotas por escopo: `AssistantMode.source_id` validado contra
  `Settings.ai_sources`; `AiRouteScope` já consumido por
  `LlmController::complete_for`.
- Docs:
  - `prds/ai-sources/*` (PRD, arquitetura, tasks, memória) — concluídos;
  - `prds/business-rules.md`: BR-044…BR-048;
  - `README.md`: seção das fontes de IA.
- Verificação: `cargo test`, `cargo clippy`, `cargo fmt --check`, `pnpm test`,
  `tsc --noEmit`, `pnpm build`.

## Critérios de aceite

- [x] As 4 abas restantes usam o layout de cards e o rodapé salva tudo.
- [x] A aba de atalhos explica que "Aplicar atalhos" registra no SO.
- [x] `source_id` de um agente que não existe mais é descartado no
      `sanitized()`.
- [x] BR-044…BR-048 registrados no `business-rules.md`.
- [x] `memory.md` do PRD atualizado com `IDs:`/`Refs:`.
- [x] Toda a verificação verde.
