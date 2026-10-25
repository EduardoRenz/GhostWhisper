# PRD — GhostWhisper Chat (paridade de conversa com o Perssua)

- **Versão:** 0.2
- **Data:** 2026-10-19
- **Status:** v0.1 implementada (tasks 01–09 `CONCLUDED`); **v0.2 implementada**
  (tasks 10–16 `CONCLUDED`) — ver §9
- **Relacionados:** [`../business-rules.md`](../business-rules.md),
  [`memory.md`](memory.md), [`issues.md`](issues.md),
  [`architecture/chat-ui.md`](architecture/chat-ui.md),
  [`architecture/transcription-panel.md`](architecture/transcription-panel.md),
  [`tasks/_tasks.md`](tasks/_tasks.md), PRD anterior: [`../mvp/prd.md`](../mvp/prd.md)

---

## 1. Objetivo

Transformar a exibição da transcrição ao vivo num **chat em bolhas** idêntico ao do
Perssua (referência de produto, BR-015) e trocar os controles de sessão por
**ícones clássicos de transporte** (play / pause / stop), corrigindo de passagem o
**bug de linha temporal** que embaralhava as falas de fontes de áudio diferentes
([`issues.md`](issues.md) → ISS-018).

O MVP (`prds/mvp`) entregou captura, streaming, sessão contínua, histórico, export e
stealth. O que falta é **forma**: hoje as falas são linhas de texto e os controles
são botões de texto, o que não corresponde ao produto de referência.

> **Revisão 0.2 (2026-10-19):** a v0.1 entregou bolhas, transporte e janelas. A
> v0.2 revisa a **forma do painel ao vivo** (print de referência anexado pelo
> usuário) e adiciona a **sugestão manual a partir de uma mensagem** — ver §9. A
> linha temporal, o transporte e as janelas da v0.1 não mudam.

## 2. Problema

1. **Linha temporal errada (ISS-018).** O id do segmento não carrega o **canal**.
   Microfone e sistema compartilham `session_id`, a origem de tempo e a numeração
   de turno (cada pipeline começa em `turn = 0`), então os dois primeiros turnos
   têm o **mesmo id**: a UI substitui a bolha (uma frase antiga "recebe" o texto do
   outro canal, no lugar antigo) e o `INSERT` do segundo final falha na chave
   primária — **um lado da conversa some do banco**, do histórico e do export.
2. **Falas não parecem uma conversa.** Linha de texto com `tempo + falante`, sem
   a leitura visual de quem falou; o Perssua usa bolhas alinhadas por falante.
3. **Prévia em andamento no meio da lista.** O texto ainda revisável (`final =
   false`) fica na posição em que apareceu e cresce ali; o Perssua o mostra como
   bolha "em andamento" **no fim**, com indicador de digitação.
4. **Controles não são os do Perssua.** Hoje são botões com rótulo ("Pausar",
   "Parar"), sem ícones, sem tempo de sessão e sem confirmação de encerramento.

## 3. Público-alvo e casos de uso

Mesmas personas do MVP (`../mvp/prd.md` §3). Casos de uso desta PRD:

- **CU-01 — Ler a conversa:** quem falou aparece em bolha à direita (Você) ou à
  esquerda (Interlocutor), em ordem de tempo, com bolhas intercaladas quando as
  duas fontes de áudio falam em paralelo.
- **CU-02 — Acompanhar a fala em curso:** a fala ainda em revisão aparece no fim,
  como bolha "em andamento" com `…`, e migra para a sua posição na linha temporal
  quando o turno fecha.
- **CU-03 — Ver a sugestão da IA na conversa:** a sugestão é uma bolha na mesma
  linha temporal, com tempo e botão de copiar.
- **CU-04 — Controlar a sessão por ícones:** play (retomar), pause e stop, mais o
  tempo decorrido; encerrar pede confirmação.
- **CU-05 — Revisar no histórico:** o detalhe da sessão usa as mesmas bolhas.
- **CU-06 — Assistir ao lado:** manter a conversa numa janela própria, movível e
  redimensionável, sobre a reunião, sem ocupar o overlay.
