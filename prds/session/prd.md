# PRD — Sessão: o diário de perguntas (aba Sessão + feature Busca)

- **Versão:** 0.1
- **Data:** 2026-10-19
- **Status:** Implementado (tasks 01–10 `CONCLUDED`)
- **Relacionados:** [`../business-rules.md`](../business-rules.md),
  [`memory.md`](memory.md), [`issues.md`](issues.md),
  [`architecture/session-journal.md`](architecture/session-journal.md),
  [`tasks/_tasks.md`](tasks/_tasks.md), PRDs anteriores:
  [`../mvp/prd.md`](../mvp/prd.md), [`../chat/prd.md`](../chat/prd.md),
  [`../notch/prd.md`](../notch/prd.md), [`../assistant/prd.md`](../assistant/prd.md),
  [`../suggestions/prd.md`](../suggestions/prd.md),
  [`../ai-sources/prd.md`](../ai-sources/prd.md)

---

## 1. Objetivo

Dar ao app uma **aba Sessão** que funciona como o **diário de tudo que o
usuário perguntou** durante a sessão, e a **feature "Busca"** que produz esse
diário: **perguntar** à IA, **tirar print de tela** e anexar à pergunta, e
**salvar perguntas prontas** para repetir com um clique.

Em uma frase: *onde a reunião está sendo gravada, o usuário também pode
**perguntar** — e tudo que ele perguntou fica registrado, legível e
pesquisável depois.*

A forma é a do **Perssua** (BR-015): lá o "Ask" é uma barra flutuante com
print e sugestões prontas, e a sessão é o agregado do diário (perguntas,
respostas, falas e resumo) pesquisável por FTS. Aqui a transposição é de
**tecnologia** (Electron/JS → Tauri/Rust) e de **escopo** (sem cloud, sem
MCP, sem login), nunca de comportamento.

## 2. Problema

- **Pergunta não é evento de primeira classe.** Hoje o usuário só recebe
  *sugestões* da IA (BR-024/BR-028) — a IA fala **quando a transcrição
 mudou**, no ritmo dela. Não existe um caminho para o usuário **perguntar**
  algo: "o que eu acabei de dizer?", "qual o nome daquele cliente?", "o que
  eu posso responder agora?". Essas perguntas são exatamente o que o
  Perssua atende com o "Ask".
- **O print de tela é o insumo que falta.** Muita dúvida de reunião é
  visual: um slide, um erro na tela, um gráfico. O Perssua captura a tela
  (`capture-screenshot`) e anexa à pergunta; o GhostWhisper não tem nada
  disso — nem porta, nem adaptador, nem suporte a imagem nos providers (o
  `CompletionRequest` só carrega texto).
- **Nada do que foi perguntado fica.** As sugestões fixadas vão para
  `messages` (BR-020), mas **perguntas** não existem. E não há como
  revisitá-las: o `messages_fts` indexa sugestões, não perguntas.
- **O slot da aba já está reservado.** BR-033 decided, ao unificar o painel,
  que *"a aba **'Sessão'** fica reservada para PRD futura"* — este é o
  fill-in daquele slot.

## 3. Público-alvo e casos de uso

Mesmas personas do MVP ([`../mvp/prd.md`](../mvp/prd.md) §3). Casos de uso
desta PRD:

- **CU-01 — Perguntar durante a reunião:** o usuário quer entender algo que
  acabou de ser dito (ou que está na tela) e pergunta; a resposta chega
  escrita, com o contexto da conversa até ali.
- **CU-02 — Perguntar sobre a tela:** o usuário printa a tela (ou cola uma
  imagem) e pergunta "o que está errado aqui?".
- **CU-03 — Repetir uma pergunta que deu certo:** o usuário salvou "quais são
  os action items?" e quer rodá-la de novo na próxima sessão.
- **CU-04 — Reler o que perguntou:** no fim do dia, o usuário quer saber "o
  que eu perguntei naquela reunião?".
- **CU-05 — Achar uma pergunta antiga:** o usuário procura por palavra-chave
  e acha a pergunta (e a resposta) de semanas atrás.

## 4. Histórias de usuário e critérios de aceite

### US-00 — Lupa no notch e barra de pesquisa no topo

Como usuário, quero **uma lupa no notch** (sempre disponível, com ou sem
transcrição) que abre uma **barra de pesquisa** discreta no topo.

