# PRD — Modo de Sugestão: automático vs. classificador

- **Versão:** 0.3 (implementado)
- **Data:** 2026-10-26
- **Status:** Implementado (MVP) — Laya POC abaixo do gate (heurístico é o padrão); roteiro manual pendente
- **Relacionados:** [`../business-rules.md`](../business-rules.md),
  [`memory.md`](memory.md), [`issues.md`](issues.md),
  [`architecture/suggestion-gating.md`](architecture/suggestion-gating.md),
  [`architecture/fine-tuning.md`](architecture/fine-tuning.md),
  [`tasks/_tasks.md`](tasks/_tasks.md) · PRDs anteriores:
  [`../suggestions/prd.md`](../suggestions/prd.md) (janela e cadência),
  [`../notch/prd.md`](../notch/prd.md),
  [`../precision-and-performance/prd.md`](../precision-and-performance/prd.md),
  [`../mvp/prd.md`](../mvp/prd.md)

---

## 1. Objetivo

Criar o **modo de sugestão** do GhostWhisper: decidir **quando** o copiloto deve
intervir, e não apenas **o que** falar.

O produto passa a ter duas estratégias explícitas, escolhidas pelo usuário:

1. **Automático (timer)** — comportamento atual: a cada intervalo, com fala nova
   e com a janela aberta, gera uma sugestão.
2. **Classificador (agente)** — a cada **bloco de conversa** fechado (com ME e
   THEY), um classificador decide se há motivo para sugerir e qual é o motivo:
   pergunta, hesitação, perda do fio, convite para responder, silêncio, nada a
   sugerir ou contexto insuficiente.

Além disso:

- um **bloco “Modo de ativação de sugestões”** no topo de **“Fontes de IA &
  Modelos”** concentra os dois cards (classificador vs. tempo), modelo,
  sensibilidade e cadência; o avançado (blocos, categorias, limiares e
  diagnóstico) fica em drawer/modal;
- o **cabeçalho da janela de Sugestões** muda conforme o modo (timer vs. status
  do classificador);
- uma **faixa de “Última classificação”** fica sempre visível na janela de
  Sugestões, mesmo quando a decisão foi **não sugerir**;
- uma **POC com o Laya** valida o classificador antes de torná-lo padrão.

**O modo automático continua sendo o padrão.** O classificador é opt-in até a POC
atingir os critérios da §9. Não há troca silenciosa de comportamento.

**Distribuição para usuário final:** o modelo Laya é um artefato ONNX
versionado, baixado pelo próprio app com confirmação, checksum e progresso —
sem Python na máquina do usuário. O mesmo contrato de artefato é o caminho para
um modelo *fine-tuned* no futuro (BR-099/BR-100).

Paridade de comportamento com o **Perssua 0.27.5** (BR-015), sem copiar código
ou assets (BR-008).

---

## 2. Problema

Hoje o auto-sugerir é um **relógio**: a cada `suggestion_interval_secs`, se há
fala nova, ele chama o LLM. Isso tem três consequências:

- **Sugere na hora errada** — uma fala corrida e sem pergunta pode gerar uma
  sugestão que o usuário não pediu.
- **Não percebe os sinais que importam** — pergunta do outro lado, hesitação do
  usuário, silêncio depois de falar, perda do fio da conversa.
- **Não explica a decisão** — quando não sugere, o usuário não sabe se foi falta
  de fala, intervalo, cache ou erro.

O Perssua 0.27.5 já tem mecanismos desse tipo (evidência na §15): regex de
pergunta, gatilho de silêncio `user-help`, classificador de hesitação por LLM e
modos de Notch. O GhostWhisper tem o timer e o *cooldown*, mas **não** tem a
camada semântica nem a transparência da decisão.

Também há um problema de **organização**: a configuração de sugestões está
espalhada na aba “Geral & HUD” (card de cadência), enquanto o produto vai crescer
bastante em modo, backend, blocos, limiares e diagnóstico. Uma aba dedicada evita
que “Geral & HUD” vire uma gaveta.

---

## 3. Decisões de produto

### 3.1 Dois modos + um interruptor mestre

- `Settings.suggestion_auto_refresh` continua sendo o **interruptor mestre**
  (“Sugestões automáticas” ligado/desligado).
- Novo `Settings.suggestion_mode`: `auto` | `classifier`.
- Estados efetivos:
  - **Desligado:** `suggestion_auto_refresh = false`.
  - **Automático:** `true` + `mode = auto`.
  - **Classificador:** `true` + `mode = classifier`.
- O botão de pausar/retomar do cabeçalho continua mexendo **só** no interruptor
  mestre; o modo escolhido é preservado.
- Migração: settings antigos (sem `suggestion_mode`) carregam como `auto`.
  Ninguém acorda com o classificador ligado.

### 3.2 O classificador decide; o gerador escreve

O classificador **não** produz a sugestão. Ele produz uma **cue** (motivo) com
confiança, e o gerador existente continua escrevendo o texto, agora com um
“foco” curto derivado da cue. Isso mantém:

- o classificador barato/rápido (pode ser um encoder local);
- o gerador caro apenas quando há decisão positiva;
- o contrato dos agentes intacto (só ganha contexto extra).

### 3.3 Bloco no topo de “Fontes de IA & Modelos” — aba própria: **não no MVP**

**Recomendação:** não criar uma aba “Sugestões” no MVP. Colocar um **bloco de
modo de ativação** no topo da aba **“Fontes de IA & Modelos”**, onde o usuário
já cadastra chaves e escolhe modelos. O modelo do classificador é, afinal, mais
uma fonte/modelo de IA.

O bloco segue o mockup do usuário:

- header com ícone de faísca, título **“Modo de ativação de sugestões”**, badge
  **“Estratégia de trigger”**, label **“Disparo inteligente em tempo real”** e
  um **interruptor mestre** (“Sugestões automáticas”);
- dois cards selecionáveis:
  1. **Modelo de classificação** — “Laya · Classificador contextual”;
     descrição de disparo por momento oportuno; badge **“Ativa”** quando
     selecionado; campo **Modelo selecionado** (`Laya (Padrão)` / `Regras
     locais`), status do modelo (instalado/não
     instalado/baixar) e **Sensibilidade** (`Baixa` | `Balanceada` | `Alta`);
  2. **Sugestão por tempo** — “Intervalo periódico contínuo”; descrição de
     cadência fixa; campo **Cadência de ciclo** (`8s` | `15s` | `30s`) e nota
     “Disparo cego / sem filtro semântico”. Se o valor salvo for outro (ex.:
     `20s`), mostrar um chip extra com o valor atual para não perder a
     configuração.

