# Histórico — PRD Assistentes

Histórico deste PRD no formato fixo do
[`rules.md`](../../.agents/rules.md) §6.2 (busca por palavra-chave, ID e período).

| Objetivo | Comando |
|----------|---------|
| Palavra-chave | `rg -n -i 'semeadura\|summary_prompt' prds/assistants/memory.md` |
| Entrada do dia | `rg -n '^## 2026-10-21' prds/assistants/memory.md` |
| Por ID | `rg -n 'BR-074\|BR-075\|BR-076\|BR-077\|BR-078' prds/assistants/memory.md` |

---

## 2026-10-21 — PRD Assistentes: painel de gestão de agentes nas Configurações

- **IDs:** BR-074, BR-075, BR-076, BR-077, BR-078, T01–T09
- **Refs:** [`prd.md`](prd.md), [`issues.md`](issues.md),
  [`architecture/assistants-panel.md`](architecture/assistants-panel.md),
  [`tasks/_tasks.md`](tasks/_tasks.md),
  [`../business-rules.md`](../business-rules.md)

### Como a PRD nasceu
O usuário mandou o mockup da nova tela de Configurações com uma aba
**"Assistentes"** — o agente Padrão no topo, os agentes configurados em cards
(ícone, nome, badge "Ativo" e os prompts dentro do card) e o botão "Criar Novo
Assistente de IA". O pedido: **mudar de lugar e melhorar a questão dos
agentes** — hoje o painel é um formulário com editor inline.

O mockup é **informativo**: os nomes e os textos dele são de exemplo. O que
vale é a estrutura e o estilo. Os dados reais são os que já existem no app
(agentes, nomes e prompts cadastrados).

### Decisões do usuário (2026-10-21)

1. **Agentes de fábrica:** tirando o `default` (protegido), todo o resto vem
   "Padrão de fábrica" e o usuário pode **editar e excluir permanentemente**
   ⇒ semeadura única dos 4 agentes na lista do usuário (BR-075).
2. **Conteúdo antigo da aba:** cadências e perguntas prontas vão para uma
   **nova seção na aba "Geral & HUD"** ⇒ a aba "Assistentes" fica só com o
   painel.
3. **Agente ativo:** **não é selecionável no painel** — o usuário segue
   selecionando na tela de sugestões (único lugar implementado). A badge
   "Ativo" é informativa.
4. **Casca:** **restilizar a barra de abas em pílulas** (o badge de versão
   "GhostWhisper 2.4" do mockup fica de fora).

### O que o mockup virou no código

| Mockup | No app |
|---|---|
| Aba "Assistentes" (pill) | `TABS[3]` renomeada; barra em pílulas (BR-078) |
| Card "★ Assistente Padrão" + "Primário" | agente `default` (BR-059), card read-only no topo |
| "AGENTES CONFIGURADOS (3)" | `custom_assistant_modes` (a lista do usuário) |
| Card com tile, nome e "Ativo" | `AgentCard` — "Ativo" compara com `assistant_mode_id` |
| "⚡ Prompt de Sugestão" / "📄 Prompt de Resumo" | `suggestion_prompt` (existe) + `summary_prompt` (novo) |
| Botão "Editar" | abre o **modal** (novo primitivo `Dialog` in-page) |
| "Criar Novo Assistente de IA" | mesmo modal, modo de criação |

### Pontos que ainda são de outra feature

- O `summary_prompt` é **persistido, mas não consumido**: o `SummaryService`
  continua com o contrato global (`SUMMARY_SYSTEM_PROMPT`). Consumir o prompt
  por agente é PRD própria (BR-076).
- **"Restaurar padrão"** de um agente de fábrica (voltar ao contrato original)
  ficou de fora — não há como distinguir origem de agente editada.
- **Vincular fonte de IA por agente** (BR-048) segue com o modelo pronto e sem
  UI.

### Ordem de implementação