**Critérios de aceite**

- [ ] O **notch** (barra de ações) ganha, ao lado da marca, um botão de
      **lupa** em cor primária (cyan) com o atalho em badge (**Ctrl+K**).
- [ ] A lupa existe **nos dois estados** — em repouso (sem sessão) e com
      transcrição gravando. **Não** depende de haver transcrição.
- [ ] Clicar na lupa (ou **Ctrl+K**) abre a **barra de pesquisa** (janela
      própria, no **topo**, centralizada): lupa + campo + `Ctrl K` + divisor +
      **Capturar Tela** (`Alt C`) — o layout da referência do Perssua.
- [ ] A barra de pesquisa **desaparece** depois do envio, e o painel abre na
      aba **Sessão** com a resposta em streaming.
- [ ] A pergunta **não exige** transcrição: sem sessão de áudio, o contexto é
      vazio e a resposta considera só a pergunta (e o print, se houver).
- [ ] **Sempre existe uma sessão** (BR-019): sem sessão aberta, a pergunta
      cria uma **pausada** — ela aparece no Histórico e na pílula do notch (o
      tempo conta), e a transcrição fica **pausada** até o usuário mandar. A
      resposta **fica** no diário (o reload do fim da resposta não apaga).
- [ ] `Esc` fecha a barra sem perguntar.
- [ ] A barra de pesquisa tem a **alça de arraste** (⋮⋮) à esquerda, como o
      notch.
- [ ] O notch mantém o **fundo** (pill opaca); o estado de repouso não perde a
      superfície.
- [ ] O notch mantém o comportamento e a posição de antes (arrastável,
      ancorado pela base).

### US-01 — Aba Sessão: o diário da sessão

Como usuário, quero uma **aba Sessão** no painel onde vejo, em ordem, tudo
que perguntei nesta sessão e o que a IA respondeu.

**Critérios de aceite**

- [ ] O painel da sessão (`chat`) passa a ter **três** abas: **Transcrição |
      Resumo | Sessão** (BR-033 reservou o slot; o `PanelTab` ganha
      `"session"`).
- [ ] A aba ativa é lembrada em `settings.panel_tab` (como as outras duas) e
      um valor legado/desconhecido cai em `transcription`.
- [ ] A aba Sessão mostra as **entradas do diário** com a **mais recente no
      topo** (revisão v5, 2026-10-20), separadas por uma linha — o equivalente
      ao `---` do Markdown. Cada entrada tem:
      - a **legenda de origem** no topo (`Pesquisa`, `Screenshot`,
        `Screenshot + Texto` ou `Pergunta pronta`) com o **tempo** (`Há 1 min`
        + a hora de parede `(10:44:12)`) e, à direita, o selo **Mais Recente**
        (na primeira) ou o modelo que respondeu;
      - a **pergunta** (`Q:`) — que **não aparece** quando o pedido foi só o
        print (`Contexto:` quando há print + texto);
      - a **miniatura do print** com a legenda sobre a imagem;
      - a **resposta** num bloco próprio ("GhostWhisper IA · Assistente"),
        **toda em Markdown** gerado pela IA.
- [ ] Uma entrada **sem resposta ainda** aparece com estado "pensando…" e
      recebe o texto **em streaming**, como a sugestão (BR-028).
- [ ] Uma entrada **com erro** mostra o erro e um botão **"Tentar de novo"**
      que reenvia a mesma pergunta (com o print, quando houver).
- [ ] Sem sessão ativa e sem diário, a aba mostra um estado vazio explicativo
      ("Nenhuma interação nesta sessão ainda. Pergunte pela lupa do notch").
- [ ] O rodapé mostra **quantas interações** a sessão tem, que os dados são
      locais (`Sync local · SQLite`) e o botão **Exportar MD**.
- [ ] **Encerrar a sessão limpa a aba**: o diário é da sessão ativa — encerrar
      (ou abrir outra) zera a lista, e a sessão nova começa vazia.
- [ ] O diário **sobrevive ao fechamento do app** e reaparece ao reabrir a
      sessão no Histórico.

### US-02 — Buscar: perguntar à IA com o contexto da sessão

Como usuário, quero **perguntar** qualquer coisa e receber a resposta na
mesma aba.

**Critérios de aceite**