O que **não** cabe no cartão do mockup vai para um **drawer/modal “Avançado”**
aberto pelo card do classificador (ícone de engrenagem ou “Gerenciar modelo”):

- model manager (versões, download/checksum/espaço, apagar, trocar);
- cooldown, timeout, fallback;
- blocos (gap, duração, turnos, contexto);
- categorias que geram sugestão;
- sonda de bloco aberto;
- mostrar última classificação;
- painel de teste do classificador.

Motivos:

1. O modo + modelo + sensibilidade são decisão de **fonte de IA**, não de
   aparência.
2. O cartão do mockup fica limpo e cabe em uma tela; o avançado não polui a
   primeira dobra.
3. Evita uma 7ª aba para uma feature que ainda vai mudar muito. Se o “Avançado”
   crescer, promovemos a aba “Sugestões” depois — sem mudar o contrato.
4. O bloco substitui o card de sugestões que hoje vive em “Geral & HUD”; o
   resumo automático continua lá.

A aba deve ser renomeada de **“Fontes de IA”** para **“Fontes de IA & Modelos”**
para refletir o conteúdo. A ordem das abas existentes não muda.

O mockup mostra também uma navegação futura (“Áudio & Microfone”, “Visão de
Tela”, “Atalhos Globais”); este PRD **não** redesenha a navegação inteira — só
move o bloco para a aba de fontes/modelos e renomeia essa aba. O restante fica
fora de escopo.

Alternativa considerada: aba própria “Sugestões”. Fica como evolução se o
“Avançado” virar uma superfície grande; no MVP, o custo de uma aba nova não se
paga.

### 3.4 Cabeçalho adaptativo

O cabeçalho atual (faísca + título + LIVE + anel de cadência + “Sugerir agora”)
passa a ter dois corpos:

| Modo | Controle do auto | O que mostra |
|---|---|---|
| **Automático** | `AutoSuggestControl` atual | anel de progresso + contagem (`12s`), `Pausado`, `Aguardando`, `Gerando`, `Sem escuta` |
| **Classificador** | `ClassifierSuggestControl` | ícone do classificador + rótulo “Classificador” + pill de estado: `Ouvindo bloco…`, `Analisando…`, `Vai sugerir: Pergunta`, `Pergunta: nada a sugerir`, `Pausado`, `Sem escuta`, `Falha` |
| **Desligado** | controle neutro | `Desligado`; “Sugerir agora” continua disponível |

- No modo classificador **não há cronômetro**: o tempo deixa de ser a decisão.
  No lugar, o estado do bloco/classificação.
- O badge **LIVE** continua.
- **“Sugerir agora”** continua em todos os modos e **ignora o classificador**
  (ação explícita do usuário).
- O clique no controle continua pausando/retomando o interruptor mestre.

### 3.5 “Última classificação” sempre visível

Logo abaixo do cabeçalho, uma faixa compacta e expansível mostra a última
classificação da sessão:

- **cue** (Pergunta, Hesitação, Perda do fio, Convite, Silêncio, Nada a sugerir,
  Contexto insuficiente);
- **decisão** (`vai sugerir` / `não sugerir`), com o motivo quando não sugeriu;
- **confiança**;
- **evidência** (trecho curto da última fala que pesou na decisão);
- **bloco** (intervalo de tempo e nº de turnos);
- **backend/modelo/latência**;
- **quando** (“há 8s”);
- ações **“Classificar agora”** (diagnóstico) e **“Tentar de novo”** (erro).

Estados:

- **Classificador nunca rodou:** “Nenhuma classificação ainda. O classificador
  roda quando um bloco fecha com contexto suficiente.”
- **Modo automático:** “Modo automático: o classificador está desligado.” + link
  para Configurações.
- **Erro/timeout:** “Classificador indisponível: …” + “Tentar de novo”.
- **Decisão negativa:** a faixa mostra a categoria e o motivo; **não** abre uma
  sugestão.

A faixa é **sempre renderizada** por padrão (`suggestion_show_classification =
true`), inclusive quando não há sugestão atual. Ela pode ser ocultada nas
Configurações, mas não é escondida automaticamente.

### 3.6 Categorias de decisão

| Cue | Significado | Gera sugestão? |
|---|---|---|
| `question` | THEY fez uma pergunta que ME ainda não respondeu | Sim, se confiança ≥ limiar |
| `comment` | THEY fez um ponto/opinião e convida ME a comentar | Sim, se confiança ≥ limiar |
| `already_answered` | A pergunta repete uma já respondida no histórico recente (ou ME já respondeu no bloco) | **Não** |
| `no_action` | Conversa casual, explicação, cumprimento ou fala sem pergunta/convite | **Não** |
| `insufficient_context` | Bloco curto/ambíguo ou classificador absteve | **Não** |

- **Não existe mais** `hesitation`, `silence`, `lost_thread` nem `invitation`.
- O usuário pode **desligar** `question` e/ou `comment`.
- `already_answered`, `no_action` e `insufficient_context` **nunca** geram.
- A confiança é comparada com o limiar derivado da **Sensibilidade**:

| Sensibilidade | Limiar (heurístico) | Limiar (Laya) | Cues habilitadas |
|---|---|---|---|
| **Baixa** | 0,85 | 0,75 | `question`, `comment` |
| **Balanceada** (padrão) | 0,70 | 0,50 | `question`, `comment` |
| **Alta** | 0,55 | 0,35 | `question`, `comment` |

- O classificador recebe o **contexto do agente selecionado** (nome, categoria,
  contexto fixo e foco) e as **últimas 5 perguntas já respondidas** (título
  curto). O histórico é configurável em 0–20 (`suggestion_answered_questions`).
- O “Avançado” pode **desligar** cues individualmente; a allowlist efetiva é
  `preset da sensibilidade ∩ toggles do usuário`. O limiar exato é mostrado como
  leitura, não como segundo campo editável (fonte única = sensibilidade).

### 3.7 Distribuição do modelo para usuário final

O classificador não pode depender de Python, `laya-serve` ou de um checkout do
repositório do Laya na máquina do usuário. A linha de produto é:

- **Artefato ONNX versionado** (`model.onnx`, `model.onnx.data`,
  `rl_agent_config.json`, `tokenizer/` + `manifest.json`), publicado como
  *release asset* com **SHA-256** e licença Apache-2.0.
- **Model manager no app**: baixar sob confirmação, verificar checksum, instalar
  de forma atômica, mostrar progresso/erro/espaço em disco, permitir apagar e
  trocar de versão.