Core primeiro (01–03: campo novo, semeadura, IPC), depois o frontend (04:
tipo + `Dialog` + ícones; 05: painel; 06: modal; 07: move das cadências;
08: pílulas) e o fechamento (09).

## 2026-10-22 — Revisão da PRD (aderência ao mockup e às camadas)

- **IDs:** BR-074, BR-075, BR-076, BR-077, BR-078, T01–T09
- **Refs:** [`prd.md`](prd.md) §5/§7/§8/§10,
  [`architecture/assistants-panel.md`](architecture/assistants-panel.md) §3/§5/§7/§9,
  [`tasks/_tasks.md`](tasks/_tasks.md)

### O que a revisão conferiu
Reli a conversa e o mockup, e comparei com a PRD: cada elemento do mockup tem
uma decisão registrada (§5) e cada pedido do usuário tem regra (BR-074–078) e
task. O que **não** entra está explícito no §4.2 (badge de versão, taxonomia
completa de abas, seleção do agente ativo pelo painel, restaurar um agente
individual, vincular fonte por agente).

### Correções feitas (o que estava errado ou ambíguo)

1. **Violação de camada:** o plano dizia que `Settings::default()` já nasceria
   com os 4 agentes de fábrica, mas `domain/settings.rs` não pode chamar
   `application::prompts::assistant` (R1). Corrigido: o default do domínio fica
   **puro** (lista vazia, marca `false`); a aplicação materializa
   (`default_settings()`), tanto no boot quanto no `settings_defaults`.
2. **"Restaurar Padrões" perderia os prompts de fábrica:** o frontend não tem
   os contratos. Corrigido: comando `settings_defaults` devolve o default
   materializado; o botão passa a usá-lo (task 03).
3. **Ordem de remoção do teste:** `AssistantSettings.test.tsx` estava previsto
   para sair só na task 09, o que quebraria a suíte entre as tasks 05 e 09.
   Agora sai junto com o componente (task 05).
4. **Fixtures desatualizadas:** os literais `AssistantMode` (Rust em 5 arquivos
   de teste; TS em `AgentSelector.test.tsx`) precisam de `summary_prompt` —
   registrado nas tasks 01/04/09.
5. **Níveis de detalhe do mockup:** o mockup mostra três tratamentos de card
   (completo → compacto → só descrição). Ficou decidido que são **ilustrativos**;
   a app usa um formato único (completo, texto truncado) — §5.
6. **Badge "Ativo" no card do Padrão:** quando o Padrão é o agente em uso
   (instalação nova), o card do topo mostra "Primário" + "Ativo" — §5.1.
7. **Prompt de resumo como campo nomeado** (não um mapa `tela → prompt`) e
   **prompts exibidos como estão** (hoje em inglês) — §5.1.
8. **Código que perde uso:** `find_mode` (o `resolve_mode` não consulta mais os
   builtins) e `is_builtin_id` (deixa de reservar id) foram explicitados na
   task 01; o painel usa `isProtectedMode`, não `isBuiltinMode` (task 09).
9. **CSS inexistente:** `.settings__section` (nova, mesmas regras de
   `.settings__hud`) e os botões de ação reaproveitando `.settings__button--small`
   e `.icon-button` — nada de `--ghost`/`--compact`, que não existem (task 14
   de ai-sources ainda é `PENDING`).
10. **Detalhes de estilo do mockup:** ícones dos painéis (`BoltIcon` /
    `SummaryIcon`), título de seção em caixa alta, tile na cor do agente via
    `--agent-color`, CTA como linha de borda sutil (não tracejada) e fallback
    estático do card do Padrão fora do Tauri.

### Resultado
A PRD passa a v0.2 sem mudança de escopo: o que o usuário pediu e o que o
mockup mostra (exceto o que ele excluiu) tem decisão, regra e task.

## 2026-10-22 — Implementação da PRD Assistentes (tasks 01–09)

