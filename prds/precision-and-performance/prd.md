# PRD — Precisão e Desempenho (precision-and-performance)

- **Versão:** 0.1
- **Data:** 2026-10-18
- **Status:** Implementado (tasks 01–03 `CONCLUDED` — E1, E2 e E3 entregues)
- **Autor:** Equipe GhostWhisper
- **Relacionados:** [`../business-rules.md`](../business-rules.md),
  [`memory.md`](memory.md), [`issues.md`](issues.md),
  [`architecture/precision-and-performance.md`](architecture/precision-and-performance.md),
  [`tasks/_tasks.md`](tasks/_tasks.md) · PRDs anteriores:
  [`../mvp/prd.md`](../mvp/prd.md), [`../chat/prd.md`](../chat/prd.md),
  [`../assistant/prd.md`](../assistant/prd.md)

---

## 1. Objetivo

Fechar a lacuna percebida em relação ao **Perssua 0.27.5** (BR-015) em três eixos,
entregues em **3 entregáveis independentes e testáveis**:

1. **Bolha de mensagem menor** — sugestão e fala em blocos curtos (≈3 linhas),
   não em documentos longos.
2. **Transcrição mais rápida** — texto/fala aparecendo antes e engine sem trabalho
   duplicado.
3. **Transcrição mais precisa** — menos erro e menos alucinação, sobretudo pt-BR.

## 2. Problema

Observação do usuário comparando com o Perssua: "o Perssua gera message bubbles
menores (no máximo ~3 linhas), e é um pouco mais rápido e mais acertivo na
transcrição".

Diagnóstico (evidência em
[`architecture/precision-and-performance.md`](architecture/precision-and-performance.md)):

| Sintoma | Causa medida |
|---|---|
| Bolha longa | nosso painel é `540×680` e o notch do Perssua `436×212`; turno de fala até 25 s; o "cue" falado não é colapsado; sem *fast-first* |
| Sugestão tardia/ausente | 1 chamada **não-streaming** de 900–1400 tokens, a cada **20 s**, só com a janela aberta; contexto só de falas **finalizadas** |
| Menos precisa | default `tiny` (Perssua só oferece `large-v3`/`large-v3-turbo`); **sem** `initial_prompt` de decodificação; **sem AEC**; reamostragem linear; gate de ruído mais permissivo |
| Menos rápida | `whisper-rs` em **CPU** sem flash attention; todo turno paga um passe final beam search extra; prévia em voo bloqueia o fechamento |

## 3. Causas (teoria consolidada)

- **E1 — Sinal e decodificação (barato, alto ganho).** O Perssua manda um
  `whisper_prompt` em toda request e reamostragem de melhor qualidade; nós não.
- **E2 — Percepção de velocidade e tamanho (produto/UI).** *Fast-first* + streaming
  + insights automáticos contínuos + UI compacta. É o que o usuário vê.
- **E3 — Base de engenharia (caro, dependente de hardware).** GPU/flash attention,
  modelo grande, AEC referenciado e menos trabalho por turno.

Nada disso é inventado: cada item tem âncora no bundle do Perssua
(`architecture/precision-and-performance.md` §5, R11–R13).

## 4. Entregáveis

| # | Entregável | Escopo | Task | Teste após concluir |
|---|-----------|--------|------|---------------------|
| **E1** | Precisão do ASR (sinal + decodificação) | `initial_prompt`, idioma, reamostragem anti-alias, catálogo de modelos, **detecção de GPU para sugerir modelo** | [`tasks/01-task.md`](tasks/01-task.md) | sessão real pt-BR + `cargo test` |
| **E2** | Bolha compacta e sugestão fast-first | fast-first, streaming, contexto com prévias, insights automáticos sem janela, UI compacta | [`tasks/02-task.md`](tasks/02-task.md) | medir t1 e nº de linhas em sessão real |
| **E3** | Áudio limpo e engine rápida | GPU + flash attention, modelo por hardware, AEC referenciado, reuso do final, commits | [`tasks/03-task.md`](tasks/03-task.md) | benchmark RTF + teste AEC + sessão real |

