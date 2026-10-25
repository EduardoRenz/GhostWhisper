# Memory — PRD Sessão (diário de perguntas)

Histórico cronológico deste PRD (entradas `## YYYY-MM-DD — Título`, com linha
`IDs:`; convenções em [`../../.agents/rules.md`](../../.agents/rules.md)).

---

## 2026-10-19 — PRD criada (aba Sessão + feature Busca)

- **IDs:** BR-052, BR-053, BR-054, BR-055, BR-056, BR-057, BR-058, ISS-041,
  ISS-042 · **Refs:** [`prd.md`](prd.md),
  [`architecture/session-journal.md`](architecture/session-journal.md),
  [`tasks/_tasks.md`](tasks/_tasks.md)

### Pedido

O usuário pediu para **analisar o código do Perssua** e preparar uma PRD que
adicionasse o conceito de **"Sessão"**: uma **aba** que vai acumulando, em um
**diário**, tudo que o usuário perguntar em uma feature **nova de busca**, onde
se pode **perguntar**, **tirar print de tela** e **salvar perguntas prontas**.

### Decisões fechadas com o usuário (2026-10-19)

| Tema | Decisão |
|---|---|
| Âmbito do diário | **Por sessão** de transcrição (`session_id`) |
| Persistência | **Tabela nova** `journal_entries` + `journal_fts` próprios (não mexer em `messages`) |
| Escopo da v1 | **perguntar + print + perguntas prontas** (sem busca web, sem busca em documentos) |
| Captura de tela | **X11 via `x11rb` `GetImage`** (sem ferramenta externa) |
| Botão "Perguntar" | **No notch, ao lado de "Resumo"**, ícone de **lupa**, só com sessão iniciada |

### Evidência do Perssua (REA no `Perssua-0.27.5-x86_64.AppImage`)

- A sessão do Perssua **é** um diário: `messages` (`role` user/assistant/
  system) + `transcriptions` + `subject`/`overview`, com `messages_fts`
  (FTS5 com triggers) para busca.
- O **Ask** é uma superfície única com taxonomia `requestType` /
  `requestSource` (`ask`, `followup`, `word`, `transcription`, `clipboard`,
  `screenshot`).
- O **print** usa `capture-screenshot` → JPEG q80 ≤1920×1080, reencodeado por
  preset (`balanced` = 1280/q0.86, o default do Ask); **1 imagem por
  request**; as janelas do app são escondidas durante a captura; **colar
  imagem** também anexa.
- As **perguntas prontas** são as "Prompt Suggestions": máx. **4 × 140
  chars**, em `localStorage["perssua.textInput.promptSuggestions"]`, com
  editor (editar/resetar/salvar) e chips que disparam a pergunta. Defaults:
  *"Summarize everything so far"*, *"List action items"*, *"List main ideas
  discussed"*, *"What can I say?"*.
- O "**Document sources**" do Perssua é **MCP** (Drive/Docs/Sheets/Notion/
  Obsidian) — **não** é busca no filesystem local. O "**Web**" é uma chamada
  separada, só na barra flutuante e só para Gemini/OpenAI.
- A busca de sessões do Perssua é `getSessionsPage` (FTS `MATCH` + `LIKE` em
  subject/overview), página de 20, debounce 180 ms, **sem ranking**.

### Desvios consciente (R12)

1. Perguntas prontas **20 × 200** (o Perssua limita por paywall; aqui não há
   login — BR-002).
2. FTS ancorado por **`id`**, não por `rowid` (o do Perssua quebra num
   `VACUUM` — ISS-041).
3. **Sem "Ask" flutuante**: o composer vive na aba Sessão e o notch ganha
   **um botão** (o overlay é `actionbar | hidden` — BR-032).
4. **Sem busca web nem busca em documentos** na v1.

### Achado colateral — índices do schema

A análise do banco mostrou **índices simples** forçando *temp b-tree* em toda
listagem ordenada (`transcriptions`/`messages` por `session_id ORDER BY
timestamp`), **full scan** em `sessions ORDER BY started_at` e **três
índices mortos** (`idx_messages_timestamp`, `idx_transcriptions_timestamp`,
`idx_sessions_user` — `user_id` é sempre `NULL`, BR-002). `idx_sessions_status`
**é** usado (pelo `end_all_active`) e **fica**. A correção está na task 10,
junta da ancoragem do FTS (ISS-041).

### Implementação (2026-10-19)

Todas as **10 tasks** foram implementadas e estão `CONCLUDED`.

