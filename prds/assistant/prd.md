# PRD — Resumo vivo e sugestões por agentes

- **Versão:** 0.1
- **Data:** 2026-10-18
- **Status:** Implementado (tasks 01–06 `CONCLUDED`)
- **Relacionados:** [`../business-rules.md`](../business-rules.md),
  [`memory.md`](memory.md), [`issues.md`](issues.md),
  [`architecture/assistant-windows.md`](architecture/assistant-windows.md),
  [`tasks/_tasks.md`](tasks/_tasks.md), PRDs anteriores:
  [`../mvp/prd.md`](../mvp/prd.md) e [`../chat/prd.md`](../chat/prd.md)

---

## 1. Objetivo

Entregar duas superfícies de assistência em **janelas próprias e móveis**, além do
overlay:

1. **Janela de Resumo** — resume a conversa de tempos em tempos, de forma
   incremental, enquanto estiver aberta.
2. **Janela de Sugestões** — analisa os últimos momentos da conversa e sugere a
   próxima fala/resposta, com **seleção de agentes** (Reunião, Entrevista Técnica,
   LeetCode, System Design e agentes personalizados).

As duas superfícies produzem **Markdown** e o renderizam com código formatado e
colorido. As sugestões automáticas são efêmeras; o usuário pode **fixar** uma
sugestão na conversa/export.

## 2. Problema

Hoje o GhostWhisper tem uma sugestão única (“Sugerir”) no cabeçalho da conversa,
com um prompt genérico de três tópicos. Isso não cobre:

- acompanhar uma reunião longa com um **resumo vivo**;
- dar contexto de domínio para a IA (entrevista técnica, LeetCode, system design);
- responder com **código** legível na hora;
- manter o assistente visível em uma janela separada, posicionável ao lado da
  reunião;
- transformar a sugestão num documento Markdown útil.

O Perssua (produto de referência, BR-015) já resolve isso com agentes embutidos e
personalizados e um resumo incremental. Vamos transpor o comportamento, não o
código (BR-008).

## 3. Público-alvo e casos de uso

Mesmas personas do MVP (`../mvp/prd.md` §3), com foco em:

- **CU-01 — Acompanhar a reunião:** abrir o Resumo e ler o que foi discutido,
  decisões, action items e questões em aberto, atualizados periodicamente.
- **CU-02 — Receber a próxima fala:** abrir Sugestões no agente “Reunião” e ver
  uma sugestão curta em primeira pessoa para dizer ao interlocutor.
- **CU-03 — Entrevista técnica:** selecionar “Entrevista Técnica” ou “LeetCode” e
  receber abordagem, complexidade e código com explicação.
- **CU-04 — System Design:** receber requisitos, estimativas, arquitetura em
  ASCII e trade-offs, uma etapa por vez.
- **CU-05 — Salvar o que importa:** fixar uma sugestão na conversa para que ela
  entre na linha temporal e no export Markdown.
- **CU-06 — Personalizar:** criar um agente com prompt de sistema, prompt de
  sugestão e contexto fixo (ex.: “Vendas”, “Suporte”, “Professor de inglês”).

## 4. Histórias de usuário e critérios de aceite

### US-01 — Janela de Resumo separada e móvel

Como usuário, quero abrir o resumo em uma janela própria que eu possa mover para
outro monitor.

- **CA-01.1** O botão **Resumo** (overlay e tray) abre/foca a janela `summary`.
- **CA-01.2** A janela é sem moldura, sempre no topo, com cabeçalho próprio para
  arrastar, botão fechar (esconde) e posição lembrada entre execuções.
- **CA-01.3** Ao abrir, mostra o resumo já gravado da sessão (se existir) e inicia
  a atualização automática.
- **CA-01.4** Com o automático ligado, atualiza a cada **30 s**, mas **só chama o
  LLM quando há fala finalizada nova**.
- **CA-01.5** A atualização é **incremental**: parte do resumo anterior e
  incorpora os trechos novos; sessões longas são processadas em chunks, sem
  perder o começo.
- **CA-01.6** **Atualizar agora** regenera o resumo do zero; **Copiar** copia o
  Markdown.
- **CA-01.7** O resumo é renderizado como Markdown (títulos, listas, tabelas,
  citações e código colorido).
- **CA-01.8** O resumo é persistido e aparece no export Markdown da sessão.
- **CA-01.9** Esconder a janela para o automático; reabrir retoma.
- **CA-01.10** Sem chave configurada, mostra erro amigável com atalho para as
  configurações.

### US-02 — Janela de Sugestões com seleção de agentes