- **Offline-first**: modelo já instalado carrega sem rede; se não houver modelo
  nem rede, o app cai no backend heurístico e explica o motivo.
- **Sem download silencioso**: nada de 1,2 GB saindo da máquina/rede sem o
  usuário mandar.
- **Mesmo contrato para fine-tuning**: um modelo ajustado é só um novo artefato
  ONNX com o mesmo `manifest.json` e uma `model_version`; o app troca de modelo
  sem mudança de código.

O caminho de desenvolvimento (exportar o ONNX localmente com Python) continua
existindo, mas é **fallback de dev**, não o fluxo do usuário final.

---

## 4. Público-alvo e casos de uso

- **CU-01 — Pergunta do outro lado:** THEY pergunta algo; o classificador marca
  `question`; a sugestão aparece com foco na resposta.
- **CU-02 — Ponto a comentar:** THEY defende uma opinião e pergunta o que ME
  acha; o classificador marca `comment` e a sugestão comenta o ponto.
- **CU-03 — Pergunta já respondida:** THEY repete uma pergunta já respondida
  nas últimas 5; o classificador marca `already_answered` e não gasta LLM.
- **CU-04 — Conversa casual/explicação:** o bloco é small talk ou alguém
  explicando; o classificador marca `no_action` e a faixa mostra isso.
- **CU-05 — Contexto curto:** o bloco não tem conteúdo suficiente; o
  classificador marca `insufficient_context` e não gera.
- **CU-06 — Diagnóstico/POC:** o usuário abre Fontes de IA & Modelos → card
  “Modelo de classificação” → Avançado, testa um bloco, vê
  cue/confiança/latência e decide se o backend está bom.
- **CU-07 — Troca de estratégia:** o usuário alterna entre automático e
  classificador sem perder a sessão nem o agente selecionado.

---

## 5. Fluxos

### 5.1 Automático (preservar)

```
tick 1s → janela aberta? → cooldown venceu? → há fala nova?
  → publica cadência (anel/contagem)
  → se tudo ok: generating=true → gera sugestão → publica
```

- Nenhuma chamada ao classificador.
- A faixa de última classificação mostra o estado “modo automático”.

### 5.2 Classificador (novo)

```
tick 1s
  → carrega settings/sessão
  → monta blocos a partir das falas finalizadas
  → há bloco fechado novo (com ME e THEY)?
      → classifica (timeout configurável)
      → publica "suggestion_classification"
      → guarda a última classificação da sessão
      → decisão positiva + cooldown ok + sem geração em curso?
          → gera sugestão com cue/foco
      → decisão negativa → apenas atualiza a faixa
```

- **Sem cronômetro**: o bloco fechado é o evento.
- **Cooldown** (`classifier_cooldown_secs`, padrão 15 s) é apenas um teto de
  frequência; não é a decisão.
- **Deduplicação:** o mesmo `block_id` não gera duas sugestões.
- **Cooldown vencido com decisão positiva:** a decisão fica pendente e gera
  quando o cooldown zerar — sem reclassificar o mesmo bloco.
- **Bloco novo antes do cooldown:** substitui a decisão pendente; se o novo
  bloco for negativo, a pendência é limpa.

### 5.3 Histórico e contexto ampliado

Não existe mais sonda de bloco aberto. O gate lê no mínimo **8 blocos
anteriores** (até 5 min/120 segmentos) e mantém, por sessão, os títulos das
perguntas que geraram sugestão. Uma pergunta repetida vira `already_answered`
sem chamar o gerador.

### 5.4 Decisão negativa

- Publica a classificação com `should_suggest = false`.
- Não chama o gerador.
- A faixa mostra cue + confiança + motivo.
- Não reinicia o cooldown por si só; o próximo bloco pode gerar.

### 5.5 Erro, timeout e abstenção

- **Timeout** → `insufficient_context` + estado de erro na faixa; por padrão
  **não** gera (fallback `skip`).
- **Backend indisponível** → estado de erro; por padrão **não** gera.
- **Abstenção do Laya** (`abstention`) → `insufficient_context`.
- **Fallback `auto`** (opcional): se o classificador falhar, cai no
  comportamento automático do timer. Padrão: `skip`, para não voltar a sugerir
  sem critério.
- O erro nunca apaga a última classificação válida; a faixa mostra a mais
  recente com um selo de “desatualizada”.

### 5.6 Manual

- **“Sugerir agora”** e o atalho global **ignoram** o classificador e o cooldown.
- A sugestão manual **não** altera a última classificação (a faixa continua
  mostrando a decisão do classificador).
- O cartão da sugestão mostra um selo discreto da cue quando a origem foi o
  classificador (`Pergunta`, `Hesitação`, etc.).

---

## 6. Requisitos funcionais

- **RF-01** `Settings.suggestion_mode` (`auto` | `classifier`) com migração
  segura; `suggestion_auto_refresh` continua como interruptor mestre.
- **RF-02** Construtor puro de **blocos de conversa** a partir de falas
  finalizadas, com fechamento por silêncio, duração máxima e nº de turnos;
  bloco classificável exige ME e THEY.
- **RF-03** Porta `SuggestionCueClassifier` com implementações:
  `HeuristicSuggestionCueClassifier` (regras locais), `LayaSuggestionCueClassifier`
  (POC) e `Fake` para testes.
- **RF-04** Decisão com `cue`, `should_suggest`, `confidence`, `evidence`,
  `focus`, `backend`, `model`, `latency_ms` e `block_id`.
- **RF-05** Categorias configuráveis, limiar de confiança, cooldown, timeout e
  fallback (`skip` | `auto`).
- **RF-06** Cabeçalho adaptativo (timer no automático; status do classificador
  no classificador; “Desligado” sem modo).
- **RF-07** Faixa de **última classificação** sempre visível (por padrão),
  hidratada ao abrir a janela, atualizada por evento e resetada ao trocar de
  sessão.
- **RF-08** Bloco **“Modo de ativação de sugestões”** no topo de **“Fontes de IA
  & Modelos”**, com os dois cards (classificador vs. tempo) e o avançado em
  drawer/modal. Sem aba “Sugestões” no MVP; o card de sugestões sai de
  “Geral & HUD”.
- **RF-09** O gerador recebe `cue` + `focus` e adapta o prompt (responder
  pergunta, continuar hesitação, retomar fio, responder convite, puxar próxima
  fala no silêncio).
- **RF-10** POC com o SDK Rust **`laya-onnx`** dentro do backend (ONNX local,
  sem Python em runtime), com fixtures, avaliação e gate de adoção.
- **RF-11** Nenhum texto de transcrição, áudio ou token em logs; logs só com
  IDs, tamanhos, cue, confiança, latência e backend.