- **IDs:** BR-074, BR-075, BR-076, BR-077, BR-078, T01–T09, ISS-060
- **Refs:** [`prd.md`](prd.md), [`issues.md`](issues.md),
  [`architecture/assistants-panel.md`](architecture/assistants-panel.md),
  [`tasks/_tasks.md`](tasks/_tasks.md)

### O que foi entregue

**Core (Rust)**
- `AssistantMode` ganhou `summary_prompt` (BR-076), com `serde(default)`;
  `validate_mode` passou a reservar **só** o id protegido (`default`) e a
  contar o prompt de resumo no "ao menos um prompt".
- O catálogo virou `default_mode()` (protegido) + `factory_modes()` (4 de
  fábrica) + `effective_modes()`; `resolve_mode` não consulta mais os
  builtins — agente excluído não volta.
- `Settings` ganhou `assistant_modes_seeded`; o default do domínio ficou
  **puro** (sem agentes). A aplicação materializa:
  `AssistantCatalogMigration::seed()` (boot) e `default_settings()`
  (`settings_defaults`).
- Novos comandos: `settings_defaults`; `assistant_modes_list` devolve o
  catálogo efetivo (protegido + lista).

**Frontend**
- `AssistantsSettings` + `AgentCard` (painel do mockup) no lugar de
  `AssistantSettings` (removido com o teste).
- `AgentEditorDialog` + primitivo `Dialog` (portal, `Esc`, backdrop, foco
  preso) para criar/editar/duplicar com os mesmos campos.
- `CadenceSettings` levou cadências e perguntas prontas para "Geral & HUD".
- Aba renomeada para "Assistentes"; barra de abas em pílulas (BR-078).
- Ícones `PencilIcon`, `StarIcon`, `PlusIcon`; `summary_prompt` no espelho TS.

### Decisões que apareceram na implementação
- **`find_mode`** passou a receber só a lista do usuário (o protegido é
  resolvido à parte) e o `is_builtin_id` voltou a ser necessário: o
  `sanitized` roda **antes** da semeadura e precisa aceitar um id de fábrica
  nessa janela (ISS-060).
- **Modal em portal** (`document.body`): sem isso o `position: fixed` sofria
  com o scroll da aba (`.settings__body`).
- **`settings__button--primary` estava antes da base no CSS** e perdia a cor
  cheia na cascata — movido para depois da base (vale para todas as telas).
- **"Editar" neutro** no card (`.agent-card__edit`): o mockup usa um botão
  discreto, não o accent.
- O **aviso de versão** ("GhostWhisper 2.4") e a taxonomia completa de abas
  do mockup continuam fora do escopo (§4.2).

### Verificação
- `cargo test --lib`: 626 testes, 0 falhas · `cargo clippy --all-targets`
  limpo.
- `pnpm test`: 403 testes, 0 falhas · `tsc --noEmit` e `pnpm build` limpos.
- Conferência visual no navegador (`?window=settings&tab=assistant`) com dados
  de exemplo: pílulas, card do Padrão, cards (tile/badges/prompts/CTA),
  destaque do agente ativo e modal — aderente ao mockup.
- `pnpm tauri:build` (Vulkan) concluído a cada mudança de core/UI (pill,
  BR-079 e BR-080): `GhostWhisper_0.1.0_amd64.deb` (17 MiB) e `.AppImage`
  (93 MiB) em `src-tauri/target/release/bundle/`. O bundle final embute
  `index-BODkkvJM.js` / `index-D8gaLzlP.css` e o loader do Vulkan
  (rebuild após o refine das perguntas em aberto — autoria de THEY, rev 5 do
  catálogo, BR-090).
- `cargo fmt --check` **não** está limpo no repositório por arquivos
  anteriores à entrega (ex.: `ask_service.rs`, `recording_service.rs`,
  `connection.rs`); os arquivos alterados nesta PRD foram formatados.

## 2026-10-22 — Revisão visual: pill "Markdown AST" removida