| Task | Entregue |
|---|---|
| 01 | `domain/journal.rs`, `ports/journal_repository.rs`, `LocalJournalRepository`, `InMemoryJournalRepository`, tabelas `journal_entries`/`journal_images`/`journal_fts` com FTS ancorado por `id` |
| 02 | Terceira aba **Sessão** no painel + botão **"Perguntar"** (lupa) no switcher do notch, entre "Resumo" e "Sugestões" |
| 03 | `application/ask_service.rs` (+ porta `AskController`), evento `journal`, escopo de rota `AiCapability::Search` |
| 04 | `features/journal/*` (aba, composer, cartão da entrada) + `useJournal` |
| 05 | `settings.saved_questions` (20 × 200) + chips + editor |
| 06 | `ports/screen_capture.rs` + `X11ScreenCapture` (`x11rb` `GetImage`, ≤1280 px, JPEG q0.86) + comando `screen_capture` (esconde as janelas do app) |
| 07 | `CompletionRequest.images` + `image_url` (OpenAI-compatível) e `inline_data` (Gemini) |
| 08 | Busca no diário com escape do `MATCH` (ISS-042) + seção **"Perguntas"** no export Markdown |
| 09 | BRs, ISS, memória, verificação e pacotes |
| 10 | Índices compostos (`(session_id, timestamp)`, `sessions.started_at`), remoções dos índices mortos e ancoragem do FTS por `id` (ISS-041) |

**Verificação:** `cargo test --lib` 492 ✓ · `cargo clippy --all-targets` limpo ·
`pnpm test` 307 ✓ · `tsc --noEmit` ✓ · `pnpm build` ✓. A captura real foi
validada no X11 desta máquina (1280 px, JPEG válido).

**Desvios do plano:** `Completion` ganhou `provider: Option<String>` (o
diário registra quem respondeu — BR-052); a dependência `image` (só JPEG)
entrou para reescalar/comprimir; `base64` para a imagem colada da área de
transferência.

**Ajustes depois do teste no app:** `journal_list`/`journal_search` passaram a
devolver **lista vazia** (e não erro) quando não há sessão — o estado vazio
da aba é a mensagem certa, não um erro. O **colar imagem** (`Ctrl+V` no
campo) entrou junto, como o caminho que funciona em Wayland.

**Validação no app rodando (round trip real):** 6 entradas no banco local
com `provider=openai`, `model=gpt-6-luna` e respostas em Markdown — duas com
print (`request_source=screenshot`) e quatro pelos chips
(`request_source=preset`). As 6 imagens (914 KB) estão em `journal_images` e
o `journal_fts` tem as 6 linhas. A busca por "ações" devolveu 2 entradas e
as queries com sintaxe FTS (`"aspas"`, `a*b`, `x:y`, `NEAR(`) devolveram 0
resultados **sem erro** (ISS-042 corrigido).

### Pendências

- Wayland nativo: sem captura pelo X11 (BR-007) — o caminho é **colar
  imagem**, com erro explícito quando não há display X11.
- Busca web / busca em documentos: PRD própria, se e quando for pedido.
- A captura pega a **tela inteira** (root window): em multi-monitor, todos os
  monitores entram na imagem.

---

## 2026-10-20 — v2: barra "Perguntar" no topo (referência do Perssua)

- **IDs:** BR-052 (revisada), BR-056 (revisada) · **Refs:** [`prd.md`](prd.md) US-00/US-03,
  [`architecture/session-journal.md`](architecture/session-journal.md) §8,
  [`tasks/02-task.md`](tasks/02-task.md), [`tasks/03-task.md`](tasks/03-task.md),
  [`tasks/06-task.md`](tasks/06-task.md)

### Pedido
O usuário achou a v1 incoerente com o Perssua: queria uma **barra discreta no
topo** como superfície padrão, a sessão abrindo **só depois** da pergunta/
captura, **screenshot sem texto** (a IA decide) e o layout da referência.

### Decisões
- **Barra no topo, centralizada** (1 040 px), uma pill: lupa + campo +
  `Ctrl Space` + divisor + **Capturar Tela** (`Alt C`). Ícones (microfone,
  ocultar, histórico, configurações) **fora da pill**.
- **A pergunta abre a sessão:** sem sessão ativa, o `AskService` cria uma
  sessão de perguntas (sem áudio); com gravação ativa, entra nela.
- **Print sem texto:** `DEFAULT_IMAGE_PROMPT` no core — a IA decide o que fazer
  com a imagem.
- **Atalhos globais:** `focus_ask` (Ctrl+Space) e `capture_screen` (Alt+C).
- **Espera do compositor** na captura: era a causa de o app aparecer no print.

### Ambiente (não é do código)
Após o reboot de 2026-10-20 o **WebKitGTK não pinta** nenhuma janela do app
(tela preta), inclusive com o código anterior à v2 — enquanto o **Chrome**
renderiza a mesma página. O layout da v2 foi validado no Chrome; o app em si
precisa ser revalidado numa sessão/compositor saudáveis.