- **RF-12** Ação manual (“Sugerir agora”) sempre disponível e fora do
  classificador.
- **RF-13** Compatibilidade: settings antigos carregam; modo padrão `auto`;
  nenhuma migração de banco; última classificação é memória de sessão no POC.
- **RF-14** **Model manager** do classificador: baixar o artefato ONNX sob
  confirmação, verificar SHA-256, instalar atomicamente, mostrar progresso,
  espaço em disco, cancelar/repetir, apagar e trocar de versão.
- **RF-15** **Manifesto versionado** do modelo (`manifest.json`) com
  `model_version`, checkpoint, SHA-256, tamanho, licença, calibração e métricas;
  o app carrega qualquer artefato que siga o contrato, inclusive um modelo
  fine-tuned.
- **RF-16** **Offline-first**: sem download silencioso; sem modelo instalado,
  o backend heurístico continua disponível e a UI explica o motivo.

---

## 7. Requisitos não funcionais

- **RNF-01 — Testabilidade (BR-013):** blocos, cues, decisão e agendamento são
  funções puras; cada porta tem dobra de teste (`FakeSuggestionCueClassifier`,
  `InMemoryTranscriptionRepository`, `FakeClock`).
- **RNF-02 — Desempenho:** montar blocos sobre os últimos 180 s/40 segmentos em
  < 5 ms; classificação não bloqueia a UI (thread do agendador); p95 do Laya
  local ≤ 1 s em CPU e ≤ 300 ms em GPU para a pergunta `choice`.
- **RNF-03 — Dependência opt-in:** o Laya entra como o crate `laya-onnx` atrás
  da feature `laya`; o build padrão não o inclui. O modelo ONNX (~1,2 GB) fica
  fora do bundle e é carregado de um caminho configurado.
- **RNF-04 — Privacidade:** os backends do MVP são locais (heurístico/Laya em
  ONNX); nenhum bloco sai da máquina. Se um backend remoto for adicionado no
  futuro, a UI deve avisar. Nada de áudio.
- **RNF-05 — l10n:** toda copy nova em `pt` e `en`, chaves `snake_case`
  (R18–R22, BR-086); nenhum texto hardcoded.
- **RNF-06 — Acessibilidade:** foco visível, `aria-label`, estados da faixa
  anunciados, contraste no badge de cue.
- **RNF-07 — Compatibilidade:** o modo automático não muda de comportamento;
  testes existentes de cadência continuam passando.
- **RNF-08 — Integridade do modelo:** SHA-256 obrigatório, download atômico
  (`.part` → renomear), retomada/cancelamento, sem arquivo parcial carregável.
- **RNF-09 — Offline:** com o modelo instalado, nenhuma chamada de rede; sem
  modelo, o heurístico funciona e a UI mostra “modelo não instalado”.
- **RNF-10 — Licença/atribuição:** o artefato e o SDK são Apache-2.0; a UI
  mostra a licença e o tamanho antes do download; nenhum asset de terceiro é
  redistribuído sem aviso (BR-008).

---

## 8. Histórias e critérios de aceite

### US-01 — Escolher entre automático e classificador

Como usuário, quero escolher como o copiloto decide quando sugerir.

- **CA-01.1** O bloco “Modo de ativação de sugestões” em **Fontes de IA &
  Modelos** tem interruptor mestre e os dois cards (`Classificador` /
  `Tempo`); desligado desabilita os cards sem perder a escolha.
- **CA-01.2** A escolha persiste e vale para a próxima sessão.
- **CA-01.3** Settings antigos abrem em `Automático`.
- **CA-01.4** O botão de pausar/retomar do cabeçalho preserva o modo.

### US-02 — Classificador decide com base em blocos

Como usuário, quero que o copiloto só sugira quando houver um motivo real.

- **CA-02.1** Um bloco só é classificado quando está fechado e tem contexto
  suficiente (ME+THEY no `blocks`; qualquer falante no `adaptive`; só THEY no
  `they_only`, que ignora/projeta os turnos de ME).
- **CA-02.2** `question` e `comment` podem gerar sugestão; `already_answered`,
  `no_action` e `insufficient_context` nunca.
- **CA-02.3** Confiança abaixo do limiar não gera.
- **CA-02.4** O mesmo bloco não gera duas sugestões.
- **CA-02.5** Cooldown é frequência mínima, não a decisão; decisão positiva
  pendente gera ao zerar.
- **CA-02.6** Timeout/erro não geram por padrão e aparecem na faixa.
- **CA-02.7** “Sugerir agora” ignora o classificador.

### US-03 — Ver a última classificação

Como usuário, quero saber o que o classificador decidiu e por quê.

- **CA-03.1** A faixa aparece abaixo do cabeçalho e mostra cue, decisão,
  confiança, evidência, bloco, backend/modelo/latência e tempo relativo.
- **CA-03.2** Sem classificação, mostra o estado vazio explicativo.
- **CA-03.3** No modo automático, mostra “classificador desligado” + atalho para
  Configurações.
- **CA-03.4** Em erro, mostra o erro e “Tentar de novo”.
- **CA-03.5** A faixa nunca esconde a sugestão atual nem rouba o topo.
- **CA-03.6** Trocar de sessão limpa a classificação exibida.

### US-04 — Cabeçalho coerente com o modo

Como usuário, quero que o cabeçalho reflita a estratégia ativa.

- **CA-04.1** Automático: anel + contagem atuais, sem regressão.
- **CA-04.2** Classificador: sem cronômetro; pill de estado do classificador.
- **CA-04.3** Desligado: “Desligado” + “Sugerir agora”.
- **CA-04.4** “Sugerir agora” fica desabilitado só enquanto gera.
- **CA-04.5** Badge LIVE permanece.

### US-05 — Escolher a estratégia no topo de “Fontes de IA & Modelos”

Como usuário, quero decidir entre classificador e cadência fixa no mesmo lugar
onde escolho modelos de IA, sem caçar uma aba nova.

- **CA-05.1** A aba “Fontes de IA & Modelos” começa com o bloco “Modo de ativação
  de sugestões”, com dois cards selecionáveis: **Modelo de classificação** e
  **Sugestão por tempo**.
- **CA-05.2** O card do classificador mostra “Modelo selecionado”
  (`Laya (Padrão)` / `Regras locais`), status do modelo
  (instalado/não instalado/baixar) e **Sensibilidade** (`Baixa` | `Balanceada` |
  `Alta`).
- **CA-05.3** O card de tempo mostra **Cadência de ciclo** (`8s` | `15s` | `30s`)
  e a nota “Disparo cego / sem filtro semântico”.