- **IDs:** BR-074 · **Refs:** [`prd.md`](prd.md) §5,
  [`architecture/assistants-panel.md`](architecture/assistants-panel.md) §7.2,
  [`tasks/05-task.md`](tasks/05-task.md)

- **Pedido:** o usuário revisou a tela e pediu para tirar a pill "Markdown AST"
  do canto do painel "Prompt de Resumo" — ela aparecia em **todo** card e não
  acrescentava informação (o prompt já é Markdown por contrato).
- **Mudança:** `PromptPanel` perdeu a prop `tag` e o `<span class="prompt-panel__tag">`;
  a regra `.prompt-panel__tag` saiu do CSS; o teste do `AgentCard` passou a
  garantir a **ausência** do texto.
- **Docs:** a linha da pill saiu do mapa de classes (§7.2) e o mockup §5 ficou
  com a nota de que a etiqueta foi removida nesta revisão.

## 2026-10-22 — Regra de fallback: prompt vazio herda o do Padrão (BR-079)

- **IDs:** BR-079, T10, BR-076 · **Refs:** [`prd.md`](prd.md) §7.4,
  [`architecture/assistants-panel.md`](architecture/assistants-panel.md) §3.2,
  [`tasks/10-task.md`](tasks/10-task.md)

- **Pedido:** "por via de regra, se algum dos prompts estiverem vazios (trate
  com trim e tudo mais também), deve usar o prompt do agente padrão sempre".
- **Antes:** um `suggestion_prompt` vazio caía no contrato de **Reunião** (o
  fallback antigo) e um `system_prompt` vazio simplesmente não entrava no
  prompt — o agente ficava sem critério nenhum.
- **Agora:** `effective_suggestion_prompt` / `effective_system_prompt` /
  `effective_summary_prompt` devolvem o valor do agente ou, quando vazio depois
  do `trim`, o do **Padrão**. O `## Domain criteria` passa a ser sempre
  incluído. O `context` fica fora (é contexto, não prompt).
- **UI:** o card resolve o prompt efetivo e mostra "Herdado do agente Padrão"
  quando o valor veio do Padrão (sem pill; a nota fica abaixo do texto).
- **Resumo:** o helper já resolve o vazio, mas o `SummaryService` continua com o
  contrato global (BR-076) — a feature futura só precisa trocar a fonte.
- **Validação:** "ao menos um prompt/contexto" continua exigido; a herança é
  por campo, não um atalho para criar agente vazio.

## 2026-10-22 — Seletor global de agentes acima do notch (BR-080)

- **IDs:** BR-080, T11 · **Refs:** [`prd.md`](prd.md) §12,
  [`architecture/assistants-panel.md`](architecture/assistants-panel.md) §11,
  [`tasks/11-task.md`](tasks/11-task.md)

- **Pedido:** o seletor de agentes passa a ficar **flutuando acima do notch**,
  compacto e discreto, **global e em tempo real**; sai da tela de Sugestões.
- **Geometria:** o overlay não tinha espaço acima da barra (janela ancorada no
  topo útil). Entrou `AGENT_SELECTOR_BAND = 34 px`: a janela cresce para baixo
  e a barra desce 34 px; o topo da janela continua em `y = work_area.y`. A
  largura passou a medir a pilha (seletor + barra), não só a barra.
- **Componente:** `AgentSwitcher` (pílula 26 px: ícone na cor + nome + divisor +
  "Alternar agente" + chevron; popover com teclado/`Esc`/clique fora). O
  `AgentSelector` da janela de Sugestões foi removido.
- **Tempo real:** comando novo `settings_set_assistant_mode` (grava só o campo
  e emite `settings_changed`); o overlay já assina `useSettings`; a janela de
  Sugestões passou a ler `modeId` direto das settings; a página de
  Configurações assina o evento só para sincronizar a badge "Ativo".
- **Popovers:** a altura extra é agregada (`max` entre menu de canais 180 e menu
  de agentes 240), e o catálogo recarrega só quando os agentes mudam (assinatura
  id/nome/ícone/cor), não a cada autosave da opacidade.