- **CU-07 — Revisar sessões antigas:** abrir o histórico numa janela própria e
  continuar/renomear/exportar sem sair do fluxo.

## 4. Estórias de usuário (com critérios de aceite)

- **US-01 — Bolhas intercaladas.** Como usuário, quero ver as falas em bolhas
  alinhadas por falante, na ordem do tempo.
  - **CA-01.1** Falas finalizadas são ordenadas por `at_ms` (empate: id) e
    renderizadas como bolha de "Você" (direita) ou "Interlocutor" (esquerda).
  - **CA-01.2** Duas fontes falando em paralelo produzem **bolhas distintas e
    intercaladas** por tempo — nunca a substituição do texto de uma pela outra.
  - **CA-01.3** Cada bolha mostra o tempo desde o início da sessão (`mm:ss`).
  - **CA-01.4** O visual segue os valores do Perssua documentados em
    [`architecture/chat-ui.md`](architecture/chat-ui.md) §3 (largura máx. 75%,
    raio 12px, tonalidades por falante, rótulo em maiúsculas).
- **US-02 — Fala em andamento no fim.** Como usuário, quero ver o que ainda está
  sendo transcrito como bolha "em andamento" no fim da lista.
  - **CA-02.1** Segmentos não finalizados **não** entram na linha temporal; são
    exibidos como bolha em andamento (uma por falante, no máximo) **depois** do
    último item.
  - **CA-02.2** A bolha em andamento tem `opacity` reduzida e indicador `…`
    piscando; quando o turno fecha, ela some e a fala finalizada aparece na
    posição correta da linha temporal.
  - **CA-02.3** O texto final **nunca** é sobrescrito por uma prévia do mesmo id.
- **US-03 — Sugestão da IA na conversa.** Como usuário, quero a sugestão da IA na
  mesma conversa, com tempo.
  - **CA-03.1** Sugestões entram na linha temporal por `at_ms`, como bolha de
    rótulo "Sugestão da IA", com `mm:ss` e botão **Copiar**.
  - **CA-03.2** A sugestão empurrada pelo atalho (evento `insight`) não duplica a
    que veio do banco (dedupe por `message_id`).
  - **CA-03.3** O botão **Sugerir** fica no cabeçalho do chat.
- **US-04 — Controles de transporte.** Como usuário, quero play, pause e stop com
  ícones, e o tempo da sessão.
  - **CA-04.1** Parado: pílula "Iniciar escuta" (ícone play + rótulo) que inicia
    com os canais escolhidos.
  - **CA-04.2** Gravando: pílula `[▶][⏸][⏹]` com ▶ esmaecido e ⏸/⏹ ativos.
  - **CA-04.3** Pausado: ▶ ativo, ⏸ esmaecido, ⏹ ativo.
  - **CA-04.4** O tempo desde o início da sessão (`mm:ss`) aparece na pílula de
    sessão em todos os estados de sessão aberta.
  - **CA-04.5** ⏹ e a pílula de sessão abrem a confirmação **"Encerrar sessão?
    Sim / Não"**; só "Sim" encerra.
  - **CA-04.6** A bolinha de status acompanha o estado (vermelha pulsando gravando,
    âmbar pausado, cinza parado).
  - **CA-04.7** Trocar os canais continua permitido em qualquer estado (BR-019).
  > Revisada pela **US-08** (task 09): o transporte passa a ser um toggle único e o
  > encerramento acontece só pela pílula de sessão.
- **US-05 — Histórico em bolhas.** Como usuário, quero o detalhe da sessão no
  histórico com as mesmas bolhas da conversa ao vivo.
  - **CA-05.1** O detalhe usa a linha temporal do backend (falas + sugestões) e o
    mesmo componente de bolha.
