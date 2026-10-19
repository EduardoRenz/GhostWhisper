# PRD — Sugestões: novo layout, identidade dos agentes e usabilidade

- **Versão:** 1.0
- **Data:** 2026-10-19
- **Status:** Implementado (tasks 01–07 concluídas)
- **Relacionados:** [`../business-rules.md`](../business-rules.md),
  [`memory.md`](memory.md), [`issues.md`](issues.md),
  [`architecture/suggestions-window.md`](architecture/suggestions-window.md),
  [`architecture/agent-identity.md`](architecture/agent-identity.md),
  [`tasks/_tasks.md`](tasks/_tasks.md), PRDs anteriores:
  [`../assistant/prd.md`](../assistant/prd.md) (janela de sugestões e agentes),
  [`../notch/prd.md`](../notch/prd.md) (notch, drill-hover e janela `dwell`),
  [`../mvp/prd.md`](../mvp/prd.md)

---

## 1. Objetivo

Reescrever a **janela de Sugestões** (`suggestions`) com o layout de referência e
vários ajustes de usabilidade, sem quebrar o que já funciona:

1. **Novo layout em blocos** — cabeçalho com icone da faísca, badge **LIVE**,
   **cadência visível** do auto-sugerir (progress circular) e **Sugerir agora**;
   seletor de agente em largura total; **bloco de resposta rápida** separado do
   **bloco de resposta completa**; **histórico em accordion** que não substitui o
   topo.
2. **Identidade visual do agente** — cada agente ganha **ícone** e **cor**
   distintas (domínio + settings + UI).
3. **Modelo de sugestão em duas fases** — a resposta **rápida** (curta, gerada
   primeiro) e a **completa** (Markdown) coexistem, em vez de uma substituir a
   outra.

Paridade de comportamento com o **Perssua 0.27.5** (BR-015), sem copiar assets
(BR-008).

## 2. Problema

Hoje a janela de Sugestões é um formulário em volta de um texto:

- O cabeçalho é um `<select>` de agente + 4 botões de texto (“Auto”, “Sugerir
  agora”, “Fixar”, “Copiar”); **não há** ícone do agente, **não há** cor, **não
  há** qualquer sinal do **quando** a próxima sugestão automática vai acontecer.
- O agente é uma lista suspensa genérica: o usuário não distingue “Reunião” de
  “LeetCode” num relance.
- O texto do *fast-first* (rascunho curto) **substitui** o texto completo em
  seguida: o usuário vê a resposta curta sumir e não sabe o que aconteceu.
- A resposta vem como um bloco de Markdown solto, sem título, sem metadados e sem
  a separação entre o que dá para **dizer agora** e o material de apoio.
- O histórico é um `<details>` com uma linha por sugestão; clicar **troca** a
  sugestão do topo (o histórico “rouba” a leitura).

## 3. Público-alvo e casos de uso

- **CU-01 — Ler a resposta em 2 segundos:** durante uma reunião, ver a
  **resposta rápida** (o que falar agora) separada do material completo.
- **CU-02 — Saber quando vem a próxima:** ver o círculo de contagem regressiva e
  entender que o auto-sugerir está rodando (ou pausado).
- **CU-03 — Trocar de agente com contexto:** identificar o agente pelo ícone e
  pela cor, e trocar por um popover de largura total.
- **CU-04 — Reler o passado sem perder o presente:** expandir uma sugestão
  antiga e rolar até ela, mantendo a sugestão atual no topo.
- **CU-05 — Identificar quem respondeu:** a pill do agente no título da sugestão
  e em cada item do histórico.

## 4. Layout de referência (blocos)

| Bloco | Conteúdo | Observação do usuário |
|---|---|---|
| **Header** | ícone da faísca (o mesmo do botão do notch) + título **“Sugestões”** + badge **LIVE** | título pode ser só “Sugestões”; ícone do print é só referência |
| **Header — auto** | **círculo** com progress circular regressivo + rótulo (contagem regressiva rodando / “Pausado” pausado) + ação de pausar | primeiro botão da direita; ao zerar, **auto sugere** |
| **Header — agir** | **“Sugerir agora”** | **sem** glow (ao contrário do print) |
| **Seletor de agente** | botão de **largura total**: ícone + nome do agente + pill **ATIVO** + chevron | popover com a lista; nada muda de tamanho conforme o print — fica fluido |
| **Sugestão atual** | tag “Sugestão Atual · Há 12s” + **título** da sugestão + **pill do agente** | a pill informa **qual agente respondeu** |
| **Resposta rápida** | texto curto e direto (1ª pessoa, fala imediata) | título, cor e estilo **próprios** |
| **Resposta completa** | Markdown livre (explicação, código, complexidade) | título, cor e estilo **próprios**; chip de metadados (“Markdown · Explicativo”) |
| **Histórico** | seção colapsável **“Histórico de Sugestões da Sessão”** (ícone + contador “2 anteriores” + chevron); cada item: **ícone do agente** + título + **pill com o nome do agente** | expandir um item **rola até** o bloco expandido; **nunca substitui** o resumo do topo. O editor do protótipo ficava **por cima** do histórico — não repetir |