---

## 2026-10-20 — v3: lupa no notch, barra de pesquisa e print só anexado

- **IDs:** BR-052 (revisão 2), BR-055 (revisão) · **Refs:** [`prd.md`](prd.md) US-00/US-03,
  [`architecture/session-journal.md`](architecture/session-journal.md) §8

### Pedido
O notch antigo **não devia ter sido substituído**: ele fica, e ganha um botão
de **pesquisa** (lupa, cor primária + atalho) **independente de transcrição**,
que abre a barra de pesquisa. A captura **não envia**: ela só **anexa**, com a
prévia no canto superior direito (como no Perssua); o envio é sempre
confirmado.

### Decisões
- **Notch restaurado** (marca, "Começar a ouvir", switcher, timer, canais,
  ícones) com a **lupa** ao lado da marca; estilo alinhado ao da barra.
- **Barra de pesquisa** numa janela própria (`ask`), aberta pela lupa ou por
  **Ctrl+K**; `Esc` fecha; some depois do envio.
- **Prévia do print** em janela própria (`capture`) no canto superior direito,
  com `×`; o print vive na porta `PendingCapture` (memória) até ser enviado.
- O envio leva o print pendente junto; sem texto, o core usa o prompt padrão.

### Ambiente (não é do código)
O WebKitGTK desta sessão (pós-reboot) **não pinta** nenhuma janela do app —
inclusive o release e o código anterior. O `dist` embutido foi validado no
Chrome (renderiza o notch com a lupa exatamente como o mockup).

---

## 2026-10-20 — v4: fundo do notch, alça na barra, print sem texto e retorno das janelas

- **IDs:** BR-052, BR-055 · **Refs:** [`prd.md`](prd.md) US-00/US-03

### Correções do teste do usuário
- **Fundo do notch** restaurado: a regra `.overlay__bar[data-state="idle"]`
  (que deixava a pill transparente para a barra de pesquisa embutida) saiu
  junto com a versão antiga; o notch volta a ser pill opaca e ancorada pela
  base.
- **Alça de arraste** na barra de pesquisa (à esquerda), como no notch.
- **Texto nunca obrigatório com print**: a barra e o composer da aba Sessão
  checam o **print pendente** do core; com ele, `Enter`/Perguntar vão sem
  texto (prompt padrão no core).
- **Janelas voltam** depois do print: o `hide`/`show` ganhou respiro
  (`RESTORE_SETTLE_MS`), `unminimize` antes do `show`, e o foco é devolvido à
  janela que o tinha; a barra ainda chama `ask_open` depois da captura como
  garantia.

---

## 2026-10-20 — v5: a aba Sessão vira o diário do mockup (Perssua)

- **IDs:** BR-052, BR-053, BR-054 · **Refs:** [`prd.md`](prd.md) US-00/US-01/US-06

### O que mudou
- **Fim da sessão limpa a aba:** `useJournal` recebe o `session_id` ativo e
  descarta a lista quando ele muda (encerrou/continou/trocou). Antes a lista
  ficava em memória e mostrava o diário da sessão anterior.
- **Mais recente no topo:** a UI inverte a ordem cronológica do core (o
  repositório e o export seguem em ordem cronológica).
- **Card do mockup** (`JournalEntryCard`): legenda de origem com ícone
  (`Pesquisa` / `Screenshot` / `Screenshot + Texto` / `Pergunta pronta`),
  tempo relativo + hora de parede, selo **Mais Recente** (ou o modelo),
  pergunta (`Q:`/`Contexto:`), miniatura do print com legenda e a resposta em
  bloco Markdown. Separador entre entradas = linha.
- **Diário puro:** o composer e a busca saíram da aba (`AskComposer` e
  `SavedQuestionsEditor` removidos). As **perguntas prontas** viraram chips na
  barra de pesquisa e são editadas em **Configurações → Assistente**.
- **Rodapé:** contagem de interações, `Sync local · SQLite` e **Exportar MD**
  (reusa o export da sessão, que já inclui as perguntas).
- **Pergunta do usuário no banco:** com print puro, `journal_entries.question`
  fica **vazia** (o prompt padrão da imagem vale só para o modelo); assim a UI
  sabe quando **não** mostrar a pergunta. `created_at_ms` (epoch ms) entrou na
  entrada para a hora de parede mesmo em sessão aberta pela lupa.
- **Miniaturas:** carregadas sob demanda por `journal_images` (só entradas com
  `request_source = screenshot`) e cacheadas por entrada.

---

