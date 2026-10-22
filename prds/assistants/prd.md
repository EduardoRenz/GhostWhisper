# PRD — Painel de Assistentes (gestão de agentes nas Configurações)

- **Versão:** 1.3
- **Data:** 2026-10-21 · **Revisão:** 2026-10-22
- **Status:** **Implementada** (tasks 01–12 `CONCLUDED`) — revisão de aderência
  ao mockup e às camadas em 2026-10-22; fallback de prompt vazio (BR-079) e
  seletor global de agentes acima do notch (BR-080) e tooltips de ajuda nos campos (BR-081) em 2026-10-22
- **Relacionados:** [`../business-rules.md`](../business-rules.md),
  [`memory.md`](memory.md), [`issues.md`](issues.md),
  [`architecture/assistants-panel.md`](architecture/assistants-panel.md),
  [`tasks/_tasks.md`](tasks/_tasks.md), PRDs anteriores:
  [`../assistant/prd.md`](../assistant/prd.md) (agentes e janelas de sugestão),
  [`../ai-sources/prd.md`](../ai-sources/prd.md) (casca das Configurações)

---

## 1. Objetivo

Uma aba **"Assistentes"** nas Configurações — um painel de gestão dos agentes de
IA, desenhado a partir do mockup do usuário (2026-10-21):

1. **O agente Padrão no topo**, em card informativo, com a badge "Primário".
   Ele **nunca** é editado nem apagado.
2. **Os agentes configurados abaixo**, em cards: ícone, nome, badge "Ativo" e
   as ações **Editar** (que abre um **modal**) e **Excluir**, mais duplicar — e a
   leitura direta dos **prompts** do agente (sugestão e resumo).
3. **Um modal único** para **criar** e **editar** agente, com os mesmos campos
   nos dois casos: nome, ícone, cor, prompt de sugestão e prompt de resumo.
4. **Prompt de resumo por agente**: campo novo, persistido agora, consumido por
   uma feature futura.
5. Os agentes que hoje são **de fábrica** (Reunião, Entrevista Técnica,
   LeetCode, Entrevista com RH) passam a ser **do usuário**: editáveis e
   excluíveis de vez.

Além disso, a **barra de abas** das Configurações passa do sublinhado para o
estilo **pílula** do mockup.

## 2. Problema

A aba "Assistente" de hoje é um formulário, não um painel:

- O agente aparece como uma **linha de tabela** (`.settings__mode`) com nome,
  "Editar", "Duplicar", "Excluir". A identidade (ícone/cor, BR-037) existe, mas
  só nessa linha compacta.
- **Os prompts não aparecem em lugar nenhum.** O usuário não consegue ver o que
  o agente faz — tem que abrir o editor para descobrir.
- **Editar abre um bloco inline** (`.settings__editor`) que empurra a lista
  inteira. Não há modal no app: os "modais" existentes são janelas Tauri
  separadas ([`../assistant/architecture/assistant-windows.md`](../assistant/architecture/assistant-windows.md)).
- Os agentes de fábrica são **imutáveis**: só duplicáveis, nunca ajustáveis e
  nunca dispensáveis. O usuário que não quer "Reunião" na lista não tem como
  tirá-la.
- **O agente Padrão não é distinguível**: ele é mais um item do `<select>`, ao
  lado dos outros, sem nenhuma marca de que é o padrão do sistema.
- **Não existe prompt de resumo por agente.** O `SummaryService` usa um contrato
  global (`SUMMARY_SYSTEM_PROMPT`), então a identidade do agente não alcança a
  tela de Resumo — e não há onde preparar isso.
- A aba ainda mistura três assuntos: **agentes**, **cadências** (resumo e
  sugestões automáticas) e **perguntas prontas**.

## 3. Público-alvo e casos de uso

Mesmas personas do MVP ([`../mvp/prd.md`](../mvp/prd.md) §3), com foco em:

- **CU-01 — Ver quem existe:** abrir a aba e reconhecer de relance os agentes
  configurados, cada um com seu ícone, sua cor e o que ele promete.
- **CU-02 — Ajustar um agente de fábrica:** editar "Reunião" para incluir o
  vocabulário da empresa, sem duplicar antes.