## 5. Histórias de usuário e critérios de aceite

### US-01 — Cabeçalho com identidade, LIVE e cadência visível

Como usuário, quero ver que a janela está viva e quando a próxima sugestão
automática vai acontecer.

- **CA-01.1** O cabeçalho mostra o **ícone da faísca** (o mesmo do botão
  “Sugestões” do notch), o título **“Sugestões”** e o badge **LIVE**.
- **CA-01.2** O primeiro controle da direita é o **auto-sugerir**: um **círculo**
  com **progress circular regressivo** que esvazia a cada janela automática.
- **CA-01.3** Com o automático **ligado**, o rótulo mostra a **contagem
  regressiva** do **timer que já existe** no agendador (ex.: “12s”) e o círculo
  esvazia; ao zerar, o auto-sugerir dispara (se houver fala nova) e a contagem
  reinicia.
- **CA-01.4** Sem fala nova, o círculo fica vazio e o rótulo diz **“aguardando
  fala”** (a sugestão só nasce com conteúdo novo).
- **CA-01.5** Com o automático **pausado**, o círculo para e o rótulo diz
  **“Pausado”**; o controle permite **retomar**.
- **CA-01.6** **“Sugerir agora”** fica ao lado, **sem glow**, desabilitado
  enquanto uma geração está em curso.
- **CA-01.7** A cadência vem do **core** (única fonte da verdade): o automático
  continua **igual** — mesmo debounce, mesmo intervalo das settings — e a UI apenas
  **mostra** o timer que já existe (feedback visual), sem recalcular nada.

### US-02 — Agente com ícone, cor e seletor em largura total

Como usuário, quero reconhecer o agente pelo ícone e pela cor, e trocar de agente
num popover claro.

- **CA-02.1** Todo agente (embutido **e** personalizado) tem **ícone** e **cor**
  próprios; os embutidos vêm fixos e os personalizados têm ícone e cor
  escolhidos (paleta do design system).
- **CA-02.2** O seletor é um **botão de largura total** (ícone + nome do agente
  selecionado + pill **ATIVO** + chevron) que abre um **popover** com a lista de
  agentes (ícone + nome + cor).
- **CA-02.3** O popover é **fluido**: acompanha a largura da janela, sem largura
  fixa que “pule” para fora.
- **CA-02.4** A escolha continua **persistida** (`settings.assistant_mode_id`) e
  vale para o automático do core.
- **CA-02.5** Agentes personalizados podem ser criados/duplicados na
  Configurações já com ícone e cor.

### US-03 — Resposta rápida e resposta completa separadas

Como usuário, quero ver a resposta curta **e** o material completo, sem que um
substitua o outro.

- **CA-03.1** A sugestão passa a ter **duas fases**: a **rápida** (curta, gerada
  primeiro) e a **completa** (Markdown, gerada em seguida).
- **CA-03.2** A **resposta rápida** aparece em bloco próprio, com **título, cor e
  estilo próprios** (citação/ênfase), e permanece visível quando a completa chega.
- **CA-03.3** A **resposta completa** aparece em bloco próprio, com **título, cor
  e estilo próprios**, e é **Markdown livre** (listas, tabelas, código).
- **CA-03.4** A completa traz um **chip de metadados** (ex.: “Markdown ·
  Explicativo”) e os blocos de código com **cabeçalho** (linguagem — arquivo) e
  **rodapé** (complexidade/tempo), quando o modelo fornecer.
- **CA-03.5** A sugestão atual tem **título** curto, **derivativo** (primeiro
  heading/linha do texto), **reusando o padrão que já existe** para o resumo
  (`subject`/`summaryTopic`), com rótulo neutro no fallback — **sem** campo novo
  no modelo e **sem** mudar o contrato de prompt.
- **CA-03.6** A resposta rápida é **efêmera** (não vai para a linha temporal); a
  completa continua sendo o que é **fixado/copiado/exportado**.

### US-04 — Histórico em accordion que não rouba o topo

Como usuário, quero reler uma sugestão antiga sem perder a atual.

- **CA-04.1** O histórico é uma **seção colapsável** “Histórico de Sugestões da
  Sessão”, com ícone, **contador** (“2 anteriores”) e chevron; dentro dela, cada
  sugestão é um **accordion** com o **ícone do agente**, o **título** e a **pill com
  o nome do agente** que respondeu.
- **CA-04.2** **Expandir** um item abre o conteúdo **no lugar** e faz o scroll
  **ir até** o bloco expandido; a sugestão atual **permanece no topo** (nada é
  substituído).
- **CA-04.3** O item expandido mostra a resposta completa e as ações
  (copiar/fixar) daquela sugestão.
- **CA-04.4** O histórico é **limitado** (memória da janela) e se reinicia quando
  a sessão muda.
- **CA-04.5** A pill do agente no histórico mostra o **ícone e a cor** do agente
  **no momento em que respondeu** (snapshot), mesmo que o agente atual seja outro.