Como usuário, quero uma janela separada que olhe os últimos momentos da conversa
e me diga o que falar/fazer, no estilo do agente escolhido.

- **CA-02.1** O botão **Sugestões** (overlay e tray) abre/foca a janela
  `suggestions`.
- **CA-02.2** A janela é sem moldura, sempre no topo, móvel e com posição
  lembrada.
- **CA-02.3** O seletor lista os agentes embutidos e os personalizados; a escolha
  é persistida.
- **CA-02.4** O contexto enviado é o dos **últimos momentos** (≈3 min, até 40
  falas finais, teto de 6.000 caracteres), com a última sugestão para evitar
  repetição.
- **CA-02.5** Com o automático ligado, atualiza a cada **20 s** (configurável),
  somente se houver fala nova; **Sugerir agora** força uma nova análise.
- **CA-02.6** O agente **sempre tenta produzir algo útil**; `...` só é aceito
  quando não há transcrição ou ela é ininteligível.
- **CA-02.7** A resposta é Markdown completo, inclusive blocos de código com
  linguagem e syntax highlighting.
- **CA-02.8** **Fixar** grava a sugestão atual em `messages` (mesma linha
  temporal/export) e emite o evento `insight`.
- **CA-02.9** **Copiar** copia o Markdown; a janela mantém um histórico curto das
  sugestões geradas na sessão de uso.
- **CA-02.10** Há uma caixa **Contexto extra** para colar enunciado/código,
  enviada como fonte suplementar (não como instrução).

### US-03 — Agentes embutidos com comportamento do Perssua

Como usuário, quero agentes prontos que reproduzam o comportamento do Perssua.

- **CA-03.1 Reunião:** devolve uma citação com a próxima fala em primeira pessoa
  e, quando aplicável, seções “O que está sendo dito”, “Ações” e “Glossário”.
- **CA-03.2 Entrevista Técnica:** fala em primeira pessoa, uma etapa por vez;
  para perguntas técnicas usa “Easy / Practical / Deep”; para system design
  esclarece requisitos, dimensiona e desenha incrementalmente.
- **CA-03.3 LeetCode:** coloca **Big O** no topo, detecta a linguagem, apresenta
  implementação inicial em bloco de código, explica a abordagem, complexidade,
  testes e glossário.
- **CA-03.4 System Design:** requisitos funcionais/não funcionais, estimativas
  (DAU, QPS de leitura/escrita, storage, banda), diagrama ASCII e trade-offs.
- **CA-03.5** Sem tela/OCR, o modo LeetCode trabalha com o enunciado falado e com
  o **Contexto extra**; não inventa o problema quando ele não foi dito.
- **CA-03.6** Os prompts são **contratos adaptados** do Perssua 0.27.5, com
  atribuição; não há cópia de código proprietário (BR-008).

### US-04 — Agentes personalizados

Como usuário, quero criar meus próprios modos para dar mais contexto à IA.

- **CA-04.1** A aba **Assistente** nas configurações lista embutidos e
  personalizados.
- **CA-04.2** É possível criar, editar, duplicar e excluir um agente
  personalizado.
- **CA-04.3** Campos: nome, categoria, prompt de sistema, prompt de sugestão e
  contexto fixo.
- **CA-04.4** Nome é obrigatório; prompts têm limite de 16.384 caracteres;
  IDs embutidos são reservados.
- **CA-04.5** Se o prompt de sugestão ficar vazio, usa o contrato do agente
  Reunião + o prompt de sistema como critérios de domínio.
- **CA-04.6** Excluir o agente selecionado volta a seleção para “Reunião”.
- **CA-04.7** As configurações são validadas e persistidas em `settings.json`
  (sem segredos).

### US-05 — Markdown renderizado e código colorido

Como usuário, quero ler as respostas como documentos, com código legível.

- **CA-05.1** O renderer suporta headings, listas, tabelas (GFM), blockquotes,
  código inline e blocos cercados com linguagem.
- **CA-05.2** Blocos de código usam highlight.js e têm botão **Copiar**.
- **CA-05.3** HTML bruto não é renderizado; links abrem no navegador externo.
- **CA-05.4** As bolhas de sugestão da IA no chat também renderizam Markdown.
- **CA-05.5** O resumo e a sugestão usam o mesmo componente `MarkdownView`.

### US-06 — Privacidade, sessão e estados

- **CA-06.1** Nenhum backend, login ou telemetria (BR-002, BR-004).
- **CA-06.2** A transcrição só é enviada ao provider configurado pelo usuário
  (BR-005); chaves nunca aparecem na UI/logs (BR-003).