- **CU-03 — Dispensar um agente:** apagar de vez o "LeetCode" se a empresa não
  faz entrevistas.
- **CU-04 — Criar do zero:** "Criar Novo Assistente de IA" com nome, ícone,
  cor e os dois prompts.
- **CU-05 — Ver quem está falando:** a badge "Ativo" diz qual agente está
  respondendo agora.
- **CU-06 — Preparar o resumo por agente:** gravar o prompt de resumo do agente
  agora, para a feature de resumo por agente consumir depois.
- **CU-07 — Não perder o Padrão:** o agente Padrão continua ali, sempre, como
  fallback seguro — ninguém o apaga por engano.

## 4. Escopo

### 4.1 Entregue

- Aba **"Assistentes"** (renomeia "Assistente"), com o painel do mockup:
  card do Padrão, seção "AGENTES CONFIGURADOS (n)", cards dos agentes e o CTA
  "Criar Novo Assistente de IA".
- Card do agente **Padrão**: informativo, com a badge "Primário", sem ações de
  edição/exclusão.
- Cards dos agentes configurados: tile de ícone na cor do agente, nome, badge
  "Ativo" (informativa), ações **Editar** / duplicar / **Excluir** e a leitura
  dos prompts (**Prompt de Sugestão** e **Prompt de Resumo**).
- **Modal único** de criação/edição (`Dialog` in-page), com os campos: nome,
  ícone, cor, prompt de sugestão, prompt de resumo e um bloco "avançado" com
  categoria, prompt de sistema e contexto fixo.
- **Prompt de resumo por agente** (`summary_prompt`) no modelo, persistido e
  editável — **não consumido** pelo `SummaryService` nesta entrega.
- **Agentes de fábrica viram do usuário**: semeadura única dos quatro agentes
  (Reunião, Entrevista Técnica, LeetCode, Entrevista com RH) na lista do usuário,
  com edição definitiva e exclusão definitiva.
- **Move** de conteúdo: as **cadências** (resumo/sugestões automáticas +
  intervalos) e as **perguntas prontas** saem da aba "Assistentes" e viram uma
  **seção nova na aba "Geral & HUD"**.
- **Barra de abas em pílulas** (mockup): aba ativa com borda e texto ciano.
- Ícones novos do projeto: `PencilIcon`, `StarIcon`, `PlusIcon` (BR-008).

### 4.2 Fora do escopo (PRD futura)

- **Consumir** o `summary_prompt` na geração de resumo (feature à parte: o
  `SummaryService` continua com o contrato global).
- **Selecionar o agente ativo** pelo painel — a troca continua na tela de
  sugestões (`AgentSelector`), único lugar implementado hoje.
- **"Restaurar padrão"** de um agente de fábrica **individual** (voltar ao
  contrato original daquele agente) — o "Restaurar Padrões" global continua
  existindo e traz todos os agentes de fábrica de volta (§8.6).
- **Vincular fonte de IA por agente** (BR-048 já tem o modelo; a UI é outra
  PRD).
- **Badge "Padrão de fábrica"** nos cards — a origem do agente não é
  rastreada; dá para adicionar depois.
- Reorganizar a barra de abas na taxonomia completa do mockup (Áudio &
  Microfone, Visão de Tela, Privacidade como abas próprias).
- Badge de versão ("GhostWhisper 2.4") no cabeçalho.

## 5. O mockup — leitura e decisões

O mockup do usuário (2026-10-21) é **informativo**: os nomes e os textos dele
são de exemplo. O que vale é a **estrutura** e o **estilo**.

