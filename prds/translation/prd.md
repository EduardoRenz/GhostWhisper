# PRD — Tradução na bolha (sob demanda)

- **Versão:** 0.1
- **Data:** 2026-10-18
- **Status:** T1 implementado (aguardando teste manual); T2 pendente
- **Relacionados:** [`../business-rules.md`](../business-rules.md),
  [`memory.md`](memory.md), [`issues.md`](issues.md),
  [`tasks/_tasks.md`](tasks/_tasks.md) · PRDs anteriores:
  [`../mvp/prd.md`](../mvp/prd.md), [`../chat/prd.md`](../chat/prd.md)

---

## 1. Objetivo

Traduzir **a fala que o usuário quer**, dentro da própria bolha, sem um painel
duplicado embaixo da conversa. E, no futuro próximo, **não traduzir quando o
idioma detectado já é o de destino**.

## 2. Problema

- O painel de tradução antigo (`TranslationView`) repetia o texto original embaixo
  da conversa — visualmente um "segundo chat" ruim de ler e de testar.
- O gate de tradução era estático (`target !== source`). Com origem **Auto** e
  destino **português**, ele traduzia pt→pt mesmo quando a fala já era português,
  porque o app não conhecia o idioma detectado.

## 3. Entregáveis

| # | Entregável | Escopo | Task | Teste |
|---|-----------|--------|------|-------|
| **T1** | Tradução na bolha, sob demanda | botão/ícone no canto superior direito da bolha; clique traduz; clique de novo volta ao original; painel duplicado removido; sem botão de copiar por tradução | [`tasks/01-task.md`](tasks/01-task.md) | conversa com o botão ligado/desligado |
| **T2** | Gate por idioma detectado | o passe final do Whisper informa o idioma; a bolha não oferece tradução quando `detectado == destino` | [`tasks/02-task.md`](tasks/02-task.md) | fala pt com destino pt |

## 4. Histórias e critérios de aceite

### US-01 — Traduzir uma fala específica (T1)

- **CA-01.1** Cada bolha de **fala** (ME/THEY) mostra um ícone de tradução no
  canto superior direito quando `translation_enabled` e o destino ≠ `auto`.
- **CA-01.2** O primeiro clique pede a tradução e mostra o resultado **abaixo** do
  original, separado por **linha tracejada**, em **itálico e menor**.
- **CA-01.3** Um novo clique **esconde** a tradução (switch de volta); o botão
  indica o estado ligado.
- **CA-01.4** A tradução é **cacheada por fala**; clicar de novo não chama o LLM.
- **CA-01.5** Erros aparecem de forma discreta; a bolha continua legível.
- **CA-01.6** O painel de tradução embaixo da conversa **não existe mais**.
- **CA-01.7** Bolhas de sugestão da IA e falas em andamento (prévia) não têm o
  botão.
- **CA-01.8** Se a tradução for igual ao original (normalizando caixa/pontuação),
  nada é exibido (guarda pt→pt).

### US-02 — Não traduzir o que já está no destino (T2)

- **CA-02.1** O passe final do ASR carrega o idioma detectado (`auto`).
- **CA-02.2** Quando o idioma detectado (normalizado, `pt-br`→`pt`) é igual ao
  destino, o botão não é oferecido (ou fica desabilitado com explicação).
- **CA-02.3** Com idioma de origem fixo, o comportamento atual é preservado.
- **CA-02.4** Detecção duvidosa **não** esconde tradução necessária: o gate é
  conservador (só omite quando tem certeza do idioma).

## 5. Requisitos não funcionais

- **RNF-01 — Testabilidade:** hook puro com `llmTranslate` mockado; componente com
  `@testing-library/react` (R7/R8/BR-013).
- **RNF-02 — Custo:** uma tradução por fala, sob demanda e cacheada; fila serial.
- **RNF-03 — Privacidade:** a fala só vai ao provider quando o usuário clica
  (BR-005); nada de tradução automática em background.
- **RNF-04 — Sem regressão de click-through:** o botão marca `data-interactive`
  (overlay X11).
- **RNF-05 — Documentação única:** `BR-030`, `ISS-029`/`ISS-030`, `NN-task` (R14).

## 6. Não-objetivos

- Tradução automática de todas as falas (volta como opção madura no futuro).
- Tradução das sugestões da IA.
- Detecção de idioma por texto no cliente (usamos o que o Whisper já detecta).
- Persistir a tradução no banco/export (por ora é efêmera).

## 7. Riscos

| Risco | Mitigação |
|---|---|
| Detecção ruim com `tiny`/áudio curto | gate conservador + guarda de identidade; melhora com `large-v3-turbo` (E3) |
| Cliques repetidos estourarem rate limit | fila serial + cache por id |
| Botão atrapalhar o clique no overlay | `data-interactive` só no botão; resto continua click-through |
| Tradução longa "esticar" a bolha | `font-size` menor, itálico e quebra de palavra |

## 8. Métricas de sucesso

- Nenhum painel de tradução duplicado na conversa.
- Uma fala traduzida em um clique; segundo clique volta ao original.
- Com origem Auto/destino pt e fala pt, **nenhuma** tradução é oferecida (T2).

## 9. Evidência (whisper-rs)

- `WhisperState::full_lang_id_from_state()` e `whisper_rs::standalone::get_lang_str(id)`
  devolvem o idioma do último `full()` (ex.: `"pt"`), sem chamada extra.