- **CA-04.6** Nenhum outro componente (ex.: um futuro editor) fica **por cima**
  do histórico: o bloco tem espaço próprio e o scroll é do corpo.

### US-05 — Ajustes de usabilidade da janela

- **CA-05.1** Ações de **copiar/fixar** viram botões compactos (ícone) no
  cartão da sugestão, em vez de botões de texto no cabeçalho.
- **CA-05.2** A sugestão atual mostra o **tempo relativo** (“Há 12s”) e se
  atualiza sozinha.
- **CA-05.3** “Contexto extra” continua disponível, **recolhido** por padrão.
- **CA-05.4** Estados vazios/erro/gerando seguem o novo layout (blocos vazios
  com mensagem), sem “pulo” de layout.
- **CA-05.5** A janela continua **sem moldura, móvel e redimensionável**, com o
  layout fluido (largura mínima 520 px para o código caber).

## 6. Requisitos

### Funcionais (RF)

- **RF-01** Cabeçalho novo (faísca + título + LIVE + cadência circular + Sugerir
  agora).
- **RF-02** Seletor de agente em popover, com ícone e cor.
- **RF-03** Sugestão com **título + resposta rápida + resposta completa**.
- **RF-04** Histórico em accordion com scroll até o bloco expandido.
- **RF-05** Identidade (ícone + cor) no domínio de agentes.

### Não funcionais (RNF)

- **RNF-01 — Testabilidade (BR-013):** funções puras para fase da sugestão,
  identidade do agente, rótulo de contagem e cálculo do progress circular.
- **RNF-02 — Desempenho:** o círculo de contagem é CSS/SVG (sem re-render por
  quadro); o poll da cadência é leve (1/s) e só com a janela aberta.
- **RNF-03 — Sem dependência nova:** ícones SVG do projeto (BR-008); sem
  biblioteca de UI, sem fonte externa.
- **RNF-04 — Compatibilidade:** settings antigos (agentes sem ícone/cor) continuam
  carregando (defaults) e o schema do banco não muda.
- **RNF-05 — Acessibilidade:** foco visível, `aria-label` em todo controle,
  popover navegável por teclado, contagem lida por leitor de tela.
- **RNF-06 — Documentação única (BR-016):** IDs `BR-037`…`BR-039`, `ISS-037`+,
  `NN-task`; sem duplicar o que já está no PRD assistant.

## 7. Não-objetivos

- **Composer/chat** na base da janela (“o que você gostaria de mudar ou criar?”)
  do print — é outra feature (refinar/regenerar com linguagem natural) e fica
  para uma PRD própria. No protótipo ele ainda **cobre** o histórico; aqui o
  histórico tem espaço próprio (CA-04.6).
- **Streaming da resposta completa** com keep-alive do texto (o modelo atual
  substitui o texto a cada delta) — sem mudança agora.
- **Persistir a resposta rápida** na linha temporal/export.
- **Tema claro**, **i18n** além de PT-BR (BR-010).
- Mover a janela de Sugestões para dentro do painel em abas (a janela é
  própria, como no notch).

## 8. Riscos

| Risco | Mitigação |
|---|---|
| **Prompt novo** quebra o agente (resposta rápida + completa) | o contrato do agente é **estendido** (não trocado): a rápida é a mesma resposta curta de hoje, agora com **fase**; o completo segue o Markdown atual + metadados de código |
| Contagem regressiva **dessincroniza** do automático real | a contagem vem do **core** (mesma fonte da decisão), não da UI |
| Duas chamadas de LLM por sugestão (rápida + completa) | a rápida é curta (teto de tokens baixo) e o completo reutiliza o contexto; opcionalmente a completa pode incluir a rápida |
| Agentes antigos sem ícone/cor | defaults por id + `serde(default)`; settings antigos carregam |
| Histórico crescendo | limite de memória (como hoje) e scroll só até o item, sem recarregar a lista |
| Popover do agente cortando a janela | popover **fluido** e ancorado no botão, com reposicionamento por borda |

## 9. Métricas de sucesso

- A resposta rápida **permanece** visível quando a completa chega.
- O usuário identifica o agente **sem ler o nome** (ícone + cor).
- O histórico **nunca** substitui a sugestão do topo.
- `cargo test`, `cargo clippy`, `cargo fmt --check`, `pnpm test`, `tsc --noEmit` e
  `pnpm build` verdes.

## 10. Evidência no Perssua (BR-015)

O print de referência é do protótipo **Stich AI** (componente de sugestão
inteligente do HUD). Vamos transpor **comportamento e hierarquia**, não assets
nem código (BR-008): cabeçalho com badge LIVE, controle de auto-sugerir com
progress circular, seletor de agente, blocos de resposta rápida/completa com
estilos próprios e histórico em accordion.

Evidência no bundle do `Perssua-0.27.5-x86_64.AppImage` (asar em
`/tmp/opencode/perssua/squashfs-root/resources/app.asar`) será registrada **uma
vez** em [`architecture/suggestions-window.md`](architecture/suggestions-window.md)
§7, na fase de implementação (R14/BR-016).