- [ ] Perguntar acontece na **barra de pesquisa** (US-00), não num campo
      dentro da aba (revisão v5, 2026-10-20: o mockup do Perssua mostra a aba
      como diário puro). Ali: `Enter` envia, `Alt+C` captura a tela, as
      **perguntas prontas** aparecem como chips em repouso e o texto é
      opcional quando há print.
- [ ] A pergunta é respondida pela **fonte de IA da rota da capacidade
      `Search`** (BR-048) — ou, sem rota, pela fonte primária.
- [ ] A pergunta leva o **contexto da sessão**: as falas finalizadas até
      aquele instante (mesma janela e mesmos tetos das sugestões — BR-024),
      para que a resposta "saiba" da conversa.
- [ ] A resposta é **Markdown**, renderizada pelo `MarkdownView` (BR-025).
- [ ] **Uma pergunta por vez**: um novo envio durante uma resposta em curso
      **não** duplica texto (paridade com o single-flight de BR-042).
- [ ] O envio é **cancelável** enquanto a resposta não chegou (botão
      "Cancelar"); a entrada fica com status "cancelada" e pode ser
      reenviada.
- [ ] Falha do provider mostra o erro **na entrada** (não em um toast
      genérico) e a fonte seguinte da cadeia é tentada (BR-045).

### US-03 — Anexar print de tela

Como usuário, quero **printar a tela** e anexar à pergunta, para a IA "ver"
o que eu vejo.

**Critérios de aceite**

- [ ] Há um botão de **print** (ícone de câmera) ao lado do campo de
      pergunta; ao clicar, a tela é capturada e a **miniatura** aparece
      anexada ao campo.
- [ ] A captura usa a **porta `ScreenCapture`**; o adaptador Linux é **X11**
      (`x11rb`, `GetImage` na root window) — ver BR-055.
- [ ] A imagem é reescalada para **≤1280 px** no lado maior e recomprimida em
      **JPEG q0.86** (paridade com o preset `balanced` do Perssua) antes de ir
      ao provider.
- [ ] **Capturar não envia**: o botão **Capturar Tela** (ou `Alt+C`) apenas
      **anexa** o print. A prévia aparece no **canto superior direito** da
      tela, com um botão de **remover** (como no Perssua).
- [ ] O envio é **sempre explícito**: o usuário confirma no botão **Perguntar**
      (ou `Enter`). Sem texto e com print, o core aplica um prompt padrão
      ("Analise a imagem da tela…") — a IA decide o que fazer com a imagem.
- [ ] **Em nenhuma tela** o texto é obrigatório quando há print anexado (barra
      de pesquisa **e** composer da aba Sessão).
- [ ] Depois do print, **todas as janelas que estavam ativas voltam** (a barra
      de pesquisa inclusive, com o foco) — sem depender de sorte com o WM.
- [ ] **Uma imagem por pergunta** (paridade com o limite do Perssua): pedir
      uma segunda mostra "Só uma imagem por pergunta".
- [ ] A miniatura pode ser **removida** antes do envio (botão `×`).
- [ ] **Colar imagem** da área de transferência funciona como alternativa
      (`Ctrl+V`/`Cmd+V` com foco no campo) — é o caminho que funciona em
      Wayland.
- [ ] Em Wayland (sem display X11), o print dá **erro explícito** ("Captura
      de tela indisponível neste ambiente — cole a imagem com Ctrl+V") e
      **nunca** falha silencioso.
- [ ] As janelas do próprio app são **escondidas durante a captura** (o
      print não sai com o HUD em cima) e restauradas depois — mesmo cuidado do
      Perssua (`protectPerssuaWindowsForCapture`).
- [ ] O `hide()` é **assíncrono**: a captura espera o compositor tirar as
      janelas da tela (settle + confirmação do `unmap`) antes do `GetImage` —
      sem isso o print sai com o app. Regressão observada no teste de
      2026-10-19.

### US-04 — Perguntas prontas

Como usuário, quero **salvar** as perguntas que eu uso sempre e **clicar**
nelas para rodá-las.

**Critérios de aceite**

- [ ] Abaixo do campo há **chips** com as perguntas salvas; clicar em um chip
      **preenche e envia** a pergunta (paridade com o "Prompt Suggestions" do
      Perssua).
- [ ] Há um botão para **salvar a pergunta atual** como pronta (ícone de
      marcador) e um **editor** para remover/editar as salvas.
- [ ] As perguntas prontas ficam nas **settings** (`settings.saved_questions`),
      não no banco — são configuração do usuário (BR-054).
- [ ] Limites: no máximo **20** perguntas, **200 caracteres** cada; o que
      exceder é recusado com aviso, não truncado em silêncio.
- [ ] Cinco perguntas iniciais semeadas (as quatro do Perssua + uma **ação**):
      *"Resumir tudo até agora"*, *"Listar as ações"*, *"Listar as principais
      ideias"*, *"O que posso falar?"*, *"Responda a última pergunta
      identificada"* (um pedido — a IA responde à última pergunta que
      identificou na conversa/tela).