- **CA-05.4** Selecionar um card grava `suggestion_mode`; o interruptor mestre
  liga/desliga sem perder a escolha.
- **CA-05.5** O card de sugestões sai de “Geral & HUD”; o de resumo fica.
- **CA-05.6** “Avançado” (drawer/modal) concentra model manager, cooldown,
  timeout, fallback, blocos, categorias, sonda e teste — sem poluir o cartão.
- **CA-05.7** “Testar classificador” usa o último bloco ou um bloco de exemplo e
  mostra o resultado sem gerar sugestão.
- **CA-05.8** Backend `Regras locais` funciona offline; `Laya` mostra caminho do
  ONNX/checkpoint e status, sem token/URL.

### US-06 — POC Laya com evidência

Como time, queremos validar o Laya antes de adotá-lo.

- **CA-06.1** POC usa o crate `laya-onnx` dentro do Rust e classifica blocos
  reais sem processo Python em runtime.
- **CA-06.2** Fixtures com pelo menos 24 blocos rotulados (pt/en) cobrindo as 7
  cues, avaliadas por um binário Rust.
- **CA-06.3** Avaliação reporta acurácia, precisão/recall por cue, matriz de
  confusão e latência p50/p95.
- **CA-06.4** Gate: acurácia de `should_suggest` ≥ 80%, falso positivo ≤ 15%,
  p95 ≤ 1 s (CPU) ou ≤ 300 ms (GPU).
- **CA-06.5** Se o gate falhar, o padrão continua `automático` e a POC fica
  documentada; nenhum release depende do Laya.

### US-07 — Instalar e trocar o modelo do classificador

Como usuário final, quero baixar o modelo pelo app, sem Python nem terminal.

- **CA-07.1** O card “Modelo de classificação” (em Fontes de IA & Modelos)
  mostra o modelo recomendado (`laya-multilingual`), versão, tamanho
  (~1,2 GB), licença e status (“não instalado”, “baixando”, “instalado”,
  “corrompido”).
- **CA-07.2** “Baixar modelo” pede confirmação, mostra progresso, permite
  cancelar e retomar; grava em `.part` e só renomeia após verificar SHA-256.
- **CA-07.3** Modelo instalado carrega sem rede; “Apagar modelo” libera espaço e
  volta ao heurístico.
- **CA-07.4** Trocar de versão do modelo (`multilingual`) é possível sem
  reinstalar o app. O checkpoint English não é exposto.
- **CA-07.5** Falha de download/verificação mostra erro claro e não deixa o app
  em estado quebrado.
- **CA-07.6** Nenhum download acontece sem ação explícita do usuário.

### US-08 — Preparar o caminho de fine-tuning

Como time, queremos que um modelo ajustado entre no produto sem reescrever a
integração.

- **CA-08.1** O app carrega qualquer artefato ONNX que siga o `manifest.json`
  (mesmo layout, mesma pergunta `cue`), com `model_version` distinta.
- **CA-08.2** O manifesto registra base checkpoint, versão, SHA-256, calibração,
  métricas e hash do dataset de avaliação.
- **CA-08.3** O harness de avaliação (fixtures + métricas) roda igual para o
  modelo base e para um fine-tuned.
- **CA-08.4** Dados de conversa **não** são enviados para treino; qualquer
  captura para fine-tuning é opt-in e local (BR-100).
- **CA-08.5** O caminho de dev (exportar ONNX localmente) continua documentado,
  mas não é necessário para o usuário final.

---

## 9. POC com Laya (SDK Rust, dentro do projeto)

### 9.1 Decisão: `laya-onnx` no backend Rust

O modelo é o **`convaiinnovations/laya-multilingual`** (Laya Decision Model,
mmBERT-base). O SDK Rust é o crate **`laya-onnx`** (importado como `laya`; vive
no submódulo `vendor/laya` porque ainda não está no crates.io) e roda **ONNX
Runtime localmente, sem Python em runtime**. Ele reproduz o caminho do SDK
Python (tokenização, sequence building, calibração e respostas tipadas) e é
`Send + Sync`, adequado para o agendador. O checkpoint raiz
`convaiinnovations/laya` é English-only e **não é exposto**; só o multilingual
entra na UI.

- **Runtime:** `laya-onnx` dentro do `src-tauri`, atrás de uma feature Cargo
  opcional (`laya`). O build padrão continua leve.
- **Exportação:** uma vez, do checkpoint PyTorch (`convaiinnovations/laya`) para
  ONNX com `laya-rust/tools/export_onnx.py`. O artefato ONNX ainda **não está
  publicado no Hugging Face**; Python só é necessário nessa etapa de build.
- **Modelo:** `convaiinnovations/laya-multilingual` (mmBERT-base, 322M, 1024
  tokens, 100+ idiomas) para pt/en; ~1,2 GB de ONNX (`model.onnx` +
  `model.onnx.data`) + tokenizer. Não entra no bundle por padrão; é carregado
  de um diretório configurado.
- **Nada de** `laya-serve`, token HTTP ou `POST /v1/systemone`.
- O router do app `aay.sh` não é usado: ele classifica eventos e cria cards no
  Laya.

Requisitos técnicos:

- **Rust ≥ 1.88** (edition 2024 + let-chains). O projeto declara `1.77`; é
  preciso subir o MSRV ou isolar o adaptador em um crate separado.
- `laya-onnx` 0.1.x + `ort` 2.x (ONNX Runtime). O `ort` pode baixar o binário
  pré-compilado no build; o release precisa vendorizar/empacotar o ONNX Runtime.
- O artefato ONNX é carregado do **modelo ativo do model manager**;
  `suggestion_classifier_model_path`/`LAYA_ONNX_DIR` são override de dev.
  `allow_download = false` no SDK: o download é do model manager, não do
  `laya-onnx` (local-only).
- O crate `laya-onnx` é opcional; sem a feature, o backend `laya` aparece como
  “indisponível neste build”.

### 9.2 Contrato Rust da POC

Uma única pergunta `choice` (`cue`) — as duas categorias negativas já estão
entre as opções. **Não usar `noul` para “deve sugerir?”**: o model card alerta
que `noul` pode seguir os rótulos `true`/`false` e errar o sinal; a decisão é
derivada da cue + confiança + abstenção.