| Elemento do mockup | O que é no app | Decisão |
|---|---|---|
| Aba "Assistentes" com pill ativa | `TABS[3] = "Assistente"` (sublinhado) | Renomear para **"Assistentes"**; barra de abas em **pílulas** |
| Card "★ Assistente Padrão" + badge "Primário" | agente `default` (BR-059), hoje só um item de `<select>` | Card **informativo no topo**, sem editar/excluir |
| "AGENTES CONFIGURADOS (3)" | lista `.settings__modes` de `custom_assistant_modes` | Título de seção com contagem real |
| Card com tile de ícone, nome e badge "Ativo" | linha `.settings__mode` | Virar **card**; "Ativo" = `assistant_mode_id` (informativa) |
| Botões "Editar" / duplicar / excluir | botões de texto | "Editar" com lápis; duplicar/excluir como botões de ícone |
| Painel "⚡ Prompt de Sugestão" + "📄 Prompt de Resumo" | `suggestion_prompt` (existe) e `summary_prompt` (novo) | Mostrar os **dois** prompts no card, com o texto truncado. A pill "Markdown AST" do mockup foi **removida** na revisão (2026-10-22) |
| Cards com **níveis de detalhe diferentes** (completo → compacto → só descrição) | — | Os três níveis são **ilustrativos**: todo card usa o formato completo (ícone + título), com o texto truncado. Não há colapsar por card nesta entrega |
| Card "Reunião" com descrição em texto livre | — | **Fora**: o card mostra os prompts, não uma descrição livre (o modelo não tem esse campo) |
| "Criar Novo Assistente de IA" | botão "Novo agente" | CTA no rodapé da lista, abre o **mesmo modal** |
| Rodapé "Salvar Alterações" / "Restaurar Padrões" | já existe | Mantido (salvamento global, BR-047) |

### 5.1 Decisões do usuário (2026-10-21)

1. **Agentes de fábrica:** tirando o `default` (protegido), **todo o resto vem
   "Padrão de fábrica"** e o usuário pode **editar e excluir permanentemente**.
2. **Conteúdo antigo da aba:** cadências e perguntas prontas vão para uma
   **nova seção na aba "Geral & HUD"**.
3. **Agente ativo:** **por hora não é selecionável no painel** — o usuário
   segue selecionando na tela de sugestões (único lugar implementado). A badge
   "Ativo" é informativa.
4. **Casca:** **restilizar a barra de abas em pílulas** (o badge de versão fica
   de fora).

Decisões de desenho que completam as do usuário:

5. **Badge "Ativo" no card do Padrão:** quando `assistant_mode_id = "default"`
   (instalação nova), o card do topo mostra **"Primário" + "Ativo"** — a badge
   "Primário" é fixa, a "Ativo" é a verdade do momento.
6. **Prompt de resumo = campo nomeado**, não um mapa `tela → prompt`: só há
   duas telas hoje (sugestão e resumo); generalizar agora criaria estrutura sem
   uso. Quando houver mais telas, o modelo evolui.
7. **Os prompts aparecem como estão:** os contratos atuais são escritos em
   inglês (o modelo responde no idioma da conversa), então o card mostra o
   texto real do agente — não uma tradução. O contrato comum de leitura da
   transcrição (BR-082) fica **fora** do campo: é aplicado em runtime a todos
   os agentes.

## 6. Regras de produto

As decisões acima viram regras canônicas em
[`../business-rules.md`](../business-rules.md):

| ID | Título |
|---|---|
| BR-074 | Painel "Assistentes": o Padrão no topo, os agentes configurados abaixo |
| BR-075 | Os agentes de fábrica viram agentes do usuário (semeadura única) |
| BR-076 | Prompt de resumo por agente (persistido, ainda não consumido) |
| BR-077 | Modal único de agente (criar e editar) |
| BR-078 | Barra de abas das Configurações em pílulas |
| BR-079 | Prompt vazio herda o do agente Padrão |
| BR-080 | Seletor de agentes global, flutuando acima do notch |
| BR-081 | Ajuda nos campos de prompt (tooltip) |
| BR-082 | O contrato de leitura da transcrição é comum a todos os agentes |
| BR-083 | Entrevista Técnica funde System Design; novo Entrevista com RH |

## 7. Modelo de dados

Detalhe de implementação:
[`architecture/assistants-panel.md`](architecture/assistants-panel.md) §3.

### 7.1 `AssistantMode` ganha `summary_prompt`

```rust
/// Prompt do resumo (persistido; o consumo é de uma feature futura — BR-076).
#[serde(default)]
pub summary_prompt: String,
```