- [ ] A pergunta salva registra a **origem** (`Pergunta pronta`) na entrada do
      diário.

### US-05 — Botão "Perguntar" no notch

Como usuário, quero **um botão só** no notch para chegar na Busca, sem
descobrir onde está a aba.

**Critérios de aceite**

- [ ] No switcher do notch **ativo**, o botão **"Perguntar"** (ícone de **lupa
     **) fica **ao lado do botão "Resumo"** — entre "Resumo" e "Sugestões".
- [ ] O botão **só existe com sessão iniciada** (o switcher do notch já é
      renderizado apenas no estado ativo — BR-032).
- [ ] Clicar abre/foca o painel (`chat`) **na aba Sessão**, com o campo de
      pergunta **pronto para digitar**.
- [ ] O botão tem `title` e `aria-label` (BR-032) e tem **drill-hover**
      (BR-036), como "Resumo" e "Sugestões".
- [ ] O ícone de lupa é **SVG do projeto** (BR-008), no mesmo estilo dos
      demais ícones do notch.

### US-06 — Pesquisar no diário

> **Revisão v5 (2026-10-20):** o campo de busca **saiu da aba** (o mockup do
> Perssua não o tem). O FTS (`journal_search`) e o escape (ISS-042) continuam
> no core, prontos para uma superfície futura; a aba mostra só o diário.

Como usuário, quero **procurar** uma pergunta que fiz.

**Critérios de aceite**

- [ ] A aba Sessão tem um campo de **busca** que filtra as entradas por
      palavra-chave, no **FTS5** (paridade com o `messages_fts` do
      Perssua).
- [ ] A busca Casa em **pergunta e resposta**, dentro da sessão atual.
- [ ] O texto digitado é **escapado** antes do `MATCH` (o Perssua passa a
      string crua e a consulta quebra com aspas/asteriscos — armadilha
      deliberadamente **não** copiada, ver ISS-042).
- [ ] A busca é **debounced** (180 ms, como o `AllSessionsPage` do Perssua).
- [ ] Entries sem resposta também são buscáveis (pela pergunta).

### US-07 — O diário no export

Como usuário, quero o diário **no export Markdown** da sessão.

**Critérios de aceite**

- [ ] O export Markdown da sessão inclui uma seção **"Perguntas"** com cada
      pergunta e a resposta, na ordem, com o instante.
- [ ] A seção respeita o contrato do export existente (BR-020): cabeçalho +
      corpo Markdown, gerado pelo core (nunca pela UI).
- [ ] Entradas com print registram que houve imagem (o binário **não** entra
      no Markdown).

### US-08 — Privacidade, limites e robustez

**Critérios de aceite**

- [ ] Nada sai do dispositivo além do provider que o usuário configurou
      (BR-002, BR-005): a imagem e o texto vão **só** para a fonte de IA
      escolhida.
- [ ] **Nenhuma telemetria** (BR-004): o uso da Busca não gera evento
      externo.
- [ ] Chave de API nunca aparece em log, erro ou no diário (BR-003).
- [ ] Excluir uma sessão exclui o diário dela e suas imagens (cascade).
- [ ] Pergunta e resposta **nunca** entram na aba **Transcrição** — o diário
      é da aba Sessão (BR-040: a Transcrição é só falas).
- [ ] Uma sessão encerrada **mantém** o diário; "Continuar" a partir do
      Histórico reabre a sessão e o diário volta junto.

## 5. Requisitos

### Funcionais (RF)