- **US-06 — Conversa em janela própria.** Como usuário, quero a conversa numa
  janela separada, ao lado da reunião, sem depender do overlay expandido.
  - **CA-06.1** A conversa ao vivo (bolhas, fala em andamento e sugestões) roda na
    janela `chat`, com a linha temporal das US-01–US-03.
  - **CA-06.2** Um botão **Chat** alterna esconder/mostrar a janela; o ✕ esconde
    (não destrói) e reabrir mantém a sessão.
  - **CA-06.3** A janela é arrastável pelo cabeçalho e redimensionável (mínimo
    360×320); **posição e tamanho voltam** ao reabrir o app (BR-026).
  - **CA-06.4** Os medidores de nível ficam na janela de Chat.
    > **Revisão 0.2 (BR-040):** os medidores **saem** da aba Transcrição; o
    > waveform do notch continua. Ver §9 (CA-09.5).
  - **CA-06.5** O overlay deixa de expandir: a `Surface` fica `actionbar`/`hidden`
    e o click-through segue preservado (RNF-02).
- **US-07 — Histórico em janela própria.** Como usuário, quero revisar sessões
  antigas numa janela própria, movível e redimensionável.
  - **CA-07.1** Um botão **Histórico** alterna esconder/mostrar a janela.
  - **CA-07.2** A lista (continuar/renomear/excluir) e o detalhe em bolhas
    funcionam como antes.
  - **CA-07.3** **Continuar sessão** volta a capturar e o resultado aparece na
    janela de Chat, sem reabrir o painel do overlay.
  - **CA-07.4** Posição e tamanho voltam ao reabrir o app.
- **US-08 — Transporte com um só stop.** Como usuário, quero um único controle de
  start/pause e um único stop, para não me confundir com dois botões de parar.
  - **CA-08.1** Parado: o controle único mostra `▶ Iniciar escuta` e inicia.
  - **CA-08.2** Gravando: mostra `⏸` e pausa; pausado: mostra `▶` e retoma (sem
    zerar o relógio — BR-019).
  - **CA-08.3** Só a pílula de sessão encerra, via "Encerrar sessão? Sim / Não";
    não existe encerramento sem confirmação (revisa a CA-04.5).

## 5. Requisitos não funcionais

- **RNF-01 — Desempenho:** sem `backdrop-filter` por bolha (lição do próprio
  Perssua), bolhas memoizadas e teto de itens em tela (`MAX_SEGMENTS`).
- **RNF-02 — Click-through:** apenas o scroller, os botões e o botão de copiar
  marcam `data-interactive`; o resto continua atravessando o clique
  ([`../mvp/architecture/overlay.md`](../mvp/architecture/overlay.md) §3).
- **RNF-03 — Sem backend novo, sem telemetria** (BR-002, BR-004); nenhuma mudança
  de schema nesta PRD.
- **RNF-04 — Testabilidade** (BR-013): a separação da linha temporal e o merge são
  funções puras com testes; componentes testados com `@testing-library/react`.
- **RNF-05 — Sem asset de terceiros** (BR-008): os ícones são SVG desenhados no
  projeto, não a fonte Material Icons do Perssua.

## 6. Não-objetivos

- **Quebra de fala longa em várias bolhas** (uma bolha por segmento, como o
  Perssua).
- Agrupamento de bolhas consecutivas do mesmo falante.
- Tradução dentro da bolha (o painel de tradução continua separado).
- Busca no histórico, paginação.
- Export: o Markdown continua com uma linha por item da linha temporal.

## 7. Riscos

| Risco | Mitigação |
|---|---|
| Re-render frequente com streaming de prévia | bolha memoizada, sem blur por bolha, teto de itens |
| Texto longo virar bolha gigante | aceito (paridade); rolagem e `max-width: 75%` |
| Sessões antigas com um lado da conversa perdido (ISS-018) | irreversível; documentado em `issues.md` |
| `started_at_ms` no evento de estado quebrar a UI | campo opcional; TS espelhado e testado |

## 8. Métricas de sucesso

- Duas fontes de áudio falando em paralelo ⇒ duas bolhas distintas na ordem certa
  (teste automatizado + verificação manual com mic + sistema).
- Nenhum `warn` de "falha ao persistir segmento" em sessão de dois canais.
- Os valores visuais conferem com a tabela de paridade
  ([`architecture/chat-ui.md`](architecture/chat-ui.md) §3).