## 2026-10-20 — v6: sempre existe uma sessão (a pergunta abre uma pausada)

- **IDs:** BR-019, BR-052, BR-053 · **Refs:** [`prd.md`](prd.md) US-00/US-01

### O bug
Perguntar **sem transcrição** mostrava a resposta em streaming e a **apagava**
em seguida: o reload do fim da resposta chamava `journal_list`, que resolvia a
sessão só pela sessão de **gravação** (`state.session.current_session_id()`);
sem gravação a lista vinha vazia e substituía a tela. O `AskService` criava uma
sessão paralela que o resto do app não via.

### A decisão
- **Uma sessão só.** `SessionController::ensure_session` devolve a sessão ativa
  ou abre uma **pausada** (`ActiveSession { live: [] }`, sem capturar), já
  publicada em `session_state` (`listening: true, paused: true`) — logo aparece
  na pílula do notch, no Histórico e no diário.
- `journal_ask` (IPC) chama `ensure_session()` e passa o id na `AskRequest`
  (ajuda extraída em `resolve_ask_session`, testada).
- `start` **continua** na sessão aberta (`set_channels` + `resume`) em vez de
  encerrar e criar outro id; `set_channels` passa a lembrar a escolha mesmo
  sem sessão (a sessão automática herda; padrão microfone).
- **Frontend:** o reload pós-resposta **mescla** (`mergeEntries`) — a resposta
  na tela nunca é apagada; encerrar a sessão continua limpando a aba (troca de
  `session_id` zera a lista).
- `ChannelMenu`: "Nova sessão" → **"Começar a capturar"** (é a mesma sessão).

---

## 2026-10-20 — v7: preset "Responda a última pergunta identificada"

- **IDs:** BR-054 · **Refs:** [`prd.md`](prd.md) US-04

- **O que é:** um **pedido** (não uma pergunta) como pergunta pronta, no fim
  da lista: a IA responde à última pergunta que identificou na conversa ou na
  tela (`domain::settings::REPLY_TO_LAST_QUESTION`).
- **Migração única** (`application/saved_questions_migration.rs`): quem já
  tinha a lista persistida recebe o preset **uma vez**
  (`saved_questions_migrated`), sem duplicar e sem ressuscitar os que apagou;
  respeita o teto de 20. Roda no boot, depois da migração do agente.
- **Instalação nova** já nasce com cinco presets (`default_saved_questions`).
- Testes: `saved_questions_migration` (adiciona uma vez, idempotente, não
  duplica, respeita o teto, instalação nova, não ressuscita apagado) e o
  default no frontend.

---

## 2026-10-20 — v8: a barra de pesquisa reabre centralizada e com o cursor no campo

- **IDs:** BR-052

- **Posição:** `show_ask_window` (`ipc/ask_commands.rs`) recentraliza a barra no
  topo **a cada abertura** (quando ela estava escondida), antes e depois do
  `show` — o WM pode reencaixar a janela ao mapeá-la, e antes a posição só era
  aplicada no boot (a barra reabria onde tivesse sido arrastada, às vezes fora
  da tela e escondendo a alça). O setup do `lib.rs` usa o mesmo
  `center_ask_window` (uma fonte só; `ASK_WIDTH` mudou para o módulo do ask).
- **Foco:** `ask_open` e o atalho emitem `ask_focus`; a janela ainda escuta
  `onFocusChanged` e devolve o cursor ao input — o foco do WM chega **depois**
  do evento, então só o evento não bastava.
- Teste: o `ask_focus` leva o `document.activeElement` para o campo; o mock do
  `askOpen` entrou no teste (antes ele estourava em silêncio no caminho do
  print).

---

## 2026-10-20 — v9: contexto incremental + resumo na própria resposta

- **IDs:** BR-053, BR-024 · **Refs:** [`prd.md`](prd.md) US-02

### O problema
Cada pergunta reenviava os **últimos 180 s** de transcrição (até 40 falas /
6 000 chars), mesmo o trecho já usado para responder a pergunta anterior — e a
IA não sabia o que já tinha respondido (as respostas não entravam no prompt).

### O desenho
- **Corte:** `asked_at_ms` da última entrada **`answered`** da sessão; entra só
  a fala posterior a ela, limitada por `max(cut, now − 180 s)`
  (`context::context_since`, função pura).
- **Resumos:** janela deslizante de **5** entradas com `summary` (~1 500 chars),
  em `<previous_questions>` no prompt, antes de `<transcript>`.
- **Resumo sem chamada extra:** o `SYSTEM_PROMPT` pede um trailer
  `<!--resumo: …-->` no fim da resposta; `application/answer_summary.rs` o
  remove (streaming com *hold-back* do prefixo, texto final sem trailer) e o
  guarda em `journal_entries.summary`. Fallback determinístico
  (`digest = "P: … / R: …"`) quando o bloco não vem.