- **CA-06.3** Pausar a sessão para o automático; retomar reativa. Encerrar mantém
  o último resumo/sugestão visível.
- **CA-06.4** Trocar de sessão limpa e recarrega o estado das janelas.
- **CA-06.5** O clique no overlay continua atravessando fora das regiões
  interativas; as janelas novas são interativas por natureza.

## 5. Requisitos não funcionais

- **RNF-01 — Testabilidade:** serviços puros com `FakeLlmController`; repositórios
  com dobra em memória; componentes com `@testing-library/react`.
- **RNF-02 — Desempenho:** uma requisição em voo por janela; ticks sem conteúdo
  novo não chamam o LLM; Markdown memoizado; janelas separadas não re-renderizam
  o overlay.
- **RNF-03 — Custo:** resumo com 24k caracteres/chamada e no máximo 3 chunks por
  tick; sugestão com 6k caracteres; intervalos configuráveis e pausáveis.
- **RNF-04 — Segurança:** prompts tratam a transcrição como **dado**, nunca como
  instrução; Markdown sem HTML bruto; links externos.
- **RNF-05 — Paridade:** comportamento e estrutura dos prompts do Perssua
  (BR-015), com desvios documentados em
  [`architecture/assistant-windows.md`](architecture/assistant-windows.md) §5.
- **RNF-06 — Documentação única:** IDs `BR-023`…`BR-026`, `ISS-022`, `NN-task`
  (R14/BR-016).

## 6. Não-objetivos

- Captura de tela/OCR (o Perssua usa; o GhostWhisper segue áudio-only nesta PRD).
- Streaming token a token nas janelas.
- Envio do resumo por e-mail ou compartilhamento externo.
- Edição manual do resumo dentro da janela.
- Múltiplas sessões simultâneas nas janelas.
- Busca full-text dentro do resumo/sugestões.
- Substituir o provider de LLM por um modelo local embutido (endpoint custom
  continua valendo).

## 7. Riscos

| Risco | Mitigação |
|---|---|
| Custo/latência de chamadas periódicas | só com conteúdo novo; intervalos; pausa; limites de contexto/chunks |
| Resumo alucinar decisões/owners | contrato de grounding do Perssua; resumo anterior como baseline; “não invente” |
| Prompt injection pela transcrição | fonte tratada como dado; sem ferramentas; instruções explícitas |
| Markdown inseguro | `react-markdown` sem `rehype-raw`; links externos; highlight.js |
| Janelas visíveis no compartilhamento | BR-006: no Linux não há exclusão de captura; documentar |
| LeetCode sem tela | enunciado falado + “Contexto extra”; OCR fica para o futuro |
| Wayland/posicionamento | mesmo caminho X11/XWayland do overlay (BR-007) |
| Respostas longas | `max_tokens`, rolagem interna, bolha compacta no chat |

## 8. Métricas de sucesso

- Resumo atualiza em até ~30 s após fala nova, sem duplicar conteúdo e sem perder
  o começo em sessões longas.
- Sugestão de LeetCode gera Big O + código colorido a partir de uma pergunta
  falada.
- Nenhuma sugestão automática aparece no export sem o usuário ter clicado em
  **Fixar**.
- `cargo test`, `cargo clippy`, `cargo fmt --check`, `pnpm test`, `tsc --noEmit`
  e `pnpm build` verdes.

## 9. Evidência no Perssua (BR-015)

Extraído de `examples/Perssua-0.27.5-x86_64.AppImage` → `resources/app.asar`.

| Assunto | Arquivo no bundle | Âncora |
|---|---|---|
| Contrato do resumo | `electron/prompts/notetakerSummaryPrompt.js` | prompt completo |
| Perguntas sobre a reunião | `electron/prompts/notetakerMeetingQuestionPrompt.js` | contrato de Q&A |
| Agentes embutidos (Note Taker e Cody) | `electron/flavors.js` | `NOTETAKER_FLAVOR_ASSISTANT`, `CODY_FLAVOR_ASSISTANT` |
| Prompt incremental do resumo | `build/static/js/main.f180d025.js` | “You are updating a live Session Summary…” |
| Intervalo de 30 s e “só conteúdo novo” | `build/static/js/main.f180d025.js` | `setInterval(...,3e4)` / `$s.current` |
| Campos de assistente personalizado | `build/static/js/assistantsWindow.8c21ae6c.js` | `systemPrompt`, `realtimePrompt`, `context`, `category` |

Detalhes de transposição e desvios em
[`architecture/assistant-windows.md`](architecture/assistant-windows.md).
