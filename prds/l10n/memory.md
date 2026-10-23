# Histórico — PRD l10n

Histórico deste PRD no formato fixo do
[`rules.md`](../../.agents/rules.md) §6.2 (busca por palavra-chave, ID e período).

| Objetivo | Comando |
|----------|---------|
| Palavra-chave | `rg -n -i 'l10n\|ui_language\|snake_case' prds/l10n/memory.md` |
| Entrada do dia | `rg -n '^## 2026-10-22' prds/l10n/memory.md` |
| Por ID | `rg -n 'BR-085\|BR-086\|R18' prds/l10n/memory.md` |

---

## 2026-10-22 — l10n pt/en, idioma do sistema uma única vez e Geral & HUD primeiro

- **IDs:** BR-010 (revisada), BR-085, BR-086, R18–R22, T01
- **Refs:** [`prd.md`](prd.md), [`issues.md`](issues.md),
  [`architecture/l10n.md`](architecture/l10n.md),
  [`tasks/01-task.md`](tasks/01-task.md),
  [`../business-rules.md`](../business-rules.md),
  [`../../.agents/rules.md`](../../.agents/rules.md) §5.1

### O que o usuário pediu
Transformar a UI em l10n (pt/en), com a escolha em Configurações → Geral & HUD,
essa aba como primeira, idioma inicial detectado do sistema **só na primeira
inicialização** (depois nunca muda sozinho) e uma regra explícita de que o
frontend nunca hardcoda texto. Numa revisão do trabalho, o usuário pediu que as
chaves de tradução fossem nomes semânticos em **snake_case** (não o texto
português como chave) — o catálogo foi refeito nesse formato.

### Decisões
- `ui_language: Option<String>` no core; `None` só antes da resolução. O boot
  grava `pt` para locale `pt*` e `en` para qualquer outro/ausente.
- `UiLanguageMigration` depende de `SettingsStore` + `LocaleSource`, então não
  há I/O em `application`; `EnvLocaleSource` é o adapter Linux-first.
- No frontend, chaves `feature_contexto` (`settings_save_changes`,
  `overlay_start_listening`), catálogos planos pt/en e `t()` tipado por
  `TranslationKey`; enums usam helpers de `core/l10n/labels.ts`.
- "Restaurar Padrões" preserva `ui_language`; dados do usuário/LLM não passam
  por `t`.
- A aba `general` virou a primeira e o default de `SettingsPage`.

### Testes
- `src/core/l10n/index.test.ts`: paridade de chaves/placeholders, interpolação,
  normalização, locale e store.
- `LanguageSettings.test.tsx`: opções, troca e re-render pt→en.
- `SettingsPage.test.tsx`: primeira aba ativa e card de idioma.
- Rust: 8 testes de `ui_language` (domínio + migração).
- `pnpm test` (434), `pnpm exec tsc --noEmit` e `cargo test ui_language` verdes.

### Limites
- Erros do core e menu da bandeja continuam no idioma do core; tradução por
  código de erro é evolução futura (architecture §6).
