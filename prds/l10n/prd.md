# PRD — Localização (l10n) da interface

- **Versão:** 0.1
- **Data:** 2026-10-22
- **Status:** Implementado
- **Relacionados:** [`../business-rules.md`](../business-rules.md) (BR-010,
  BR-085, BR-086), [`memory.md`](memory.md), [`issues.md`](issues.md),
  [`architecture/l10n.md`](architecture/l10n.md),
  [`tasks/01-task.md`](tasks/01-task.md)

---

## 1. Objetivo

Permitir que o usuário escolha o idioma da interface (**português** ou
**inglês**), com o primeiro valor vindo do sistema **uma única vez** e todo
texto de frontend passando por um catálogo de traduções.

## 2. Problema

- A UI nasceu monolíngue em português. Textos espalhados em JSX e atributos
  tornam qualquer tradução uma caça a strings e uma regressão a cada tela nova.
- A escolha de idioma não existia nas Configurações, e a aba **Geral & HUD**
  ficava no fim da barra, escondendo as preferências mais usadas.
- Se o idioma fosse detectado a cada boot, a escolha do usuário seria
  sobrescrita por mudanças de locale do sistema.

## 3. Entregáveis

| # | Entregável | Escopo | Task |
|---|-----------|--------|------|
| T1 | Catálogo l10n | chaves semânticas `snake_case`, pt/en, interpolação, store global, helpers de rótulos dinâmicos | [`tasks/01-task.md`](tasks/01-task.md) |
| T2 | Configuração de idioma | `ui_language` nas settings, detecção única no core, card em **Geral & HUD** | [`tasks/01-task.md`](tasks/01-task.md) |
| T3 | Aba inicial | **Geral & HUD** vira a primeira aba (e o default) | [`tasks/01-task.md`](tasks/01-task.md) |
| T4 | Regra de produto | frontend sempre traduzido; proibição de copy hardcoded | [`../../.agents/rules.md`](../../.agents/rules.md) §5.1 |

## 4. Histórias e critérios de aceite

### US-01 — Escolher o idioma

- **CA-01.1** Configurações abre em **Geral & HUD**, primeira aba.
- **CA-01.2** Há um card **Idioma da Interface** com as opções **Português
  (Brasil)** e **English (US)**.
- **CA-01.3** A escolha é persistida em `Settings.ui_language` e reflete em
  todas as janelas abertas via `settings_changed`.
- **CA-01.4** "Restaurar Padrões" não troca o idioma escolhido.

### US-02 — Primeira inicialização

- **CA-02.1** Sem `ui_language` persistido, o core resolve `pt*` → `pt` e
  qualquer outro locale (ou ausência) → `en`.
- **CA-02.2** A resolução é persistida **uma vez** (`UiLanguageMigration`).
- **CA-02.3** Depois da primeira resolução, nenhum boot/migração altera o valor,
  mesmo que o locale do sistema mude.

### US-03 — Copy localizada

- **CA-03.1** Nenhum texto visível de UI é hardcoded: tudo passa por `t()` ou
  por um helper que devolve `TranslationKey`.
- **CA-03.2** Chaves são semânticas e em `snake_case`; pt e en têm o mesmo
  conjunto de chaves e os mesmos placeholders.
- **CA-03.3** Dados do usuário/LLM (nomes de agentes, transcrições, respostas,
  títulos de sessão) não são traduzidos.
- **CA-03.4** Trocar o idioma re-renderiza a UI sem recarregar a janela.

## 5. Requisitos não funcionais

- **RNF-01 — Testabilidade:** catálogo, resolução de locale, migração e store
  cobertos por testes puros (R7/R8/BR-013).
- **RNF-02 — Offline/local-only:** nenhuma chamada de tradução de interface;
  catálogos embarcados (BR-002/BR-004).
- **RNF-03 — Arquitetura:** detecção via porta `LocaleSource`; migração em
  `application`; composição em `lib.rs` (BR-011).
- **RNF-04 — Documentação única:** BR-085/BR-086, R18–R22 e esta PRD (R14).

## 6. Não-objetivos

- Traduzir transcrições/respostas do LLM (isso é a feature de tradução de
  conteúdo, PRD `translation`).
- Mais de dois idiomas nesta entrega.
- Traduzir o menu da bandeja/strings internas do core nesta primeira versão.
- Detectar idioma no frontend ou a cada boot.

## 7. Riscos

| Risco | Mitigação |
|---|---|
| Chave nova sem tradução | teste de paridade pt/en + fallback pt |
| Texto hardcoded passar na revisão | R18/R22 e teste de catálogo; revisão obrigatória |
| Locale do sistema sobrescrever a escolha | migração só com `ui_language = None` |
| Duplicar tradução de dados do usuário | helpers só para enums de UI; dados passam direto |

## 8. Métricas de sucesso

- Trocar pt↔en reflete em todas as janelas abertas.
- Segunda inicialização mantém a escolha mesmo com `LANG` diferente.
- Nenhum texto visível hardcoded nos componentes do frontend.
