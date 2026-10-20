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

## Ajustes pós-entrega (2026-10-19)

- **Sem catálogo de modelos** (BR-049): o campo de modelo virou **livre** no
  formulário de adição e no editor do card; `PROVIDER_MODELS` e
  `PROVIDER_DEFAULT_MODELS` saíram do frontend.
- **Chave de fonte no rascunho** (ISS-039): a chave no keyring passa a ser
  indexada pelo **id** da fonte (`secret_key_for`), sem depender do arquivo de
  settings; `validate` recebe a fonte inteira.
- **Log de debug do modelo** (BR-050): `tracing::debug!` por chamada com
  escopo, fonte, provider, modelo, parâmetros, latência, tokens e nível de
  raciocínio (`CompletionUsage` + parse de `usage`/`usageMetadata`).
- **Badge "Nunca usada" fixo** (ISS-040): a tela rebusca a saúde ao ganhar
  foco (a janela nasce oculta) e a validação emite `llm_source_used`.
- **Modelo visível na UI** (BR-051): `LlmSourceUsage` com `model`/`provider`/
  `scope`, hook `useLlmSource(scope)`, componente `LlmSourceBadge` no card da
  sugestão e modelo na linha de metadados do resumo.

## Critérios de aceite

- [x] As 4 abas restantes usam o layout de cards e o rodapé salva tudo.
- [x] A aba de atalhos explica que "Aplicar atalhos" registra no SO.
- [x] `source_id` de um agente que não existe mais é descartado no
      `sanitized()`.
- [x] BR-044…BR-048 registrados no `business-rules.md`.
- [x] `memory.md` do PRD atualizado com `IDs:`/`Refs:`.
- [x] Toda a verificação verde.