- `#[serde(default)]` ⇒ `settings.json` antigo continua válido.
- Limite de `MODE_PROMPT_MAX_CHARS` (16 384), como os demais prompts.
- Entra na regra de "ao menos um prompt preenchido" da validação.
- Espelho em `src/core/domain/assistant.ts`.

### 7.2 A lista do usuário passa a ser a fonte da verdade

| Conceito | Onde | Efeito |
|---|---|---|
| Agente **protegido** | `application::prompts::assistant::default_mode()` (id `default`) | Sempre existe, nunca editável, nunca excluível |
| Agente **de fábrica** | `application::prompts::assistant::factory_modes()` (4: Reunião, Entrevista Técnica, LeetCode, Entrevista com RH) | **Semente** da lista do usuário (BR-075) |
| Agente **do usuário** | `settings.custom_assistant_modes` | A lista "AGENTES CONFIGURADOS" |

Resolução (`resolve_mode`): `default` → agente protegido; qualquer outro id →
só na lista do usuário. Agente de fábrica excluído **não volta** (a exclusão é
definitiva).

### 7.3 Semeadura e revisões do catálogo

`Settings.assistant_modes_seeded: bool` (`serde(default)` = `false`) marca que os
agentes de fábrica já foram materializados. Uma migração no boot:

1. se `!assistant_modes_seeded` → acrescenta os agentes de fábrica que ainda
   não existem (por id) em `custom_assistant_modes`;
2. marca `assistant_modes_seeded = true` e salva;
3. se `assistant_catalog_revision < ASSISTANT_CATALOG_REVISION` → aplica a
   revisão do catálogo **uma vez** (BR-083): funde/remove/adiciona agentes,
   preserva `context`/`source_id` do agente técnico e o resto da lista, e grava
   a revisão atual. Depois dela, edições e exclusões do usuário são respeitadas.

A semeadura roda também em **instalação nova**: o `Settings::default()` do
domínio nasce **puro** (lista vazia, marca desligada, revisão 0), porque o
catálogo de fábrica vive na camada de aplicação — o `domain` não pode depender de
`application` (R1). Quem materializa é a aplicação, no boot e no
"Restaurar Padrões" (§8.6).

### 7.4 Prompt vazio herda o do agente Padrão (BR-079)

Em **qualquer** prompt do agente — `system_prompt`, `suggestion_prompt` e
`summary_prompt` —, um valor **vazio depois do `trim`** usa o prompt
correspondente do **agente Padrão**:

| Campo | Se vazio, usa |
|---|---|
| `suggestion_prompt` | `DEFAULT_SUGGESTION_PROMPT` (contrato do Padrão) |
| `system_prompt` | `DEFAULT_SYSTEM_PROMPT` (critérios do Padrão) |
| `summary_prompt` | `SUMMARY_SYSTEM_PROMPT` (contrato de resumo do Padrão) |

- Vale no **runtime** (`application::prompts::assistant::effective_*_prompt`),
  não só na criação: um agente antigo com campo em branco também herda.
- O `context` **não** entra na regra: é contexto fixo, não prompt; vazio
  continua não acrescentando nada.
- A validação do agente continua exigindo **ao menos um** prompt/contexto
  preenchido — a herança é por campo, não um atalho para criar um agente vazio.
- O card da aba "Assistentes" mostra o prompt **efetivo** e, quando ele veio do
  Padrão, a nota "Herdado do agente Padrão".
- O prompt de resumo continua **não consumido** (BR-076); o helper já resolve o
  vazio para quando a feature de resumo por agente entrar.

## 8. Fluxos

### 8.1 Abrir a aba

`AssistantsSettings` desenha, nesta ordem:

1. **Card do agente Padrão** (protegido) — nome, badge "Primário" e a frase de
   papel ("Padrão do sistema, tenta automaticamente se adaptar e entender o
   contexto"). Se ele for o agente em uso, leva também a badge "Ativo".
2. **"AGENTES CONFIGURADOS (n)"** — `n` = tamanho de `custom_assistant_modes`.
3. **Um card por agente** da lista do usuário, na ordem em que estão.
4. **CTA "Criar Novo Assistente de IA"**.

O painel **não** escolhe o agente ativo: a badge "Ativo" compara o id do card
com `settings.assistant_mode_id`.

O card do Padrão usa o catálogo do core (`assistant_modes_list`) e, se ele não
carregar (fora do Tauri, testes), cai num fallback estático com as constantes
do domínio (`DEFAULT_MODE_NAME`, `DEFAULT_AGENT_ICON`, `DEFAULT_AGENT_COLOR`).

### 8.2 Criar

"Criar Novo Assistente de IA" abre o modal em modo de criação (formulário
vazio, id novo `custom-<uuid>`). "Salvar" valida (nome obrigatório + ao menos
um prompt) e acrescenta o agente à lista.

### 8.3 Editar

"Editar" abre o **mesmo** modal, com os valores do agente. "Salvar" substitui o
agente na lista (mesmo id).

### 8.4 Duplicar

Duplicar abre o modal em modo de criação, **pré-preenchido** com o agente
copiado (nome com sufixo "(cópia)", id novo).

### 8.5 Excluir

"Excluir" remove o agente da lista do usuário. Se ele for o agente ativo
(`assistant_mode_id`), a seleção volta para o agente Padrão (`default`). Não há
como excluir o agente Padrão — ele não aparece na lista.

### 8.6 Restaurar Padrões

"Restaurar Padrões" na janela de Configurações volta os agentes de fábrica. Como
o catálogo vive na camada de aplicação (não no `Settings::default()` do
domínio), o core expõe um default **materializado** (`settings_defaults`): o
default do domínio + os 4 agentes de fábrica + a marca ligada + a revisão do
catálogo. Ver
[`architecture/assistants-panel.md`](architecture/assistants-panel.md) §5.

## 9. Critérios de aceite do produto

- [ ] A aba se chama **"Assistentes"** e é a 4ª da barra (mockup).
- [ ] A barra de abas usa **pílulas**; a aba ativa tem borda e texto ciano.
- [ ] O **agente Padrão** aparece no topo, com a badge "Primário" e **sem**
      ações de edição/exclusão; leva também "Ativo" quando for o agente em uso.
- [ ] A seção "AGENTES CONFIGURADOS (n)" lista os agentes do usuário com a
      contagem real.
- [ ] Cada card mostra **ícone na cor do agente**, nome, badge "Ativo" quando
      for o agente em uso, o **Prompt de Sugestão** e o **Prompt de Resumo**,
      no mesmo formato (sem níveis de detalhe por card).
- [ ] "Editar" abre um **modal** com nome, ícone, cor, prompt de sugestão e
      prompt de resumo; "Criar Novo Assistente de IA" abre o mesmo modal.
- [ ] O modal **salva** as alterações e o **rodapé "Salvar Alterações"** persiste
      (BR-047).
- [ ] O prompt de resumo **sobrevive** ao reinício (persiste em `settings.json`).
- [ ] Os agentes de fábrica podem ser **editados** e **excluídos de vez**.
- [ ] As **cadências** e as **perguntas prontas** aparecem na aba "Geral & HUD".
- [ ] A tela de sugestões continua trocando o agente ativo (BR-037).
- [ ] O catálogo de fábrica tem **Entrevista Técnica** (código + system design)
      e **Entrevista com RH**; não existe mais o agente **System Design**
      separado (BR-083).
- [ ] O prompt de **Entrevista com RH** cobre perguntas em aberto, aprofundamento
      de respostas vagas e STAR quando cabível (BR-083).

## 10. Riscos e limites

| Risco | Mitigação |
|---|---|
| Usuário exclui o agente que estava ativo | O `assistant_mode_id` volta para `default` na mesma edição |
| `settings.json` antigo sem `summary_prompt` | `serde(default)` + `sanitized()` (o app já normaliza na carga) |
| Usuário perde os contratos de fábrica ao excluir | É o comportamento pedido ("exclusão definitiva"); o "Restaurar Padrões" traz de volta |
| A semeadura duplicar agentes | A migração confere por **id** e só acrescenta o que falta |
| `assistant_mode_id` apontando para um agente semeado | O id é preservado na semeadura ⇒ a seleção continua válida |
| `domain` depender de `application` para semear o default | O `Settings::default()` continua puro; quem materializa é a aplicação (§7.3) |
| Os prompts aparecerem em inglês na UI | É o texto real dos contratos hoje (decisão 7 do §5.1) |

## 11. Plano de tasks

| # | Resumo | Status |
|---|--------|--------|
| 01 | Domínio do agente: `summary_prompt` e catálogo protegido | CONCLUDED |
| 02 | Settings: semeadura dos agentes de fábrica e resolução | CONCLUDED |
| 03 | IPC: catálogo efetivo e defaults do core | CONCLUDED |
| 04 | Frontend: espelho do domínio, `Dialog` e ícones novos | CONCLUDED |
| 05 | Painel "Assistentes" (cards do mockup) | CONCLUDED |
| 06 | Modal de agente (criar/editar) | CONCLUDED |
| 07 | "Geral & HUD": cadências e perguntas prontas | CONCLUDED |
| 08 | Casca: barra de abas em pílulas | CONCLUDED |
| 09 | Regressão, testes e fechamento | CONCLUDED |
| 10 | Fallback de prompt vazio: herda o do Padrão (BR-079) | CONCLUDED |
| 11 | Seletor global de agentes acima do notch (BR-080) | CONCLUDED |
| 12 | Ajuda nos campos de prompt (BR-081) | CONCLUDED |
| 13 | Contrato comum de leitura da transcrição (BR-082) | CONCLUDED |
| 14 | Entrevista Técnica funde System Design; novo Entrevista com RH (BR-083) | CONCLUDED |

Detalhes, entregáveis e critérios por task: [`tasks/_tasks.md`](tasks/_tasks.md).

## 12. Seletor global de agentes acima do notch (BR-080)

O agente ativo deixa de ser escolhido na janela de Sugestões e ganha um
**seletor flutuante acima da barra do notch**, sempre visível com o overlay.

### 12.1 O que muda

- **Entra** o seletor global (`AgentSwitcher`) na faixa superior do overlay:
  pílula compacta com **ícone na cor do agente + nome + divisor + "Alternar
  agente" + chevron**; o popover lista o catálogo efetivo (Padrão + agentes do
  usuário) e marca o ativo.