- **Fora do escopo:** arrastar o seletor, menu de contexto no notch e prompts no
  seletor.

## 2026-10-22 — Tooltips de ajuda nos campos de prompt (BR-081)

- **IDs:** BR-081, T12 · **Refs:** [`prd.md`](prd.md) §13,
  [`architecture/assistants-panel.md`](architecture/assistants-panel.md) §12,
  [`tasks/12-task.md`](tasks/12-task.md)

- **Pedido:** explicar o que são os "campos avançados"/prompts avançados dos
  agentes, como eram usados antes, e adicionar um tooltip em cada campo na UI.
- **Explicação canônica:** PRD §13 ganhou a tabela de campos (uso real hoje,
  fallback quando vazio) e a ordem de montagem do prompt da sugestão. O
  "avançado" é só um agrupamento visual (`<details>`); o comportamento é o
  mesmo do editor antigo.
- **UI:** `InfoTooltip` (primeiro tooltip do design system) + `InfoIcon`, com
  hover **e** foco; campos do modal passaram a usar `<label htmlFor>` + `id`
  para o botão de ajuda não virar o controle do label. O card também tem ajuda
  nos dois painéis.
- **Texto importante:** o Prompt de Resumo diz que ainda **não** é consumido
  (BR-076) — evita a leitura de que editar o campo já muda o Resumo.

## 2026-10-22 — Correção: notch escondido atrás de rolagem (ISS-061)

- **IDs:** ISS-061, BR-080 · **Refs:** [`issues.md`](issues.md),
  [`tasks/11-task.md`](tasks/11-task.md), `src/styles/overlay-layout.test.ts`

- **Sintoma:** ao crescer (início de sessão/menu), a barra ficava cortada e
  escondida por uma barra de rolagem na janela do overlay.
- **Causa:** `.overlay__stack` tinha `max-width: 100%`. A medição da largura
  (que agora observa a pilha, BR-080) era clampada à janela atual: a barra
  crescia, o CSS clampava, a janela nunca crescia — e a rolagem escondia o
  notch.
- **Correção:** pilha com `width: max-content` **sem** `max-width`; a classe
  `.overlay-window` (posta pelo `Overlay`) trava `overflow: hidden` em
  `html/body/#root` **só no overlay**. Nenhuma barra de rolagem pode esconder
  o notch.
- **Regressão (R16):** `src/styles/overlay-layout.test.ts` falharia no CSS
  anterior (max-width na pilha e ausência do lock de overflow).

### Correção dos tooltips (2026-10-22)

- **Sintoma:** os balões ficavam ocultos — principalmente o do painel da
  direita do card — porque eram `absolute` dentro de contêineres com
  `overflow` e podiam sair da janela.
- **Correção:** o balão agora é renderizado em **portal no `body`** com
  `position: fixed`; a posição vem de `tooltipPosition` (função pura em
  `src/core/domain/tooltip.ts`): prefere abaixo, sobe quando não há espaço,
  centraliza e prende nas bordas; recalcula em scroll/resize. Testes em
  `tooltip.test.ts` cobrem os casos (abaixo, acima, bordas, janela pequena).

## 2026-10-22 — Contrato comum de leitura da transcrição (BR-082)

- **IDs:** BR-082 · **Refs:** [`prd.md`](prd.md) §13,
  [`architecture/assistants-panel.md`](architecture/assistants-panel.md) §3.2,
  [`tasks/13-task.md`](tasks/13-task.md)

- **Pedido:** regra geral do app (transcrição de fala, ME/THEY, correção de ASR,
  origem da fala) não deve ser copiada no prompt de cada agente; deve estar
  **pré-embutida** no system prompt de todos.
- **Decisão:** `TRANSCRIPT_CONTRACT` é injetado no início de
  `suggestion_system_prompt` (protegido, de fábrica e personalizado) e de
  `fast_first_system_prompt`. O campo editável do agente fica só com o que é da
  cena; as linhas duplicadas de ME/THEY saíram dos 5 prompts de fábrica.