---

## 9. Revisão 0.2 — "Transcrição ao Vivo" e sugestão manual por mensagem

> **Escopo:** tasks 10–16 (`CONCLUDED`). O layout vem do **print de referência
> anexado pelo usuário** (2026-10-19); não é uma nova extração do Perssua. O
> contrato técnico vigente fica em
> [`architecture/transcription-panel.md`](architecture/transcription-panel.md);
> a tabela de `chat-ui.md` §3 permanece como evidência histórica da v0.1. A
> **task 15** fechou a concorrência (sugestão em andamento + janela
> aberta/fechada) e o `_tasks.md` tem a ordem real (`13 → 15 → 14`).

### 9.1 Objetivo

1. **Forma fiel ao print:** painel **"Transcrição ao Vivo"**, abas
   **Transcrição | Resumo** em largura total, bolhas de mensagem em **largura
   total** e rodapé com idioma + estado real do motor.
2. **Identidade por canal:** **Microfone** em cor primária/cyan e **Outros** em
   violeta, com a **hora da mensagem** ao lado do nome.
3. **Ação de IA por mensagem:** botão `[✦]` no canto superior direito de cada
   fala finalizada; o clique leva a mensagem como **contexto** para a janela de
   Sugestões.
4. **Sugestão Manual:** no Sugestões, o usuário vê a mensagem selecionada e
   dispara a geração manual; a resposta é marcada como **"Sugestão Manual"**.
5. **Sem concorrência confusa:** uma geração por vez no serviço, manual espera a
   atual, automática não rouba o cartão manual e a janela oculta não perde
   estado.

### 9.2 Problema

- O painel ao vivo ainda usa o contrato da v0.1: bolhas de até **75%**, alinhadas
  à direita ("Você")/esquerda ("Interlocutor"), sem a leitura de cartão do print.
- Não existe caminho para pedir uma sugestão **a partir de uma fala específica**:
  a janela de Sugestões só recebe o contexto recente e a caixa de contexto extra.
- O rodapé do print mostra "Latência: 48ms · WebRTC"; o produto **não** mede essa
  latência e **não** usa WebRTC — não inventar o número.

### 9.3 Casos de uso

- **CU-08 — Ler a conversa no formato do print:** cada fala é um cartão de
  largura total, com nome/cor do canal e hora.
- **CU-09 — Acompanhar a fala em curso:** a prévia aparece como cartão cyan com
  "Falando agora…" e selo **Streaming**.
- **CU-10 — Pedir uma sugestão sobre uma mensagem:** clicar no `[✦]` da fala,
  revisar o contexto no Sugestões e gerar uma sugestão manual.
- **CU-11 — Revisar no histórico:** o detalhe da sessão usa os mesmos cartões e a
  mesma hora; a ação de IA fica só no ao vivo.
- **CU-12 — Clicar durante uma geração:** o pedido manual espera a sugestão em
  andamento e assume o cartão quando chegar; a janela pode estar aberta ou não.

### 9.4 Estórias de usuário e critérios de aceite

#### US-09 — Casca "Transcrição ao Vivo"

Como usuário, quero que o painel da sessão siga o layout do print.

- **CA-09.1** O cabeçalho da aba Transcrição mostra **ícone + "Transcrição ao
  Vivo"**; a aba Resumo mostra **"Resumo"**. O subtítulo `id · status` sai do
  cabeçalho.
- **CA-09.2** A pill de abas ocupa a **largura total**, com duas colunas iguais;
  a aba ativa usa **contorno/texto cyan** (não fundo preenchido). A navegação por
  teclado e o estado persistido (`Settings.panel_tab`) continuam.
- **CA-09.3** O rodapé mostra **"Idioma de Entrada:"** + seletor à esquerda e, à
  direita, **"Whisper {modelo} · {Gravando|Pausado|Sem sessão}"** — sem latência
  inventada e sem "WebRTC".
