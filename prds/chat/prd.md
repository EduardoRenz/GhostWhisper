# PRD — GhostWhisper Chat (paridade de conversa com o Perssua)

- **Versão:** 0.1
- **Data:** 2026-10-18
- **Status:** Implementado (tasks 01–06 `CONCLUDED`); 07–09 `PENDING`
- **Relacionados:** [`../business-rules.md`](../business-rules.md),
  [`memory.md`](memory.md), [`issues.md`](issues.md),
  [`architecture/chat-ui.md`](architecture/chat-ui.md),
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