- **Busca:** `ASK_RULES` explica ME/THEY e a atribuição imperfeita no bloco
  `<transcript>`, sem herdar o contrato ao vivo inteiro (lá a pergunta do
  usuário é a prioridade, não a fala do THEY). O resumo mantém o contrato
  profundo (normalização, classificação, estrutura) e ganha a definição de
  ME/THEY.
- **Instalações existentes:** a migração de boot remove os trechos antigos dos
  agentes de fábrica **já semeados** (só os trechos conhecidos; o resto do texto
  e as edições do usuário ficam), de forma idempotente; agentes personalizados
  não são tocados.
- **Testes:** 634 no `cargo test --lib`, incluindo agente personalizado,
  fast-first, a ausência de repetição nos prompts de fábrica e a normalização.

## 2026-10-22 — Entrevista Técnica funde System Design; novo Entrevista com RH (BR-083)

- **IDs:** BR-083 · **Refs:** [`prd.md`](prd.md) §7.2/§7.3,
  [`architecture/assistants-panel.md`](architecture/assistants-panel.md) §3.4,
  [`tasks/14-task.md`](tasks/14-task.md)

- **Pedido:** o catálogo estava granular demais; fundir Entrevista Técnica com
  System Design e criar **Entrevista com RH** com três focos: perguntas ainda
  não feitas/respondidas (vaga, planos, benefícios), aprofundamento quando ME
  foi vago (ancorado no que o RH disse/cargo) e STAR quando cabível.
- **Decisão:** `technical_interview` absorve o system design (clarify → size →
  design, QPS, ASCII, componentes genéricos); `system_design` sai; entra
  `hr_interview` (`HR_SYSTEM_PROMPT` + `HR_SUGGESTION_PROMPT`).
- **Migração:** `assistant_catalog_revision` (0 → 1) roda uma vez em instalações
  já semeadas: remove `system_design`, sobrescreve o técnico com o prompt
  fundido (preservando `context`/`source_id`; carrega contexto/fonte do system
  design quando o técnico não tinha), adiciona o RH e troca a seleção ativa de
  `system_design` para `technical_interview`. Depois da revisão, edições e
  exclusões são respeitadas; "Restaurar Padrões" usa o catálogo novo.
- **Testes:** 638 no `cargo test --lib`; 422 no `pnpm test`; `tsc` limpo.

## 2026-10-22 — Resumo por agente ativo + debrief de RH (BR-076/BR-084)

- **IDs:** BR-076, BR-084 · **Refs:** [`prd.md`](prd.md) §13.4,
  [`architecture/assistants-panel.md`](architecture/assistants-panel.md) §3.6,
  [`tasks/15-task.md`](tasks/15-task.md)

- **Pedido:** colocar o `summary_prompt` em prática com a mesma regra de
  fallback (vazio = Padrão) e testar um contrato específico para a **Entrevista
  com RH**.
- **Decisão:** o `SummaryService` resolve o agente ativo
  (`assistant_mode_id`) e usa `effective_summary_prompt` na primeira geração e
  na regeneração manual. As atualizações incrementais usam
  `SUMMARY_UPDATE_SYSTEM_PROMPT` e preservam a estrutura do resumo corrente;
  trocar de agente no meio da sessão só muda a estrutura na próxima
  regeneração/sessão.
- **RH:** `HR_SUMMARY_PROMPT` com **Overall summary**, **What I answered**,
  **What the recruiter said about the company**, **Benefits and salary
  expectations**, **Main questions and answers** e **Brief opinion and insight
  on how I did**; seções vazias dizem "Not discussed so far in the session."
  (traduzido).
- **Migração:** revisão de catálogo **2** troca o `summary_prompt` do RH já
  semeado somente se ele ainda for o contrato geral; prompt editado/vazio é
  preservado e um RH excluído não volta.