- **Schema:** coluna `summary` (nullable) + migração `ALTER TABLE` idempotente
  em `connection.rs` (testada com um banco "antigo").
- **Prompt do sistema** ganhou: não repetir o que já foi respondido; a
  transcrição é só o depois.

### Verificação
- `cargo test --lib` 537 ✓ · `pnpm test` 340 ✓ · `clippy` limpo · `tsc` ✓.
- Testes novos: trailer (extração, streaming sem vazar o marcador, texto final,
  fallback), `context_since` (delta, janela, nada novo), segunda pergunta só com
  o delta + resumo, corte ignorando `error`, rolling de 5, migração da coluna.

---

## 2026-10-20 — v10: o prompt volta a pedir a resposta completa

- **IDs:** BR-053 · **Refs:** [`prd.md`](prd.md) US-02

- **Sintoma:** depois da v9, respostas como "inverta uma árvore binária em
  Python" saíam rasas — sem bloco de código completo nem exemplo.
- **Causa:** o prompt da v9 juntou "use os resumos para **não repetir** o que
  já foi dito" e "termine **sempre** com o bloco de resumo". Juntas, soam como
  "seja breve": o modelo tratava a resposta como um resumo. O "direta e curta"
  que já existia só piorava.
- **Correção:** o prompt agora pede **Markdown completo** (bloco com a
  linguagem + exemplo de uso, passos quando for explicação) e diz que o bloco
  de resumo é **apenas o fecho** — "a resposta acima deve estar completa". Os
  resumos anteriores servem para manter o fio, não para encurtar. `MAX_TOKENS`
  900 → **1 500** (código + exemplo + resumo; modelos que contam raciocínio no
  mesmo teto também cabem).
- **Teste:** regressão do prompt (contém "Markdown completo"/"exemplo de uso",
  não contém "direta e curta"/"não repetir").

---

## 2026-10-20 — v11: a Busca usa o agente **Padrão** e nunca devolve a pergunta

- **IDs:** BR-053, BR-059, BR-060 · **Refs:** [`prd.md`](prd.md) US-02

- **Sintoma:** "inverta uma árvore binária em Python" virou *"Você quis dizer
  inverter…? Pode enviar o código?"* — a IA devolvia a pergunta.