| ID | Requisito |
|---|---|
| RF-01 | Aba **Sessão** no painel da sessão (`chat`), ao lado de Transcrição e Resumo. |
| RF-02 | Composer da **Busca**: campo de pergunta, botão de print, anexos, chips de perguntas prontas, botão "Perguntar". |
| RF-03 | Caso de uso de **pergunta**: resolve a sessão, monta o contexto, chama a IA com streaming, persiste pergunta e resposta. |
| RF-04 | Diário persistido por sessão, com **FTS5** próprio para busca. |
| RF-05 | Porta e adaptador de **captura de tela** (X11) + **colar imagem**. |
| RF-06 | `CompletionRequest` **multimodal** (imagens) nos providers OpenAI-compatíveis e Gemini. |
| RF-07 | **Perguntas prontas** nas settings, com chips e editor. |
| RF-08 | Botão **"Perguntar"** (lupa) no switcher do notch, ao lado de "Resumo". |
| RF-09 | Diário incluído no **export Markdown** da sessão. |

### Não funcionais (RNF)

| ID | Requisito |
|---|---|
| RNF-01 | Hexagonal estrito: `domain → application → ports → adapters`; `domain`/`application` sem I/O; composition root só em `lib.rs` (BR-011, R1/R2). |
| RNF-02 | Toda porta nova tem **dobra de teste** (`InMemory*`/`Fake*`) (BR-013, R8). |
| RNF-03 | Cobertura **> 80%** em `domain`/`application` (BR-013, R9). |
| RNF-04 | Sem `unwrap()`/`expect()` em produção; erros tipados (R5 das rules). |
| RNF-05 | O print **não** é persistido como áudio nem trafega fora do provider escolhido. |
| RNF-06 | A pergunta é **persistida antes** de chamar a IA (se o app cair no meio, a entrada fica no diário como pendente). |
| RNF-07 | Latência percebida da resposta: o texto começa a chegar em streaming, sem esperar o fim (BR-028). |

## 6. Não-objetivos

- **Busca web** (`forceWebSearch` do Perssua): exige Responses API /
  `google_search` e capability por modelo — fica para uma PRD própria.
- **Busca em documentos** (o "Document sources" do Perssua): lá é via MCP
  (Drive/Docs/Sheets/Notion/Obsidian); aqui exigiria indexação de arquivos
  locais — PRD própria.
- **MCP / conectores / assistants** do Perssua.
- **Conta, login, sincronização, telemetria** (BR-002, BR-004).
- **Wayland nativo** para captura de tela (BR-007) — o caminho é X11/XWayland
  + colar imagem.
- **Export para PDF/e-mail** (o export é Markdown, BR-020).
- **Diário global** (memória entre sessões): o diário é **por sessão**
  (BR-052). A busca é por sessão; o aggregate entre sessões é o Histórico.

## 7. Riscos

| ID | Risco | Mitigação |
|---|---|---|
| R-01 | Captura de tela não funciona em Wayland nativo | Erro explícito + **colar imagem** como caminho principal; PRD do Wayland é spike (BR-007) |
| R-02 | Custo de tokens com imagem | 1 imagem por pergunta + reencode 1280/q0.86 (paridade com o `balanced` do Perssua) |
| R-03 | Streaming duplica texto quando o provider falha no meio | Single-flight no `AskService` (paridade com BR-042): só troca de fonte **antes** de emitir texto |
| R-04 | `CompletionRequest` multilíngue quebra providers que não aceitam `image_url` | Fallback: provider sem suporte multimodal → erro claro, sem retry cego |
| R-05 | Diário cresce sem limite (imagens em BLOB) | 1 imagem por entrada; cascade ao excluir a sessão; sem "limpar" automático (a sessão é o limite) |
| R-06 | Busca FTS com texto do usuário quebra a consulta | **Escapar** o `MATCH` (ISS-042) — o Perssua não escapa e falha |
| R-07 | Perguntas prontas viram "lixo" no arquivo de settings | Limite de 20 × 200 chars, IDs próprios, ordenação estável |
| R-08 | `settings.json` grande demais com muitas perguntas | Limite de 20; é configuração, não dado de sessão |

## 8. Métricas de sucesso

Sem telemetria (BR-004) — as métricas são **verificações manuais** do
desenvolvedor, não medições em campo:

1. A aba Sessão mostra o diário da sessão e ele **sobrevive a um reinício**.
2. Uma pergunta com print anexado recebe resposta **com o print** (a IA
   descreve o que está na tela).