- **Testes:** 643 no `cargo test --lib`; 422 no `pnpm test`; `tsc` limpo.

## 2026-10-23 — Sugestão: rascunho e resposta completa em paralelo (BR-088)

- **IDs:** BR-088 · **Refs:** [`prd.md`](prd.md) §13.1,
  [`architecture/assistants-panel.md`](architecture/assistants-panel.md) §3.7,
  [`tasks/16-task.md`](tasks/16-task.md)

- **Sintoma:** a resposta completa demorava porque o passe `fast-first` (não
  streaming, até 420 tokens) precisava terminar antes de o passe completo
  começar.
- **Correção:** o rascunho roda em uma thread de escopo e o completo no thread
  atual, em paralelo. Um `AtomicBool` `full_emitted` marcado no primeiro delta
  do completo descarta o rascunho atrasado; falha do rascunho só loga. A UI
  mantém um `Set` de gerações com completa recebida e ignora `quick` atrasada
  da mesma geração.
- **Trade-off:** duas chamadas concorrentes ao mesmo provider; o rascunho é
  descartável e a completa continua se ele falhar.
- **Testes:** 658 no `cargo test --lib`; 454 no `pnpm test`; `tsc` limpo.

## 2026-10-23 — Resumo: contrato comum injetado + debrief da Entrevista Técnica (BR-090)

- **IDs:** BR-090 · **Refs:** [`prd.md`](prd.md),
  [`architecture/assistants-panel.md`](architecture/assistants-panel.md) §3.2/§3.4/§3.6,
  [`tasks/17-task.md`](tasks/17-task.md)

- **Pedido:** resumo próprio para a Entrevista Técnica (que usava o contrato de
  reunião) e, na mesma passada, "não haver repetição": as regras de
  ME/THEY + erro de ASR + normalização, que já eram comuns, vão para um
  `SUMMARY_TRANSCRIPT_CONTRACT` pré-injetado no `SummaryService` (padrão da
  BR-082 nas sugestões).
- **Mudanças:** contrato injetado nas três chamadas do resumo (primeira,
  regeneração manual e incremental); contratos geral e RH sem os blocos comuns
  (o update ganha a regra de "base editável"); Técnica recebe
  `TECHNICAL_SUMMARY_PROMPT` com seis seções de debrief técnico. Revisão de
  catálogo 3 limpa as sementes antigas de forma textual/idempotente e troca o
  técnico quando ainda está com o contrato geral (editado/vazio preservado).
- **Testes:** 673 no `cargo test --lib`; `clippy` limpo.

## 2026-10-23 — Debrief técnico: "em aberto" só com o que ficou sem resposta (BR-090, rev 4)

- **IDs:** BR-090 · **Refs:** [`tasks/18-task.md`](tasks/18-task.md)

- **Sintoma:** no resumo do ledger, "Open questions" marcava perguntas que ME
  respondeu (mas raso) como em aberto — exigência demais para o próprio
  usuário validar.
- **Ajuste:** "Open questions" fica só com o que ficou **sem resposta**; o que
  foi respondido, mas podia ser mais fundo, vira o sub-bloco **What to detail
  better** dentro de "Overall assessment of my performance". O item "resposta
  vaga" saiu de "Gaps" (era duplicado).
- **Migração:** revisão de catálogo 4 troca os três trechos textualmente;
  idempotente, edição do usuário preservada.
- **Testes:** 676 no `cargo test --lib`; `clippy` limpo.

## 2026-10-23 — Debrief técnico: autoria das perguntas em aberto (BR-090, rev 5)

- **IDs:** BR-090 · **Refs:** [`tasks/19-task.md`](tasks/19-task.md)

- **Sintoma:** no resumo do ledger, "Questões em aberto" listava perguntas de
  ME (fragmentos de ASR no canal do microfone) como se fossem do entrevistador.