```rust
use laya::{
    Answer, LayaCheckpoint, LayaEngine, LayaOptions, MinConfidence,
    PredictOptions, Question, QuestionSet,
};
use serde_json::json;

let engine = LayaEngine::create(LayaOptions {
    model_directory: Some(path_to_onnx),
    checkpoint: LayaCheckpoint::Multilingual,
    allow_download: false,
    ..Default::default()
})?;

let questions = QuestionSet::new().with(
    "cue",
    Question::choice(
        "Qual é o próximo movimento do copiloto?",
        [
            ("question", "THEY fez uma pergunta que ME ainda não respondeu."),
            ("comment", "THEY fez um ponto/opinião e convida ME a comentar."),
            ("already_answered", "A pergunta repete uma já respondida no histórico recente."),
            ("no_action", "Conversa casual, explicação ou fala sem pergunta/convite."),
            ("insufficient_context", "Contexto insuficiente para decidir."),
        ],
    )?,
)?;

let result = engine.predict_with_options(
    json!({
        "locale": "pt-BR",
        "assistant": "default",
        "block": "ME: ...\nTHEY: ...",
        "context": "blocos anteriores, se houver",
    }),
    &questions,
    &PredictOptions::new().min_confidence(MinConfidence::scalar(0.6)?),
)?;

let answer = result.get("cue").ok_or(ClassifierError::InvalidResponse)?;
let choice = answer.as_choice().ok_or(ClassifierError::InvalidResponse)?;
let cue = choice.choice.as_str();
let confidence = answer.answer_confidence();
let abstained = answer.abstention() == Some("abstained")
    || answer.low_confidence();
let should_suggest = !abstained
    && confidence >= min_confidence
    && !matches!(cue, "already_answered" | "no_action" | "insufficient_context");
```

Mapeamento para o domínio:

- `cue = choice.choice`;
- `confidence = answer.answer_confidence()` (não a `confidence()` de entropia);
- `abstained = answer.abstention() == Some("abstained")` ou
  `answer.low_confidence()`;
- `should_suggest` derivado de cue + confiança + abstenção (sem segunda
  pergunta);
- `focus`/`evidence` determinísticos por cue e pelo último turno relevante;
- latência medida no adaptador;
- **sem `reason` livre** — a evidência é o trecho do bloco.

### 9.3 Onde roda

- `LayaEngine` carregado uma vez (lazy, no primeiro uso) e compartilhado como
  `Arc`; é `Send + Sync`.
- O adaptador `LayaSuggestionCueClassifier` roda a inferência em uma **worker
  thread** com fila e `recv_timeout`, para que o agendador não fique preso em um
  ONNX lento; timeout → `ClassifierError::Timeout` e o resultado atrasado é
  descartado.
- Modelo ausente/inválido → backend `Unavailable`; a UI mostra o erro e o
  usuário pode continuar no heurístico.
- Sem rede durante a inferência; a única rede é o download explícito do
  modelo. Sem Python.
- A POC é um binário/exemplo Rust (`poc/laya-classifier` ou
  `examples/laya_eval.rs`) que lê os fixtures e calcula as métricas — não um
  serviço HTTP.

### 9.4 Distribuição para usuário final

O caminho de produto não é “compile o ONNX”; é **instalar o modelo pelo app**.

**Armazenamento local**

```
<app_data>/models/laya/
  manifests/<model_version>.json
  <model_version>/
    model.onnx
    model.onnx.data
    rl_agent_config.json
    tokenizer/
      tokenizer.json
      tokenizer_config.json
  current -> <model_version>   # link/arquivo de ponteiro atômico
```

**Catálogo (`catalog.json`)**

```json
{
  "schema_version": 1,
  "models": [
    { "model_id": "laya-multilingual", "recommended": true, "manifest_url": "https://…/manifest.json" }
  ]
}
```

O app lê o catálogo apenas em “Gerenciar modelos”/“Verificar atualizações”; o
manifesto de cada versão é a fonte de verdade.

**Manifesto (`manifest.json`)**

```json
{
  "schema_version": 1,
  "model_id": "laya-multilingual",
  "model_version": "2026.10.0",
  "base_checkpoint": "convaiinnovations/laya-multilingual",
  "format": "onnx",
  "onnx_layout": "fused",
  "size_bytes": 1288490188,
  "license": "Apache-2.0",
  "files": [
    {
      "path": "model.onnx",
      "url": "https://downloads…/2026.10.0/model.onnx",
      "size_bytes": 2500000,
      "sha256": "…"
    },
    {
      "path": "model.onnx.data",
      "url": "https://downloads…/2026.10.0/model.onnx.data",
      "size_bytes": 1280000000,
      "sha256": "…"
    },
    {
      "path": "rl_agent_config.json",
      "url": "https://downloads…/2026.10.0/rl_agent_config.json",
      "size_bytes": 745,
      "sha256": "…"
    },
    {
      "path": "tokenizer/tokenizer.json",
      "url": "https://downloads…/2026.10.0/tokenizer/tokenizer.json",
      "size_bytes": 1000000,
      "sha256": "…"
    }
  ],
  "calibration": { "temperature": [1.0, 1.0, 1.0], "ece": 0.31 },
  "metrics": { "cue_accuracy": 0.0, "should_suggest_fp": 0.0 },
  "created_at": "2026-10-26T00:00:00Z"
}
```

- O download é **arquivo a arquivo** (sem tar/zip): cada `files[]` tem URL,
  tamanho e SHA-256; o adaptador baixa para `<version>.part/`, verifica e só
  então renomeia para `<version>/`. Isso permite retomada por arquivo e evita
  uma dependência de compressão.

**Fluxo do model manager**

1. O app lê um **catálogo** (`catalog.json`) com o modelo recomendado/versões e
   mostra versão, tamanho, licença e status. O catálogo é configurável
   (`suggestion_classifier_model_catalog_url`; padrão oficial).
2. Usuário confirma “Baixar modelo”.
3. Download para `.part` com progresso e cancelamento; retomada se o servidor
   suportar `Range`.
4. Verificação SHA-256 de cada arquivo; se falhar, apaga `.part` e mostra erro.
5. Instalação atômica (`<version>.part/` → `<version>/`); só então atualiza
   `current`.
6. `LayaEngine` carrega de `current`; se não houver, backend `Unavailable` e o
   heurístico assume.
7. “Apagar modelo” remove a versão (exceto a ativa) e libera espaço; “Trocar
   versão” só muda o ponteiro.

**Hospedagem**

- O artefato é publicado no **release/download server do GhostWhisper** (ex.:
  GitHub Releases ou `downloads.ghostwhisper…`) com `manifest.json` e SHA-256.
- O `suggestion_classifier_model_catalog_url` é configurável (dev/self-host);
  o padrão é o servidor oficial.