- **Causa:** o prompt da Busca era uma persona fixa de reunião ("responda
  perguntas sobre uma reunião em andamento… diga quando a resposta não está no
  contexto"). Sem a pergunta na transcrição, o modelo pedia confirmação.
- **Correção:** a Busca passa a compor `DEFAULT_SYSTEM_PROMPT` (agente
  **Padrão**: identifica a cena, "a responder, não um entrevistador — nunca
  pergunta de volta") + `ASK_RULES` (resposta completa de uma vez, bloco de
  código com exemplo, **assume erros de ditado**, transcrição é contexto e não
  requisito, trailer de resumo como fecho). `NEVER_ASK_BACK_RULE` **não** entra
  (ele manda parar sem contexto). `MAX_TOKENS` 1 500.
- **Teste:** o prompt compõe o Padrão, contém "nunca devolva a pergunta" e
  "erros de transcrição", não contém a persona fixa de reunião nem a regra ao
  vivo.

---

## 2026-10-20 — v12: composer compacto de volta na aba Sessão

- **IDs:** BR-053 · **Refs:** [`prd.md`](prd.md) US-02

- **Pedido:** voltar o composer da aba (que a v5 tirou para seguir o mockup),
  agora **compacto**: campo + sugestões e **print sem prompt**.
- **O que entrou:**
  - `features/journal/AskComposer.tsx` — uma linha (câmera + input + enviar),
    chips das perguntas prontas (scroll horizontal) e a miniatura do print com
    "×". `Enter` envia; com print, o texto é opcional (a IA decide).
  - `core/application/usePendingCapture.ts` — o print pendente do core
    (`capture_pending` + `capture_changed`), compartilhado com a prévia do
    canto; capturar **só anexa**.
  - `useJournal.ask`: com print a origem vira `screenshot` **sempre** (antes só
    quando a origem era `search`), para a miniatura da entrada carregar.
- **O que continua:** a lupa/Ctrl+K, a barra de pesquisa e o trailer de resumo;
  a aba segue com a lista (mais recente no topo), o rodapé e o export.

---

## 2026-10-21 — Sinal de transcrição na aba (streaming por canal)

- **IDs:** BR-063, T11 · **Refs:** [`tasks/11-task.md`](tasks/11-task.md),
  [`../chat/architecture/transcription-panel.md`](../chat/architecture/transcription-panel.md) §2.1

### Pedido
Na aba **Sessão** (Perguntas e respostas), saber quando a fala ainda está sendo
transcrita. O efeito vai na **aba Transcrição existente**, sem botão novo:
ciano L→R para microfone, cor de Outros R→L, ambos simultâneos.

### Implementação
- Evento `transcript_activity` (`session_id`, `channel`, `processing`) emitido
  pelo `TranscriptionPipeline`: `true` no primeiro bloco falado do turno,
  `false` quando o passe final termina (ou o turno é descartado/parado).
- `useTranscriptActivity` filtra a sessão e mantém um booleano por canal.
- `PanelTabs` (aba existente) recebe a atividade, mostra o ponto de status e
  aplica os sweeps CSS; o clique continua trocando para Transcrição.

### Verificação
- `cargo test --lib` 545 ✓ · `clippy` ✓ · `rustfmt` ✓.
- `pnpm test` 357 ✓ · `tsc` ✓ · `pnpm build` ✓.
- Validado no AppImage: ponto + mic na aba existente; áudio do sistema acendeu
  o ponto na cor de Outros e o sweep na aba.


## 2026-10-21 — Barra “Perguntar” não se move sozinha (ISS-055, task 12)

- **IDs:** ISS-055, T12, BR-052 (rev. 6), BR-072
- **Refs:** [`issues.md`](issues.md), [`tasks/12-task.md`](tasks/12-task.md),
  [`prd.md`](prd.md) US-00,
  [`architecture/session-journal.md`](architecture/session-journal.md) §8

### Relato
*“A barra de pesquisa está com comportamento semelhante [ao notch]; em especial
testei quando há captura de tela. Após ela retornar a visibilidade, ela pula
para outro lugar. Qualquer janela não pode se mover sozinha a não ser que eu
arraste explicitamente.”*

### Diagnóstico
- `show_ask_window` chamava `center_ask_window` antes e depois do `show` quando
  a janela estava escondida (revisão 5 da BR-052).
- A captura esconde todas as janelas por ~260 ms e o `AskWindow` chama
  `askOpen()` ao voltar; isso recentralizava a barra.
- A barra não tinha `ask_position` nem tracker: a posição real não era
  persistida nem protegida de snapshots antigos de settings.

### Correção
- `show_ask_window` só mostra e foca; nenhum `set_position` em abertura/foco.
- `ask_position` novo em `Settings` (Rust + TS), com `serde(default)` e default
  `None`; restaurado apenas no boot; primeira execução ainda centraliza.
- `spawn_ask_position_tracker` persiste a posição visível.
- `preserve_window_geometry` inclui `ask_position`: save de outra janela não
  sobrescreve a geometria real.
- BR-072 formaliza a regra geral: só arraste explícito move uma janela, com
  exceções de colocação inicial, âncora/densidade do notch e popovers ancorados.

### Verificação
- `cargo test --lib` **596 ✓** (10 ignorados) · clippy `-D warnings` ✓ ·
  rustfmt dos arquivos tocados ✓.
- `pnpm test` **387 ✓** · `tsc --noEmit` ✓ · `pnpm build` ✓.
- AppImage no Xvfb: `440,12` → movida para `300,200` → captura de tela →
  permaneceu `300,200`; reinício → reabriu `300,200`. Settings do usuário
  restaurados do backup. Pacotes às 14:22.

## 2026-10-21 — Follow-up: WM remapeava a barra em 0,0 (ISS-055, task 12)

- **IDs:** ISS-055, T12, BR-052 (rev. 6), BR-072
- **Refs:** [`issues.md`](issues.md), [`tasks/12-task.md`](tasks/12-task.md)

### Relato do teste do usuário
*“A barra de pesquisa continua mudando de lugar, ela quase sempre reseta no
canto superior esquerdo. Justamente ela eu queria que ficasse na última posição
que deixei sempre.”*

### Diagnóstico
- O `ask_position` persistido estava correto (`398,343`), então o autosave não
  era o problema.
- No WM real, uma janela oculta (`hide`) é remapeada em `0,0` ao chamar `show`;
  a primeira correção não reaplicava a posição no show e o WM vencia.
- O fluxo de captura também mostrava a janela pelo `restore_app_windows`
  genérico antes do `askOpen`; quando o `askOpen` via a janela já visível, não
  corrigia a posição.

### Correção complementar
- `show_ask_window` reaplica a **mesma `ask_position` salva** antes e depois do
  `show`; nunca centraliza quando há posição salva.
- `persist_ask_position` grava a posição real antes de `ask_hide` e antes da
  captura (arraste seguido de fechamento imediato não se perde).
- `screen_capture` agora guarda `(posição, tamanho)` de **todas** as janelas
  visíveis antes do `hide` e reaplica antes/depois do `show` no retorno.
- O tracker periódico continua como fallback, mas não é mais necessário para o
  ciclo de captura/fechamento.

### Verificação
- AppImage novo no Xvfb:
  - abriu em `398,343`; escondida e movida para `0,0` enquanto oculta; reabriu
    em `398,343`;
  - captura de tela manteve `398,343` e a prévia apareceu;
  - arraste para `250,450` + `Esc` imediato → `ask_position = [250,450]`;
    reabriu em `250,450`.
- `cargo test --lib` **596 ✓** · clippy `-D warnings` ✓ · rustfmt ✓.
- Estado do usuário restaurado do backup (`ask_position = [398,343]`) e pacotes
  gerados às 15:04.

## 2026-10-21 — Notch atrás das janelas após o print (ISS-056, task 13)

- **IDs:** ISS-056, T13, BR-073
- **Refs:** [`issues.md`](issues.md), [`tasks/13-task.md`](tasks/13-task.md)

### Relato
*“Ao usar o atalho para tirar print, o Notch não volta; me parece que ele fica
atrás da tela. A ideia é que todas as janelas do GhostWhisper sempre fiquem à
frente.”*

### Diagnóstico
- A captura restaura as janelas com `show()` genérico.
- A correção da ISS-055 passou a reaplicar `position`/`size` **depois** do
  `show`; no GTK/Mutter esse remapeamento derruba `_NET_WM_STATE_ABOVE`
  (ISS-043).
- O overlay só reafirmava o topo dentro do seu próprio controlador
  (`TauriOverlayWindow::show`/`apply_stealth`), mas a captura chamava
  `WebviewWindow::show()` direto.

### Correção
- Novo helper `ipc::window_visibility::{show_above, assert_above}`: reafirma
  `alwaysOnTop` antes/depois do `show`.
- A restauração da captura só altera geometria quando o WM realmente mudou e
  termina com `assert_above` em todas as janelas.
- Prévia do print, Perguntar, painéis, Configurações, seletor de gravação,
  modal de fim de sessão, dwell e o tray usam a mesma reafirmação.
- BR-073 formaliza “todas as janelas sempre à frente”.

### Verificação
- `cargo test --lib` **596 ✓** · clippy `-D warnings` ✓ · rustfmt ✓.
- `pnpm test` **387 ✓** · `tsc` ✓ · build do AppImage em andamento.

### Validação
- O AppImage final foi aberto no Xvfb; o overlay iniciava sem `_NET_WM_STATE`.
- Após o atalho de print (ciclo hide/captura/show), `xprop` passou a mostrar:
  - overlay: `_NET_WM_STATE_ABOVE, _NET_WM_STATE_SKIP_TASKBAR,
    _NET_WM_STATE_SKIP_PAGER`;
  - Perguntar e prévia do print: os mesmos flags.
- Geometria preservada: overlay `554,855`, Perguntar `567,32`, prévia
  `1664,16`. Estado do usuário restaurado do backup; pacotes às 15:36.

## 2026-10-21 — Regressão: tracker voltou a salvar posição automática do WM (ISS-057, task 14)

- **IDs:** ISS-057, T14, BR-072, R16, R17
- **Refs:** [`issues.md`](issues.md), [`tasks/14-task.md`](tasks/14-task.md),
  [`../../.agents/rules.md`](../../.agents/rules.md) §4

### Relato
*“Voltou um problema que havia acabado de resolver: a Search bar perdeu a
posição que deixei. Como impedir regressões? Talvez testes de regressão.”*

### Diagnóstico
- O `settings.json` tinha `ask_position: [0,32]` (canto do WM), embora a
  posição anterior conhecida fosse `[567,32]`.
- O `show` reaplicava a posição salva, mas o **tracker** e o
  `persist_ask_position` antes de esconder/capturar gravavam qualquer posição
  observada. Uma posição automática do WM substituía a escolha do usuário.
- A correção anterior (ISS-055) tratava o reposicionamento, não a **origem**
  da posição persistida.

### Correção
- `AskPositionSource` no domínio e `Settings::apply_ask_position`:
  `Automatic` nunca persiste, `UserDrag` substitui, `InitialPlacement` só
  preenche posição vazia.
- `AskDragState` gerenciado pelo Tauri, iniciado por `ask_start_drag` e expirando
  em 2,5 s; a alça da barra usa esse comando em vez do arraste genérico.
- Tracker e persistência classificam a origem; posições automáticas do WM são
  descartadas.
- Reafirmação tardia da posição salva depois do `show` vence o mapeamento do WM.
- Regras R16/R17 adicionadas: teste anti-regressão obrigatório e decisão de
  ambiente isolada em lógica pura.

### Verificação
- Testes novos: `automatic_ask_position_never_overwrites_user_position`,
  `initial_ask_position_only_fills_empty`, `drag_intent_expires_after_idle`,
  `ending_drag_disables_persistence_immediately` e o teste da alça no frontend.
- `cargo test --lib` **600 ✓** (10 ignorados) · clippy `-D warnings` ✓ ·
  rustfmt ✓.
- `pnpm test` **388 ✓** · `tsc --noEmit` ✓.
- AppImage 15:52: movimento automático para `[0,32]` foi ignorado
  (`ask_position` permaneceu `[567,32]`); arraste real para `271,272` foi
  persistido e sobreviveu a fechar/reabrir/capturar. Estado restaurado.

## 2026-10-25 — Perguntar com memória longa: resumo no contexto, em paralelo (task 15)

- **IDs:** BR-053, BR-024, R16, T15 · **Refs:** [`prd.md`](prd.md) US-02,
  [`architecture/session-journal.md`](architecture/session-journal.md) §5.1,
  [`tasks/15-task.md`](tasks/15-task.md),
  [`../precision-and-performance/memory.md`](../precision-and-performance/memory.md)

### Relato

Depois da avaliação de desempenho, o usuário notou que a janela de 180 s do
Perguntar faz a IA "esquecer" o começo em reuniões longas: aos 30 min, "o que
decidimos no começo?" não tinha de onde vir. O resumo incremental já cobre a
reunião desde o início, mas **não** entrava no prompt da Busca.

### Decisão (com o usuário)

- Opção escolhida: **manter a janela recente + anexar o resumo da reunião**
  (criando/atualizando sob demanda), em vez de um campo de tempo nas
  configurações (que continuaria limitado pelo teto de 6 000 chars e só
  aumentaria custo/risco) ou de manter como estava (buraco de confiança).
- **Velocidade primeiro:** a resposta nunca espera o resumo; a atualização roda
  em paralelo e a falha dela não impede a pergunta.
- Sem toggle nas configurações nesta versão; o resumo entra sempre que existir.

### Implementação

- `AskService` passou a depender de `SummaryGenerator` (porta existente) e lê o
  resumo gravado em `build_context` (`get`); a leitura é **não-fatal** (erro →
  `None`).
- O prompt ganhou `<meeting_summary>` (assunto + markdown, teto de 6 000 chars)
  **antes** do `<transcript>`; o `ASK_RULES` declara que a fala recente tem
  precedência. Com o resumo presente, a janela de resumos de perguntas antigas
  cai de 5 para 3.
- `spawn_summary_refresh` dispara `generate(Auto)` em thread própria **antes**
  da chamada ao LLM da resposta, com single-flight por sessão (hash set) e
  cooldown de 60 s após falha; um guard de `Drop` libera o single-flight mesmo
  em panic. Erros só em log.
- Injeção no composition root (`lib.rs`).

### Verificação

- `cargo test --lib` **692 ✓** (10 ignorados) · `clippy --all-targets -D
  warnings` ✓ · `cargo fmt --check` ✓.
- Testes novos: bloco/ordem/assunto no prompt; ausência do bloco sem resumo;
  janela 3; resumo fresco não dispara; falha do resumo não falha a resposta;
  prova de paralelismo com o resumo bloqueado; single-flight/cooldown;
  staleness; corte UTF-8.
- Nenhuma mudança de frontend.

### Limite conhecido

Se a **primeira** pergunta ocorrer antes de qualquer resumo existir, ela
responde sem a memória longa (velocidade primeiro); o resumo é gerado em
background e a próxima pergunta já o utiliza. O warm-up na abertura da barra
ficou de fora para não gerar chamada extra a cada abertura nem duplicar a
geração disparada pelo próprio Perguntar.

## 2026-10-25 — Atalho de pesquisa ativa a barra mesmo com o app inativo (ISS-064)

- **IDs:** ISS-064, BR-052 (rev. 7) · **Refs:**
  [`../chat/issues.md`](../chat/issues.md) (ISS-064),
  [`../chat/tasks/18-task.md`](../chat/tasks/18-task.md)

O atalho global de pesquisa passa a **ativar** a janela `ask` com o timestamp
do servidor X11 (o `present` padrão usa timestamp 0 e o Mutter recusa a
ativação de um app inativo), inclusive quando a barra já está aberta. O campo
continua recebendo o cursor por `ask_focus` + `onFocusChanged`. A posição e o
tamanho da barra não mudam. Detalhes e testes no ISS-064 (PRD Chat).