- **CA-09.4** O layout se mantém em 360×320, sem overflow; só tokens `--oh-*`.
- **CA-09.5** A aba Transcrição **não** mostra a linha de medidores de nível nem
  o botão de texto "Sugestões": o waveform e o atalho para Sugestões ficam no
  notch (BR-032/BR-033). O **grip de arraste** fica no canto superior esquerdo
  de **todas** as janelas de painel, como padrão (BR-064).

#### US-10 — Bolhas em largura total, com identidade e hora

Como usuário, quero ler a conversa em cartões largos e saber quem falou e quando.

- **CA-10.1** Toda bolha preenche a **largura da lista**; não há `max-width:
  75%`, alinhamento à direita/esquerda nem diferença de largura pelo tamanho do
  texto.
- **CA-10.2** **Microfone** usa a cor primária/cyan; **Outros** usa violeta. A
  **hora** aparece ao lado do nome em `HH:MM:SS` (hora de parede derivada de
  `started_at_ms + at_ms`); sem `started_at_ms`, cai em `mm:ss`.
- **CA-10.3** A fala em andamento mostra **"Falando agora…"**, selo **Streaming**
  com ponto cyan, borda/fundo cyan e caret; **não** mostra a ação de IA.
- **CA-10.4** A continuação de fala longa (BR-031) não repete nome/hora/ações e
  fica colada na parte anterior.
- **CA-10.5** O histórico usa o mesmo contrato visual e a mesma hora; a ação de
  IA não aparece nele.
- **CA-10.6** A tradução sob demanda (BR-030) continua no canto superior direito,
  ao lado do espaço da IA, sem sobrepor o texto.

#### US-11 — Ação de IA em cada mensagem

Como usuário, quero pedir uma sugestão sobre uma fala específica.

- **CA-11.1** Cada fala **finalizada** tem um botão `[✦]` no canto superior
  direito, com `aria-label`/`title`.
- **CA-11.2** O clique envia **sessão, falante, texto inteiro do segmento e
  instante** ao núcleo, abre/foca a janela de **Sugestões** e **não** chama o
  LLM.
- **CA-11.3** Prévia em andamento, continuação, bolha de sugestão e ausência de
  sessão não oferecem a ação.
- **CA-11.4** O contexto é substituído a cada novo clique; se a janela estava
  fechada, ele continua disponível ao abrir (store + busca no `panel_opened`).
- **CA-11.5** O botão é acessível por teclado e tem estado de foco visível.

#### US-12 — Sugestão Manual no Sugestões

Como usuário, quero gerar uma sugestão a partir da mensagem que selecionei.

- **CA-12.1** O Sugestões mostra um bloco **"Contexto manual"** com falante, hora
  e trecho, mais o botão **"Gerar sugestão manual"**. Só o clique do usuário
  chama o LLM (sem custo surpresa ao abrir).
- **CA-12.2** A sugestão gerada é marcada como **"Sugestão Manual"** no cartão e
  no histórico, preservando as fases rápida/completa (BR-038). Ela é efêmera;
  "Fixar" continua sendo o caminho para a linha temporal/export.
- **CA-12.3** O contexto chega ao caso de uso como **parâmetro explícito**
  (`SuggestionRequest.manual_context`), não como estado global de UI. O
  automático do core **não** usa o contexto manual.
- **CA-12.4** O ✕ limpa o contexto; trocar de sessão não devolve contexto de
  outra; o bloco não some no estado vazio.
- **CA-12.5** Sem chave/erro do provider, a mensagem amigável existente aparece e
  o contexto continua disponível para nova tentativa.
- **CA-12.6** Nenhuma migração de banco e nenhum backend novo (BR-002/BR-004).

#### US-13 — Sugestão em andamento e estado da janela

Como usuário, não quero que uma sugestão manual se misture com a que já está
sendo gerada, nem perder o que estava na tela ao abrir/fechar a janela.

- **CA-13.1** **Single-flight no serviço:** no máximo uma geração
  (`quick`/`full`) por vez (janela × agendador × manual). O automático e a
  manual não emitem deltas intercalados; a segunda chamada entra na vez.