- **Sai** o seletor da janela de Sugestões. A janela continua consumindo o
  agente ativo (`settings.assistant_mode_id`) para gerar a sugestão e para o
  escopo do modelo (`agent:<id>`), mas não o escolhe mais.
- A troca é **global e em tempo real**: grava `assistant_mode_id` no core e
  emite `settings_changed`; Sugestões, Configurações (badge "Ativo") e o
  próprio seletor refletem a escolha sem reabrir nada.

### 12.2 Geometria

O overlay não tinha espaço acima da barra: a janela é ancorada no topo útil do
monitor e a barra ocupava o primeiro pixel. Para o seletor flutuar acima:

- a janela reserva uma faixa fixa de **34 px** (`AGENT_SELECTOR_BAND`) acima da
  densidade atual; a barra **desce** 34 px e a janela cresce para baixo (o topo
  continua ancorado em `y = work_area.y`);
- a barra deixa de encostar no topo da tela — é o preço de ter o seletor acima
  dela, como no mockup;
- a largura continua sendo a da pilha (seletor + barra): o seletor é medido
  junto, então nunca é cortado quando a barra está estreita;
- o menu do seletor abre **abaixo** da pílula e reserva altura extra (240 px),
  como o menu de canais (180 px); quando os dois estão abertos, vale o maior.

### 12.3 Interação

- O seletor é `[data-interactive]` como o resto da barra; o overlay continua
  click-through fora dele.
- Teclado: setas + `Home`/`End` navegam, `Enter` escolhe, `Esc` fecha; clique
  fora fecha.
- Fora do catálogo (boot/janela do navegador), o nome cai no Padrão
  (`PROTECTED_MODE_NAME`), então a pílula nunca aparece vazia.

### 12.4 Fora do escopo

- Arrastar o seletor ou reposicioná-lo independentemente do notch.
- Um menu de contexto (editar o agente a partir do notch).
- Mostrar os prompts do agente no seletor.

## 13. Os campos de prompt do agente (e a ajuda na UI)

Os prompts do agente não são todos usados do mesmo jeito. A tabela é a
referência canônica do que cada campo faz **hoje**.