3. Clicar numa pergunta pronta **envia** a pergunta e registra a origem.
4. A busca por palavra-chave acha a pergunta e a resposta.
5. O export Markdown traz a seção **Perguntas**.
6. `pnpm test` e `cargo test` verdes, com a cobertura do núcleo acima de 80%
   (RNF-03).

## 9. Evidência no Perssua (BR-015)

Extraída com REA do `Perssua-0.27.5-x86_64.AppImage`
(`examples/Perssua-0.27.5-x86_64.AppImage` → `resources/app.asar`).

| Comportamento no Perssua | Onde | Decisão aqui |
|---|---|---|
| Sessão como **diário**: `messages` (`role` user/assistant/system) + `transcriptions` + `subject`/`overview`, com `messages_fts` | `electron/database/DatabaseManager.js`, schema de `conversations.db` | Diário **por sessão** em tabela **própria** (`journal_entries` + `journal_fts`) — a tabela `messages` fica com as sugestões (BR-052) |
| **Ask** é uma **barra** flutuante: lupa + campo + atalho + botão de print, com os atalhos ⌘ Space / ⌥ C | barra do Perssua (`text-input`, `9707.9385c2cc.chunk.js`) | **Barra "Perguntar" no topo** (US-00), com Ctrl+Space / Alt+C |
| **Ask** = uma superfície só, com taxonomia `requestType` (`ask`/`followup`/`word`/`transcription`/`clipboard`/`screenshot`) e `requestSource` | `build/static/js/310.8def3e52.chunk.js` (`Cr`), `main.f180d025.js` (módulo 4924) | Origem registrada na entrada (`search`/`preset`/`screenshot`), para o diário distinguir |
| **Print**: `capture-screenshot` → JPEG q80 ≤1920×1080; reencode por preset (`balanced` = 1280/q0.86, default do Ask) | `electron/ipc/screen-capture/index.js`, `main.f180d025.js` (módulo 5525) | Reencode **1280/q0.86**; fonte do print é **X11** (`x11rb`), não `desktopCapturer` (Electron) |
| Limite do Perssua: **1 imagem por request** no plano free | `310.8def3e52.chunk.js` (`useImageManager`) | **1 imagem por pergunta** |
| **Prompt Suggestions**: máx. **4 × 140 chars**, em `localStorage["perssua.textInput.promptSuggestions"]`, com editar/resetar/salvar; chip dispara a pergunta | `main.f180d025.js` (módulo 3336), `9707.9385c2cc.chunk.js` | **20 × 200 chars** nas **settings** (sem paywall, BR-054); 4 defaults traduzidos |
| **Colar imagem** da área de transferência anexa à pergunta | `310.8def3e52.chunk.js` (`useScreenCapture`) | Mantido — é o caminho que funciona em Wayland |
| Contexto: a pergunta carrega a transcrição viva (`transcriptionContext`) e o histórico da conversa | `310.8def3e52.chunk.js` (`Cr`) | Reuso de [`application/context.rs`](../mvp/architecture/session-history.md) |
| Janelas do app escondidas durante a captura | `electron/ipc/screen-capture/index.js` (`protectPerssuaWindowsForCapture`) | Mantido |
| Busca de sessões: `getSessionsPage` com `messages_fts MATCH` + `LIKE` em subject/overview, página 20, debounce 180 ms | `electron/database/DatabaseManager.js`, `6982.6faaf9d1.chunk.js` | Busca só no diário da sessão, **com escape do `MATCH`** (o Perssua quebra com texto do usuário) |
| Sugestões de follow-up geradas pelo modelo | `main.f180d025.js` (módulo ~613209) | **Não** entra na v1 (as perguntas prontas cobrem o caso) |

### Desvios conscientes (R12)

1. **Perguntas prontas maiores** (20 × 200) que as do Perssua (4 × 140): o
   Perssua limita por *paywall*; aqui não há (BR-002).
2. **FTS ancorado por `id`**, não por `rowid` do FTS: o Perssua usa `rowid` e
   quebra num `VACUUM` (ver [`issues.md`](issues.md) → ISS-042).
3. **Sem "Ask" flutuante** no overlay: aqui o composer vive na aba Sessão e
   o notch ganha **um botão** — o overlay é `actionbar | hidden` (BR-032) e
   não há superfície expansível.
4. **Sem busca web nem busca em documentos** (v1) — ver §6.