- **CA-13.2** A manual **não cancela** o LLM em voo: o botão mostra
  **"Aguardando a sugestão em andamento…"** e fica desabilitado até a geração
  atual terminar. Sem duas chamadas concorrentes.
- **CA-13.3** Com contexto manual ativo, a sugestão **automática** vai para o
  **histórico** e não substitui o cartão manual; a manual tem prioridade de
  leitura. Ao limpar o contexto, o automático volta a poder ocupar o cartão.
- **CA-13.4** A janela de Sugestões continua **montada quando oculta**: os
  eventos gerados enquanto ela estava aberta são acumulados e reabrir mostra o
  estado. **Abrir nunca gera** (BR-028 revisada): a abertura apenas consulta
  `suggestionLast` (cache, sem LLM) para hidratar o cartão; o automático só
  gera com a janela aberta e após o intervalo, e o botão “Sugerir agora”
  continua sendo a ação explícita.
- **CA-13.5** Um novo clique no `[✦]` durante uma manual substitui o contexto;
  a manual antiga termina rotulada com seu **source snapshot** (falante/hora da
  mensagem original) e o contexto novo fica pronto para a próxima geração.
- **CA-13.6** Trocar de sessão limpa a leitura e ignora eventos/resultados da
  sessão antiga; a geração antiga pode terminar no serviço, mas não contamina a
  nova sessão.
- **CA-13.7** Ocultar a janela durante a geração manual não perde o resultado.
- **CA-13.8** Cada geração tem um `generation_id` (UUID) compartilhado por
  `quick`/`full`/deltas; a UI usa o id como chave, então duas gerações no mesmo
  `at_ms` (relógio fixo em teste) não se confundem.

#### US-14 — Linha temporal intercalada quando os canais se sobrepõem

Como usuário, quero que minha fala entre no meio da fala contínua do outro lado,
em vez de aparecer depois dela.

- **CA-14.1** Se a fala de um canal começa **antes** e termina **depois** do
  início da fala do outro, a fala longa é cortada no início da fala nova.
- **CA-14.2** O texto é dividido proporcionalmente ao tempo, preferindo fim de
  frase e, depois, fim de palavra; se não houver corte seguro, a fala fica
  inteira (nada é perdido).
- **CA-14.3** A primeira parte mantém o tempo original; a fala nova entra no
  tempo dela; o restante entra **1 ms depois**, como uma nova bolha com rótulo e
  hora (não como continuação silenciosa).
- **CA-14.4** Vale nos dois sentidos (Microfone/Outros) e no **histórico**, que
  passa a carregar o `end_ms` da fala.
- **CA-14.5** O grip de arraste fica visível no **canto superior esquerdo** de
  todas as janelas de painel (Chat, Sugestões, Histórico), com o header padrão
  da BR-064; **o header inteiro arrasta** a janela, exceto os controles.

### 9.5 Requisitos

**Funcionais**

- **RF-01** Cabeçalho, abas e rodapé do print.
- **RF-02** Bolhas full-width com rótulos Microfone/Outros, hora e Streaming.
- **RF-03** Botão de IA por fala finalizada + handoff de contexto.
- **RF-04** Bloco de contexto manual + geração manual no Sugestões.
- **RF-05** `Suggestion.trigger` (`auto | now | manual`) e rótulo "Manual".
- **RF-06** Contexto por parâmetro no `SuggestionRequest`; auto-sugerir imune.
- **RF-07** `Suggestion.source` (snapshot da mensagem manual),
  `generation_id` por geração e **single-flight no serviço**; prioridade do
  manual no cartão.
- **RF-08** Corte da fala longa no início do outro falante, com intercalação no
  ao vivo e no histórico; grip padrão no canto direito.

**Não funcionais**

- **RNF-01 — Testabilidade (BR-013):** `formatMessageClock`, store do contexto,
  prompt manual, `trigger` e regras do hook são puros/testáveis.
- **RNF-02 — Desempenho:** sem blur por bolha; a lista segue memoizada
  (`ChatView`), com teto de itens (BR-021).
- **RNF-03 — Sem métrica falsa:** o rodapé não mostra latência/WebRTC; só dados
  reais.