- **Ajuste:** o parágrafo de "Open questions" fixa a autoria em THEY ("Never
  list questions ME asked"); tag decide, duvidosa resolve por conteúdo, linha
  inutilizável é descartada em vez de adivinhada.
- **Migração:** revisão de catálogo 5 troca o parágrafo textualmente; idempotente.
- **Testes:** 678 no `cargo test --lib`; `clippy` limpo.

## 2026-10-25 — Resumo do Padrão livre; Reunião e LeetCode com contrato próprio (BR-091, rev 6)

- **IDs:** BR-091 · **Refs:** [`tasks/20-task.md`](tasks/20-task.md)

- **Sintoma:** o resumo do agente **Padrão** saía sempre com a pauta de uma
  reunião de empresa (Overview, Decisões confirmadas, Metas, Métricas, Action
  Items, Questões em aberto), mesmo quando a conversa era uma aula, um
  atendimento, um brainstorm ou um papo. O LeetCode, que é código, também usava
  o contrato de reunião.
- **Mudança:** o contrato fixo vira o do agente **Reunião**
  (`SUMMARY_SYSTEM_PROMPT` → `MEETING_SUMMARY_PROMPT`, texto preservado); o
  **Padrão** ganha `DEFAULT_SUMMARY_PROMPT`, **livre e adaptativo** — detecta o
  tipo de conversa, abre com um parágrafo curto e escolhe 2–4 títulos `##`
  conforme o conteúdo, sem seção vazia. Os invariantes (idioma, não inventar,
  preservar números, distinguir decisão × proposta × hipótese × pergunta em
  aberto) continuam. **LeetCode** passa ao debrief técnico
  (`TECHNICAL_SUMMARY_PROMPT`), como a Entrevista Técnica.
  `effective_summary_prompt` vazio herda o contrato do **Padrão** (BR-079).
- **Migração:** revisão de catálogo **6** troca o `summary_prompt` do `leetcode`
  semeado só quando ainda é o contrato de reunião; edição/vazio preservado e
  Reunião intacta. O Padrão não precisa de migração (vive no core). Resumos já
  gerados mantêm o layout antigo até a regeneração manual (BR-076/BR-084).
  `SUMMARY_UPDATE_SYSTEM_PROMPT` não muda: é compartilhado com RH/Técnica e só
  preserva a organização existente.
- **Testes:** 701 no `cargo test --lib`; `clippy` e `fmt --check` limpos; 485 no
  `pnpm test`.

## 2026-10-28 — Resposta rápida independente da completa (BR-088, rev)

- **IDs:** BR-088 · **Issues:** ISS-062 · **Refs:** [`tasks/21-task.md`](tasks/21-task.md),
  [`prd.md`](prd.md) §13.1,
  [`architecture/assistants-panel.md`](architecture/assistants-panel.md) §3.7

- **Sintoma:** ao testar DeepSeek e Mercury, a **Resposta rápida** (resumida)
  aparecia só às vezes; a completa saía normalmente. O comportamento variava
  por modelo e por geração.
- **Causa raiz:** a rápida (não-streaming) competia com a completa
  (streaming). O primeiro delta da completa marcava `full_emitted` e o rascunho
  atrasado era descartado no core; a UI repetia o descarte com um `Set` de
  gerações completas. Não era contrato por provider — modelos mais rápidos
  simplesmente venciam a corrida. Nenhum aviso de falha do rascunho aparecia
  nos logs.
- **Correção:** removida a flag `full_emitted`; a `quick` sempre é publicada
  quando termina. A UI aceita `quick` atrasada da mesma geração e ignora apenas
  a de geração antiga (`generation_id` + `at_ms`), sem limpar a completa nem
  reabrir o aviso de completa pendente.
- **Testes:** `a_late_quick_draft_still_fills_the_quick_block` (Rust) e os dois
  casos novos em `useSuggestions.test.tsx`; 855 testes no `cargo test --lib`,
  858 com Laya, 527 no `pnpm test`; `clippy`/`tsc` limpos.