**Contrato comum de leitura (BR-082).** Todo prompt de sistema de sugestão —
de qualquer agente, inclusive personalizado — começa com o
`TRANSCRIPT_CONTRACT`: o texto é uma transcrição de fala, `ME` é o usuário
(microfone), `THEY` é o outro lado (áudio do sistema), com erros de
reconhecimento e atribuição imperfeita. Ele **não** é um campo do agente: é
aplicado em runtime e não aparece no card/modal. O fast-first recebe o mesmo
contrato; a Busca explica ME/THEY no bloco de regras dela (sem a prioridade do
THEY, que é do contexto ao vivo).

| Campo | O que é | Onde é usado hoje | Se ficar vazio |
|---|---|---|---|
| `suggestion_prompt` — **Prompt de Sugestão** | Contrato da sugestão ao vivo (só a cena) | Base do prompt de sistema da sugestão (passe completo, `suggestion_system_prompt`), **depois** do contrato comum de leitura (BR-082) | Usa o prompt do **Padrão** (BR-079) |
| `summary_prompt` — **Prompt de Resumo** | Contrato do resumo deste agente | **Não consumido**: o `SummaryService` ainda usa o contrato global (BR-076) | Usa o do **Padrão** (BR-079), quando a feature entrar |
| `system_prompt` — **Prompt de sistema (critérios de domínio)** | Papel, tom e critérios do domínio | Bloco `## Domain criteria` do prompt da sugestão; **não** é usado pelo resumo nem pela Busca | Usa o do **Padrão** (BR-079) |
| `context` — **Contexto fixo do agente** | Fatos estáveis (produto, empresa, vocabulário) | Bloco `## Agent context` nos **dois** passes da sugestão (rápido e completo) | Nenhum contexto extra |
| `category` — Categoria | Rótulo de organização | Nenhum: é metadado (validado, ≤ 60 caracteres) | `custom` |
| `icon` / `color` | Identidade visual | Seletor, card, badge e snapshot da sugestão (BR-037) | `sparkle` / `slate` |
| `source_id` | Fonte de IA vinculada | Modelo pronto, sem UI (BR-048) | Fonte primária |

O que **não** é usado pelo resumo hoje: nenhum campo do agente. O
`SummaryService` usa `SUMMARY_SYSTEM_PROMPT` + `SUMMARY_UPDATE_SYSTEM_PROMPT`
(globais) e os construtores de prompt de usuário com a transcrição.

### 13.1 Como o prompt da sugestão é montado

Ordem do `suggestion_system_prompt`:

1. `TRANSCRIPT_CONTRACT` — contrato comum de leitura (BR-082);
2. `suggestion_prompt` (ou o do Padrão, BR-079);
3. `NEVER_ASK_BACK_RULE` (BR-060);
4. `PURE_CODE_RULE` (BR-061);
5. `## Domain criteria` — `system_prompt` (ou o do Padrão);
6. `## Agent context` — `context`, quando preenchido;
7. `## Language`;
8. `## Safety` (transcrição é conteúdo não confiável).

O **fast-first** (primeira bolha curta) começa com o mesmo
`TRANSCRIPT_CONTRACT` e depois usa um contrato fixo, `Agent: <nome>`, o
`context` (se houver), idioma e segurança. Ele **não** usa `suggestion_prompt`
nem `system_prompt`.

### 13.2 "Campos avançados" não é outro motor

O bloco "Campos avançados" do modal é só um `<details>`: agrupa os campos
menos usados (Categoria, Prompt de sistema e Contexto fixo). O comportamento é
exatamente o mesmo de antes de o painel existir — antes, os três apareciam
soltos no editor inline ao lado do Prompt de Sugestão.

### 13.3 Ajuda na UI (BR-081)

Cada prompt (e a Categoria) tem um tooltip com o texto da tabela acima, no
modal de edição; os painéis do card também têm a ajuda dos dois prompts.
O Prompt de Resumo diz explicitamente que ainda não é consumido. O tooltip do
Prompt de Sugestão avisa que o contrato comum de leitura (BR-082) é aplicado
automaticamente — o texto do campo não precisa repeti-lo.