- Nada de download silencioso; o usuário vê tamanho e licença antes.
- O fallback de dev (exportar ONNX localmente) continua aceito por
  `model_path`, mas não é o fluxo do usuário final.

### 9.5 Caminho de fine-tuning (preparado, não executado agora)

O fine-tuning não faz parte deste PRD, mas o contrato precisa nascer pronto:

- **Mesmo layout ONNX**: um modelo fine-tuned é exportado com o mesmo
  `laya-rust/tools/export_onnx.py` e carregado pelo mesmo `LayaEngine`.
- **Mesma pergunta `cue`**: o schema de 7 opções é estável; mudanças de labels
  exigem nova `model_version` e novo eval.
- **Manifesto**: `model_version`, `base_checkpoint`, SHA-256, calibração,
  métricas e hash do dataset de avaliação. O app não precisa saber se o modelo é
  base ou fine-tuned.
- **Calibração**: cada artefato traz a temperatura/ECE usada; o limiar de
  confiança é validado nas fixtures antes de trocar a versão ativa.
- **Dataset/eval**: o harness `poc/laya-classifier` (fixtures + métricas) é o
  mesmo para base e fine-tuned. Um dataset de treino futuro é **opt-in, local e
  anonimizado**; nenhuma conversa sai da máquina sem consentimento explícito
  (BR-100).
- **Rollback**: manter a versão anterior instalada; trocar o ponteiro `current`
  é o rollback.
- **Registro**: `manifest.json` + `metrics` versionados junto do release;
  `memory.md` do PRD registra a avaliação de cada versão. Detalhes em
  [`architecture/fine-tuning.md`](architecture/fine-tuning.md).

### 9.6 Avaliação

- Fixtures `poc/laya-classifier/fixtures/blocks.jsonl` com ≥ 24 blocos
  balanceados (pt/en) e rótulo humano.
- Um binário Rust com `laya-onnx` calcula acurácia de `should_suggest`,
  precisão/recall/F1 por cue, matriz de confusão e p50/p95.
- Resultado vai para `memory.md`; se falhar, registra a hipótese e mantém o
  heurístico como backend padrão do modo classificador.

### 9.7 Riscos específicos

| Risco | Mitigação |
|---|---|
| ONNX ainda não publicado no HF | publicar como release asset do GhostWhisper com manifesto + SHA-256; export local continua como dev |
| Modelo grande (~1,2 GB) | não entra no bundle; download sob confirmação + model manager |
| Download interrompido/corrompido | `.part`, SHA-256, instalação atômica, retomada e rollback |
| Hospedagem/limite de banda | CDN/release asset; URL configurável; conteúdo estático |
| Fine-tuning quebrar o contrato | mesmo layout ONNX + `manifest.json` + eval obrigatório antes de ativar |
| `laya-onnx` exige Rust ≥ 1.88 | subir o MSRV ou isolar o adaptador; validar no CI |
| `ort` baixa binário no build | vendorizar/empacotar ONNX Runtime no release; documentar build offline |
| Modelo pequeno erra nuance/ironia | gate de acurácia; fallback heurístico; modo automático continua padrão |
| RAM extra (~1,2 GB) | feature opt-in; carregar sob demanda; descarregar quando trocar para heurístico |
| Confiança mal calibrada (ECE ~0,31 no multilingual) | usar `answer_confidence`; calibrar limiar nas fixtures |
| Multilingual mais fraco em English que o checkpoint English | só `multilingual` (cobre pt/en e ocupa menos espaço); English não é exposto |
| `choice` com muitas opções perde acurácia | 7 opções (bem abaixo do limite ~20 do model card) |
| `noul` com viés de rótulo | não usar `noul`; derivar de `choice` + confiança |
| Privacidade | 100% local; sem serviço/rede no runtime |

---

## 10. Métricas de sucesso

**Produto**

- Nenhuma regressão no modo automático (mesmos testes e comportamento).
- O usuário consegue explicar, olhando a faixa, por que uma sugestão apareceu
  ou não.
- Menos sugestões “sem motivo”: queda de sugestões automáticas por 10 min sem
  queda de perguntas atendidas (medida na POC e em sessões reais).
- Trocar de modo leva ≤ 2 cliques.
- Download do modelo conclui com sucesso em ≥ 95% das tentativas; falha de
  checksum nunca deixa o app quebrado.
- Modelo instalado carrega sem rede em ≤ 5 s (após o boot/lazy).

**POC**

- `should_suggest` acurácia ≥ 80%, falso positivo ≤ 15%.
- p95 ≤ 1 s (CPU) / ≤ 300 ms (GPU).
- Nenhum conteúdo de transcrição em log.
- Um artefato fine-tuned (quando existir) entra pelo mesmo manifesto e passa no
  mesmo eval, sem mudança de código.

**Engenharia**

- `cargo test`, `cargo clippy -D warnings`, `cargo fmt --check`, `pnpm test`,
  `tsc --noEmit` e `pnpm build` verdes.

---

## 11. Não-objetivos

- Treinar/fine-tunar o Laya neste PRD (a POC é zero-shot; a task 12 prepara o
  contrato/avaliação, mas o treino em si é um PRD futuro).
- Persistir classificações no SQLite ou no histórico da sessão (POC usa memória
  por sessão; persistência é evolução).
- Trocar o classificador por padrão sem o gate da §9.
- Mudar os contratos dos agentes além de `cue`/`focus`.
- Backend remoto, login, telemetria (BR-002/BR-004).
- Classificar áudio; o insumo é texto finalizado.
- Substituir o “Sugerir agora” ou o atalho global.
- UI de feedback (“útil/não útil”) para treinar o classificador — fica para
  depois.
- Suporte a outros idiomas além de pt/en na UI (o classificador pode ser
  multilíngue, mas a copy segue R18–R22).

---

## 12. Riscos e mitigação

| Risco | Impacto | Mitigação |
|---|---|---|
| Blocos mal cortados geram decisões ruins | Alto | algoritmo puro e testado; gap/duração/turnos configuráveis; prévia do bloco na faixa |
| Classificador latente trava o agendador | Médio | timeout; thread separada; `skip` por padrão; estado “Analisando” |
| Falso positivo incomoda | Alto | limiar de confiança; cooldown; dedupe; modo automático como fallback |
| Falso negativo perde pergunta | Alto | regra determinística de `?` no heurístico; `question` sempre habilitada por padrão |
| Configuração demais confunde | Médio | bloco com presets e “Avançado” em drawer; defaults seguros |
| Modelo não instalado/hardware insuficiente | Médio | heurístico offline; download opcional; documentar RAM/CPU |
| Download grande/falho | Alto | confirmação, progresso, `.part`, SHA-256, retomada, rollback |
| Modelo fine-tuned quebra o contrato | Médio | mesmo layout ONNX + manifesto + eval obrigatório |
| Privacidade | Alto | local por padrão; aviso; sem log de transcrição |
| Migração quebrar settings antigos | Alto | `suggestion_mode` com `serde(default)`; teste de JSON antigo |
| Duplicar sugestão no mesmo bloco | Médio | `block_id` + `generation_id` + dedupe no gate |
| Faixa “roubar” o topo | Baixo | uma linha compacta, expansível; scroll do corpo |