Regra do ciclo: **só começa E(n+1) depois de E(n) aprovado no teste manual**, e
cada task atualiza `tasks/NN-task.md` para `CONCLUDED` e o `memory.md` deste PRD
(R15, BR-017).

## 5. Rastreio das sugestões originais

| Sugestão | Entregável |
|---|---|
| 1. `initial_prompt` de decodificação | E1 |
| 2. subir o modelo / `large-v3-turbo` | E1 (catálogo, opção e **sugestão por GPU**) · E3 (default por hardware) |
| 3a. fast-first de insight | E2 |
| 3b. streaming do LLM | E2 |
| 3c. gerar sugestão a partir de prévias | E2 |
| 3d. clamp do cue e UI compacta | E2 |
| 4. GPU + flash attention | E3 |
| 5. AEC e reamostragem anti-alias | E1 (resampler/gate) · E3 (AEC) |
| 6. commits/turno menores | E3 |

## 6. Histórias e critérios de aceite

### US-01 — Transcrição mais precisa (E1)

- **CA-01.1** Toda decodificação envia o `whisper_prompt` configurado, que é
  **contexto** (default **vazio**). Instruções são desencorajadas pela UI: o
  modelo as continua quando o áudio é incerto (ISS-059).
- **CA-01.2** O idioma é fixado por `set_language` + `set_translate(false)`; a
  regra de idioma **não** vai mais no prompt (`pt-br` → `pt`).
- **CA-01.3** A reamostragem do microfone aplica anti-aliasing antes de decimar
  (sem tom espelhado acima de 8 kHz).
- **CA-01.4** Limiares de gate/VAD permanecem os do **BR-018** (o gate fica no
  limiar de fala do app, não nos 0,025 do Perssua — decisão já registrada) e
  ganham teste; o efeito do `initial_prompt` é medido isoladamente.
- **CA-01.5** Catálogo passa a oferecer `large-v3` e `large-v3-turbo`, com aviso
  de desempenho; o default **não** muda nesta entrega (muda em E3, com GPU).
- **CA-01.6** O app **detecta GPU** (Vulkan no Linux-first) e, quando encontra,
  **sugere** o modelo adequado na tela de configurações — sem trocar sozinho.
  Enquanto a aceleração por GPU não existir (E3), a sugestão deixa claro que o
  build atual ainda roda em CPU.
- **CA-01.7** Sem regressão de latência > 10 % no benchmark de RTF (`tiny`, CPU).

### US-02 — Bolha menor e sugestão que aparece rápido (E2)

- **CA-02.1** A **primeira** sugestão é um *fast-first* curto (≤ ~3 linhas,
  `max_tokens` 300–420, temperatura 0, prompt compacto) e aparece antes do texto
  completo.
- **CA-02.2** A resposta do LLM é **streamed** para a UI (evento por delta), com
  fallback não-streaming quando o provider não suporta.
- **CA-02.3** O contexto da sugestão inclui **falas em andamento** (prévias), não
  só finais.
- **CA-02.4** Existem insights **automáticos** sem a janela de sugestões aberta,
  disparados por fala nova, com *supersede* (nunca empilha requisições) e toggle
  nas configurações.
- **CA-02.5** No chat, o "cue" falado é colapsado em 1–3 linhas (clique expande);
  seções longas são colapsáveis.
- **CA-02.6** Nenhuma sugestão automática entra no export sem **Fixar** (BR-023).

### US-03 — Áudio limpo e engine rápida (E3)

- **CA-03.1** Com GPU disponível, o app usa local Whisper em GPU com **flash
  attention**; sem GPU, mantém o caminho CPU.
- **CA-03.2** O modelo default é escolhido por hardware: `large-v3-turbo` com
  GPU, menor em CPU; a UI mostra a recomendação e o custo.
- **CA-03.3** O microfone recebe **AEC referenciado** no áudio do sistema
  (atraso configurável), reduzindo eco/duplicata em viva-voz.