- **RNF-04 — Acessibilidade:** `aria-label`, foco visível, teclado.
- **RNF-05 — Sem dependência nova** (BR-008) e **sem schema**.
- **RNF-06 — Documentação única (BR-016):** IDs `BR-040`/`BR-041`/`BR-042`,
  `T10–T15`; contrato em `transcription-panel.md`.
- **RNF-07 — Sem concorrência de LLM:** o `gate` do serviço serializa as
  gerações (uma por vez); `pin` não o usa; a ordem `gate → last` evita deadlock.

### 9.6 Não-objetivos

- Gerar a sugestão **automaticamente** no clique do `[✦]` (o usuário decide no
  Sugestões; evita custo/LLM surpresa).
- Persistir o contexto manual entre reinícios do app.
- Instrumentar latência real de decodificação ou trocar o rótulo técnico do
  rodapé por "WebRTC".
- **Cancelar um LLM em voo** para a manual “furar a fila”: a manual espera a
  geração atual (task 15). Cancelamento/abort seria uma task própria.
- **Timestamps por palavra** no corte de sobreposição: o corte é proporcional ao
  tempo com fim de frase/palavra (task 16); a fala inteira continua no ASR/banco.
- Mudar o teto de 180 caracteres/continuação (BR-031), a linha temporal (BR-021)
  ou a captura de áudio.
- Ação de IA no histórico de sessões encerradas.
- i18n além de PT-BR (BR-010).

### 9.7 Riscos

| Risco | Mitigação |
|---|---|
| Evento de contexto perdido com a janela fechada | store no núcleo + busca ao montar e no `panel_opened`; não depender de evento efêmero |
| Contexto velho de outra sessão | store keyed por `session_id`; `get` só devolve se bater; troca de sessão limpa |
| Injeção de prompt pelo texto da mensagem | o texto vai num bloco `<selected_message>` e o system prompt já trata fala como fonte não confiável (nunca instrução) |
| Custo de LLM ao abrir o Sugestões | com contexto manual, a janela **não** gera automaticamente; só o botão gera |
| Automático e manual emitindo `quick`/`full` intercalados | `gate` do serviço (single-flight); a manual espera a atual |
| Automático substituir a manual que o usuário está lendo | com contexto manual ativo, a automática vai para o histórico; a manual tem prioridade |
| Reabrir a janela disparar geração | abrir não gera (BR-028 revisada); `suggestionLast` hidrata sem LLM |
| Novo `[✦]` durante a manual perder a origem | `Suggestion.source` (snapshot leve: falante/hora/preview) rotula o resultado antigo |
| Gate de geração causar deadlock | ordem fixa `gate → last`; `pin` não usa o gate |
| Bolha full-width com texto longo | teto de 180 caracteres + continuação (BR-031); scroll do painel |
| Corte proporcional da sobreposição cair no meio da frase | preferir fim de frase/palavra; se não houver corte seguro, manter a fala inteira (nada é perdido) |
| Hora de parede sem `started_at_ms` | fallback determinístico para `mm:ss` |
| Dois ícones no canto superior direito (tradução + IA) | grupo de ações com espaçamento; o texto nunca fica por baixo |

### 9.8 Métricas de sucesso

- Nenhum cartão do ao vivo tem `max-width: 75%` ou alinhamento lateral.
- Microfone/Outros e a hora aparecem em todas as falas; a prévia mostra
  Streaming.
- Clicar no `[✦]` abre o Sugestões com o contexto **sem** chamada ao LLM; o
  clique em "Gerar sugestão manual" gera e rotula **Manual**.
- O automático do core continua com `trigger = auto` mesmo com contexto manual
  ativo.
- O serviço nunca tem dois streams simultâneos; a manual clicada durante o
  automático espera e gera depois.
- Reabrir a janela com sugestão carregada/em andamento não faz nova chamada ao
  LLM; a automática que chega durante o contexto manual aparece no histórico.
- `cargo test`, `cargo clippy`, `cargo fmt --check`, `pnpm test`, `tsc --noEmit`
  e `pnpm build` verdes.