---

## 13. Regras de negócio

Registradas em [`../business-rules.md`](../business-rules.md) (escrita única,
R14):

- **BR-092 — Modo de sugestão:** `suggestion_mode = auto | classifier`;
  `suggestion_auto_refresh` é o interruptor mestre; migração de settings antigos
  para `auto`; nenhuma troca silenciosa.
- **BR-093 — Blocos de conversa:** só falas finalizadas; fechamento por silêncio,
  duração ou turnos; contexto de 8–20 blocos; sem sonda de bloco aberto.
- **BR-094 — Classificador por intenção:** porta `SuggestionCueClassifier`; cues
  `question`, `comment`, `already_answered`, `no_action`,
  `insufficient_context`; confiança/limiar; timeout; privacidade.
- **BR-101 — Histórico de perguntas respondidas:** últimas 5 (0–20) entram no
  prompt e na checagem de repetição.
- **BR-095 — Última classificação:** sempre visível por padrão; cue, decisão,
  confiança, evidência, bloco, backend e latência; não bloqueia a geração.
- **BR-096 — Cabeçalho adaptativo:** automático mostra timer; classificador
  mostra estado do classificador; “Sugerir agora” sempre disponível.
- **BR-097 — Bloco em Fontes de IA & Modelos:** o modo de sugestão vive no topo
  da aba “Fontes de IA & Modelos” (dois cards: classificador vs. tempo);
  o avançado fica em drawer/modal; não há aba “Sugestões” no MVP; o card de
  sugestões sai de “Geral & HUD”.
- **BR-098 — POC Laya:** SDK Rust `laya-onnx` atrás da feature `laya`; ONNX
  local, sem Python em runtime; modelo fora do bundle; gate de
  acurácia/latência; local por padrão.
- **BR-099 — Artefato e model manager:** o modelo é um artefato ONNX versionado
  com `manifest.json` + SHA-256 por arquivo, baixado sob confirmação,
  verificado e instalado atomicamente; sem download silencioso; rollback por
  ponteiro de versão.
- **BR-100 — Caminho de fine-tuning:** modelo ajustado usa o mesmo layout ONNX,
  a mesma pergunta `cue` e o mesmo manifesto/eval; qualquer captura de dados
  para treino é opt-in, local e anonimizada; nada sai da máquina sem
  consentimento explícito.

---

## 14. Decisões pendentes

1. **Padrão:** manter `Automático` até a POC passar (recomendado) ou já lançar
   `Classificador` como padrão?
2. **Hospedagem do artefato:** GitHub Releases (recomendado para o MVP) ou
   servidor/CDN próprio? O contrato já aceita
   `suggestion_classifier_model_catalog_url` configurável.
3. **Hesitação em bloco aberto:** sonda a cada 4 s (recomendado) ou só classificar
   blocos fechados no primeiro momento?
4. **Última classificação:** memória por sessão no POC (recomendado) ou persistir
   no SQLite desde já?
5. **Fallback:** em erro do classificador, `skip` (recomendado) ou cair no
   automático?
6. **Cue no cartão:** mostrar o selo da cue na sugestão gerada (recomendado) ou
   só na faixa de classificação?

---

## 15. Evidência no Perssua 0.27.5 (BR-015)

Análise estática do `Perssua-0.27.5-x86_64.AppImage`
(`sha256 4b2dacfe…`) + logs `[REALTIME-INSIGHTS]`:

- **Regex determinística de pergunta:** `/[?？]/u` sobre mensagens de THEY e
  transcrição ao vivo; dispara cedo (debounce 750 ms; 150 ms no Cody).
- **Gatilho de silêncio `user-help`:** após 1–2 turnos seguidos de ME e 3–5 s de
  silêncio; prompt “USER/ME … went silent. Write a useful next line for ME…”.
- **Classificador de hesitação por LLM:** `evaluateHesitation()` após 4 s de fala
  contínua, ≥ 3 palavras; prompt com fillers, repetições, autocorreções,
  “how can I say”, trailing off; retorna `HESITATING`/`FLUENT` e, se positivo,
  libera o insight mesmo com `isMeSpeaking`.
- **Modos de Notch:** only_questions, moderate, proactive (padrão, silêncio 2
  turnos/5 s), full_power (1 turno/3 s), meeting_notes; check-ins de 15/30/90 s.
- **Automatic triggers:** “When I may need help”, “Questions”, “Periodic
  check-ins”, “Session start”, “Screen selection”.
- **Transparência:** o Perssua não tem uma faixa persistente de “última
  classificação”; essa é uma adição nossa para diagnóstico e confiança.

Referências de código (asar extraído): `310.8def3e52.chunk.js`
(`useRealtimeSummary`, `evaluateHesitation`), `main.f180d025.js` (regex de
pergunta, prompts de trigger, modos), `6552.a9882e27.js` (labels).

---

## 16. Entregáveis

| # | Entregável | Tasks |
|---|---|---|
| E1 | Domínio de blocos, cues e decisão pura | [`01`](tasks/01-task.md) |
| E2 | Porta do classificador + evento + fake | [`02`](tasks/02-task.md) |
| E3 | Serviço de gate + cache da última classificação | [`03`](tasks/03-task.md) |
| E4 | Settings, migração, tipos e IPC | [`04`](tasks/04-task.md) |
| E5 | Backend heurístico local | [`05`](tasks/05-task.md) |
| E6 | Modelo Laya + model manager + avaliação | [`06`](tasks/06-task.md) |
| E7 | Cabeçalho adaptativo | [`07`](tasks/07-task.md) |
| E8 | Faixa de última classificação | [`08`](tasks/08-task.md) |
| E9 | Bloco de ativação em “Fontes de IA & Modelos” | [`09`](tasks/09-task.md) |
| E10 | Integração no agendador | [`10`](tasks/10-task.md) |
| E11 | Hardening, docs, pacote e teste manual | [`11`](tasks/11-task.md) |
| E12 | Caminho de fine-tuning: manifesto, dataset, eval, registry | [`12`](tasks/12-task.md) |