- **CA-03.4** Quando o VAD nativo já fechou o turno, o resultado da prévia é
  reusado como final (sem segundo beam search desnecessário).
- **CA-03.5** O fechamento de turno não fica preso atrás de prévia em voo.
- **CA-03.6** RTF < 1 em GPU com o modelo recomendado.

## 7. Requisitos não funcionais

- **RNF-01 — Testabilidade:** regras novas puras (`application`), portas com dobra
  em memória; nenhum I/O em `domain`/`application` (R7, R8).
- **RNF-02 — Paridade (R11–R13):** nomes/valores observáveis do Perssua mantidos;
  desvios documentados no doc de arquitetura.
- **RNF-03 — Custo:** E2 não pode aumentar o custo por minuto sem controle
  (fast-first curto + supersede + intervalo configurável).
- **RNF-04 — Privacidade:** nada de novo sai da máquina além do que já sai (BR-002,
  BR-004, BR-005); áudio nunca é logado.
- **RNF-05 — Sem regressão:** `cargo test`, `cargo clippy`, `cargo fmt --check`,
  `pnpm test`, `tsc --noEmit`, `pnpm build` verdes em cada entregável.
- **RNF-06 — Documentação única:** IDs `BR-027`…`BR-029`, `ISS-023`…, `NN-task`
  (R14, BR-016).

## 8. Não-objetivos

- Captura de tela/OCR e MCP/connected tools (seguem fora, como no MVP).
- Provider próprio/backend, login ou telemetria (BR-002, BR-004).
- Trocar o whisper.cpp por outra engine (ex.: Parakeet/Sherpa) nesta PRD.
- Fine-tuning de modelo.
- Streaming token a token no **resumo** (só na sugestão, em E2).
- Reescrita do protocolo de áudio para Wayland nativo.

## 9. Riscos

| Risco | Mitigação |
|---|---|
| `initial_prompt` virar "instrução" e o modelo responder em vez de transcrever | prompt fixo, curto, de transcrição; teste com áudio real; fallback para vazio por setting |
| Reamostragem nova introduzir atraso/ganho | filtro curto, testes com tom e ganho unitário; medir RTF |
| Streaming do LLM aumentar complexidade do IPC | porta de streaming com fallback; eventos por delta; `supersede` |
| Insights automáticos elevarem custo de API | fast-first curto, debounce, teto de requisições em voo, toggle e pausa |
| GPU/flash attention instável em alguns drivers | detecção + fallback CPU; medir antes de tornar default |
| AEC mal ajustado comer o começo da fala | atraso configurável (default 60 ms), teste sintético e toggle |
| Modelo maior inviabilizar CPU | default por hardware; CPU permanece no modelo atual |

## 10. Métricas de sucesso

- **E1:** com o mesmo áudio pt-BR, menos palavras erradas e menos alucinação que
  antes; RTF sem regressão > 10 %.
- **E2:** primeira bolha de sugestão visível em **< 2–3 s** (cloud) e com
  **≤ ~3 linhas**; insights automáticos sem abrir a janela.
- **E3:** RTF < 1 com `large-v3-turbo` em GPU; eco do microfone reduzido no
  fixture AEC; sessão real com bolhas curtas e sem "faltou o final".
- Suíte completa verde em cada task.

## 11. Protocolo de teste (entregável a entregável)

1. Concluir a task → `CONCLUDED` em `tasks/NN-task.md` + entrada no `memory.md`.
2. Rodar a suíte e o roteiro da task (`tasks/NN-task.md` §"Como testar").
3. Usuário testa em sessão real (reunião/vídeo pt-BR) e aprova ou abre ajuste.
4. Só então iniciar a próxima task.

## 12. Evidência no Perssua (BR-015)

Extraído de `Perssua-0.27.5-x86_64.AppImage` → `resources/app.asar` (já em
`/tmp/opencode/perssua/extracted`). Tabela completa de âncoras em
[`architecture/precision-and-performance.md`](architecture/precision-and-performance.md) §5.
