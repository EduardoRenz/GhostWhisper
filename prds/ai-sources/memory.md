# Histórico — PRD Fontes de IA

Histórico deste PRD no formato fixo do
[`rules.md`](../../.agents/rules.md) §6.2 (busca por palavra-chave, ID e período).

| Objetivo | Comando |
|----------|---------|
| Palavra-chave | `rg -n -i 'fallback' prds/ai-sources/memory.md` |
| Entrada do dia | `rg -n '^## 2026-10-19' prds/ai-sources/memory.md` |
| Por ID | `rg -n 'ISS-039\|BR-044' prds/ai-sources/memory.md` |

---

## 2026-10-19 — PRD Fontes de IA: múltiplas fontes, fallback e Configurações repaginadas

- **IDs:** BR-044, BR-045, BR-046, BR-047, BR-048, T01–T09
- **Refs:** [`prd.md`](prd.md), [`issues.md`](issues.md),
  [`architecture/ai-sources.md`](architecture/ai-sources.md),
  [`architecture/settings-redesign.md`](architecture/settings-redesign.md),
  [`tasks/_tasks.md`](tasks/_tasks.md)

### Como a PRD nasceu
O usuário mandou o mockup da nova tela de Configurações e pediu, junto, um
passo grande nas IAs: **cadastrar várias IAs**, **reordenar** (a primeira é a
primária), **circuit breaker com fallback** quando a primária falha, e
**DeepSeek** como provedor de primeira classe. Deixou claro que a
personalização por agente/tela fica para depois, mas que a aplicação deve ficar
**preparada** para isso.

### Decisões do usuário
- Fallback automático em **todas** as chamadas de IA (sugestão, resumo,
  tradução).
- **Manter as 5 abas atuais** — a repaginação é visual (header, tabs com
  ícone, cards, rodapé fixo), não uma nova taxonomia de seções.
- **Só DeepSeek** agora; Anthropic exige adapter nativo e fica para outra PRD.
- **Whisper fora da lista de fontes** (é ASR local, não fonte de LLM).
- **Um "Salvar Alterações" global** no rodapé (sai o botão por aba).

### O que foi extraído do Perssua (0.27.5)
- `electron/settingsStore.js` — `current_provider` (fonte única),
  `openai_model`/`gemini_model`/`openrouter_model`/`custom_api_model`,
  `openrouter_credential_source`, chaves em `settings.json`.
- `build/static/js/main.f180d025.js` — rota por capacidade
  (`analysis_model` + `analysis_model_source`, `translation_model` +
  `translation_model_source`).
- `~/.config/Perssua/settings.json` — chaves de API em **texto puro** (viola
  BR-003 ⇒ transpomos para o keyring).

### Desvios conscientes do Perssua (R12)
1. Lista ordenada + fallback + circuit breaker — **não existem** lá; motivo
   técnico: sem fallback, um 429 derruba sugestão/resumo/tradução **durante**
   a reunião.
2. DeepSeek de primeira classe — lá só entraria pelo `custom` com `base_url`
   digitada à mão.
3. Chave no keyring, não em texto puro no arquivo de settings.

### Entregue
- **T01** domínio de fontes (`domain/ai_source.rs`, `domain/ai_route.rs`),
  `ProviderKind::DeepSeek`, `Settings.ai_sources`/`ai_routes`.
- **T02** migração idempotente das settings e das chaves legadas.
- **T03** serviço de fontes + registro de saúde + circuit breaker.
- **T04** roteador com cadeia de fallback, latência e evento `llm_source_used`.
- **T05** adapter DeepSeek + `GhostWhisperError::ProviderStatus`.
- **T06** IPC das fontes + wiring no composition root.
- **T07** casca da tela (header, tabs com ícone, cards, rodapé fixo).
- **T08** aba "Fontes de IA" (cards, reordenar, promover, chaves, saúde).
- **T09** demais abas no layout novo, `complete_for` nos serviços, docs.

### Detalhes que valem lembrar
- **Chave por fonte, não por tipo:** `llm.source.{id}` no keyring. A chave
  legada (`llm.openai`) é **copiada** na migração e **não** é apagada.
- **`provider`/`model`/`base_url` viraram campos legados** de leitura
  (`serde(rename + skip_serializing)`): o `sanitized()` migra para uma fonte e
  o arquivo de settings nunca mais escreve esses campos.
- **Circuit breaker:** 3 falhas ⇒ aberto por 60 s; `401/403` abre na hora;
  depois do cooldown a fonte entra em meio-aberto. Erro de status virou
  **tipado** (`GhostWhisperError::ProviderStatus { code, detail }`) para o
  breaker classificar por código, sem parsear a mensagem.
- **O streaming não troca de fonte depois de emitir texto** (evita texto
  duplicado na bolha).
- **Escopo por rota (BR-048):** o serviço de sugestão pede
  `AiRouteScope::Agent(mode_id)`, o resumo pede
  `Capability(Summary)` e a tradução `Capability(Translation)`. Sem rota
  configurada o comportamento é o de sempre (a primária).
- **Um caminho de escrita só:** a lista de fontes é salva com o resto das
  settings (`settings_set`); a porta `AiSourceService` cuida de leitura,
  chaves e saúde.
- **"Restaurar Padrões"** volta o rascunho aos padrões de fábrica (com
  confirmação) e **não** apaga as chaves no keyring.

### Verificação
- `cargo test` (438), `cargo clippy`, `cargo fmt --check`.
- `pnpm test` (284), `tsc --noEmit`, `pnpm build`.
- Tela conferida em navegador headless (5 abas).

---

## 2026-10-19 — Ajustes pedidos pelo usuário após o uso real

- **IDs:** BR-049, BR-050, ISS-039
- **Refs:** [`prd.md`](prd.md), [`architecture/ai-sources.md`](architecture/ai-sources.md),
  [`../business-rules.md`](../business-rules.md)

### 1) Sem catálogo de modelos (BR-049)
O usuário pediu para poder digitar o modelo. `PROVIDER_MODELS` saiu do frontend: o campo
de modelo é **livre** no formulário de adição e no editor do card, e o botão
"Adicionar" só habilita com o modelo preenchido. `PROVIDER_DEFAULT_MODELS`
também saiu (virou apenas `PROVIDER_DEFAULT_BASE_URLS`).

### 2) Chave não aparecia no DeepSeek — bug corrigido
**Causa:** o serviço de fontes exigia que a fonte **existisse no arquivo de
settings** para derivar a chave do keyring. Uma fonte recém-adicionada ainda
está só no **rascunho** da tela ⇒ o comando respondia "fonte de IA
desconhecida" e a chave nunca era gravada (nem a prévia aparecia).

**Correção:** a chave no keyring é indexada pelo **id** da fonte
(`ai_source::secret_key_for(id)`), que o frontend gera no rascunho — não há
mais dependência do arquivo. A **validação** passou a receber a fonte
inteira (`validate(&AiSource)`) para validar uma fonte ainda não salva.

### 3) Log de debug do modelo em uso (BR-050)
Cada chamada emite `tracing::debug!` com escopo, fonte, provider, **modelo**,
`max_tokens`, `temperature`, latência, tokens e o **nível de raciocínio**.
Para isso: `domain::llm::CompletionUsage` (com `reasoning_tokens`), parse de
`usage` (OpenAI-compatível) e `usageMetadata`/`thoughtsTokenCount` (Gemini) nos
adapters, e o helper `reasoning_level`. `RUST_LOG=debug` para ver.

### 4) Badge "Nunca usada" persistia (ISS-040)
Duas causas: (a) a janela de Configurações **nasce oculta** — o React monta no
boot e buscava a saúde **uma vez só**, antes de qualquer uso; (b) o caminho de
**validação** registrava a saúde mas não emitia `llm_source_used`.
Correção: a tela **rebusca a saúde ao ganhar foco** (`onWindowFocus`) e a
validação passa a emitir o evento. O log de debug ficou num struct
(`ModelCallInfo`) para ser testável sem depender do subscriber global do
`tracing` (que era flaky com testes paralelos).

### 5) Onde ver o modelo que respondeu (BR-051)
O log de debug vai para o **stdout** (o terminal que lançou o app) e só no
nível `debug` — o filtro padrão é `info`, então sem `RUST_LOG` a linha é
descartada. Para o `.deb` instalado (sem terminal) isso não resolve nada.
Decisão do usuário: **mostrar na UI**.

- `LlmSourceUsage` ganhou `model`, `provider` e `scope` (o escopo da tarefa:
  `agent:<id>` / `capability:<nome>` / `default`) — o escopo permite que cada
  tela mostre o modelo **daquela** tarefa.
- `useLlmSource(scope)` (frontend) guarda o último uso, filtrado por escopo.
- `LlmSourceBadge` (componente único, no padrão do `AgentBadge`): pill com o
  modelo no card da sugestão; no resumo, a linha de metadados ganhou
  `fonte · modelo` (+ `(fallback)`).
- O **resumo** mostra o modelo da **última resposta** (não é persistido —
  sem migration de banco).
- README corrigido: `RUST_LOG=ghostwhisper_lib=debug` (o `debug` global faz o
  `keyring` despejar credenciais).



---

## 2026-10-20 — v0.2: aba "Geral & HUD" e acabamento visual (BR-062)

- **IDs:** BR-062, tasks 10–13
- **Refs:** [`prd.md`](prd.md) §10,
  [`architecture/settings-redesign.md`](architecture/settings-redesign.md) §9,
  [`tasks/10-task.md`](tasks/10-task.md) … [`tasks/13-task.md`](tasks/13-task.md),
  `domain/hud_layout.rs`, `HudSettings.tsx`

### Contexto
Mockup do usuário: a tela de Configurações deve ficar mais próxima da referência
e o comportamento do HUD ganha uma aba própria ("Geral & HUD") no lugar de
"Ajustes", com opacidade, ancoragem, dwell-hover e densidade.

### Decisões
- **5 abas mantidas** (decisão do usuário): só "Ajustes" → **"Geral & HUD"**; as
  6 abas do mockup ficam para outra PRD (§10.2).
- **Dwell**: faixa do mockup, **100 ms–1,5 s**, padrão continua **500 ms**.
- **Opacidade**: 20 %–100 %, padrão 75 %, aplicada ao fundo do notch via
  `--hud-alpha` + canais RGB do token (`rgba(var(...), alpha)`).
- **Ancoragem**: `top_center` (padrão) · `top_right` · `free`; as fixas usam a
  **área útil** do monitor primário. Arrastar o notch **vira `free`** e persiste.
- **Densidade**: `compact` 62 px · `standard` 72 px · `expanded` 84 px de janela
  (barra 44/52/60 e padding/gap por variável CSS).
- Geometria pura em `domain/hud_layout.rs`, sem I/O; o adaptador só pergunta ao SO.

### UI
- `HudSettings.tsx` com cabeçalho de seção + `HUD Anchor:` vivo, grade 2×2,
  segmentados (âncora/densidade), toggle (dwell/stealth), sliders com badge e
  legendas; card de privacidade atravessa as colunas.
- Casca: abas com **sublinhado** na ativa (em vez de pílula) e novos estilos
  (`.settings__grid`, `__section-header`, `__segmented`, `__toggle`, `__badge`).

### Verificação
- `cargo test --lib` **519 ✓** · `clippy -D warnings` ✓ · arquivos tocados
  formatados (o `cargo fmt --check` global ainda acusa dívida **pré-existente**
  em arquivos não tocados).
- `pnpm test` **339 ✓** (55 arquivos) · `tsc --noEmit` ✓ · `pnpm build` ✓.
- **Pendente**: validação visual do usuário (mockup + notch real).

## 2026-10-21 — Configurações sem moldura nativa (BR-064/T17)

- **IDs:** BR-064, T17 (chat)
- **Refs:** [`architecture/settings-redesign.md`](architecture/settings-redesign.md)
  §1, [`../chat/tasks/17-task.md`](../chat/tasks/17-task.md)

- A janela `settings` agora é `decorations: false`, `transparent: true` e
  `shadow: false`; o header próprio é a única barra de título, usando o
  `WindowHeader` comum (grip à esquerda, selo + título/subtítulo, `Esc` e `×`).
- O conteúdo vive em um shell transparente de 8 px, com borda/raio
  (`--oh-window-radius`) e `ResizeHandles`; o fechar continua chamando
  `settings_close` e o drag chama `panel_start_drag`.
- Sem mudança no rodapé, nas abas ou no rascunho global.

## 2026-10-21 — Opacidade global das janelas e autosave (task 15)

- **IDs:** BR-062, T15
- **Refs:** [`prd.md`](prd.md) §10,
  [`architecture/settings-redesign.md`](architecture/settings-redesign.md) §9,
  [`tasks/15-task.md`](tasks/15-task.md),
  [`../notch/architecture/design-tokens.md`](../notch/architecture/design-tokens.md)

### Pedido
- A opacidade da aba "Geral & HUD" deixar de valer só no notch e refletir em
  **todas as telas**.
- Aplicar em tempo real, sem esperar o "Salvar Alterações"; autosave permitido.

### Decisão/entrega
- `useHudOpacity` no `App` de cada webview aplica `--hud-alpha` no `<html>`;
  o valor vem do campo legado `overlay_opacity` (sem migration).
- Superfícies de notch, painéis, Configurações (shell/cards/rodapé), Perguntar,
  dwell e prévia do print usam canais RGB + `--hud-alpha`; texto/controles
  continuam opacos.
- O slider publica `settings_preview_overlay_opacity` (evento
  `hud_opacity_preview`) e agenda `settings_set_overlay_opacity` com debounce
  de 200 ms. O save global continua para os demais campos; um save completo
  também publica o preview.
- `settings_set_overlay_opacity` grava **somente** a opacidade, sem reaplicar
  atalhos/geometria, e emite preview + `settings_changed`.

### Verificação
- `pnpm test` **363 ✓** · `tsc --noEmit` ✓ · `pnpm build` ✓.
- `cargo test --lib` **545 ✓** · `cargo clippy --lib -D warnings` ✓; rustfmt do
  arquivo Rust tocado.
- Validação visual no navegador em 20/75/100 % sobre fundo magenta, em
  Configurações e Chat.
- AppImage no Xvfb: Configurações sem moldura nativa, slider 40→85→50→40 %
  persistido sem "Salvar" (overlay/Histórico abertos); pacotes às 11:13.

## 2026-10-22 — Escala de fonte da UI com bônus para a IA (task 16)

- **IDs:** BR-087, T16
- **Refs:** [`tasks/16-task.md`](tasks/16-task.md),
  [`../business-rules.md`](../business-rules.md) BR-087,
  [`../notch/architecture/design-tokens.md`](../notch/architecture/design-tokens.md) §3–4

### Pedido
- Ajuste em **Configurações → Geral & HUD** para aumentar/diminuir o tamanho
  relativo da fonte — **principalmente as respostas da IA**, "muito pequenas".
- Todos os elementos e botões podem escalar, mas têm que **crescer junto** e
  **não estourar**.

### Decisões do usuário
- **Dois controles**: escala global + bônus extra só para o texto de IA.
- Faixa **75–160 %**, passo de **5 p.p.**, padrão 100 % (bônus IA 100–150 %).
- Escala vale **só nas janelas de conteúdo**: notch e barra "Perguntar"
  mantêm tamanho/densidade próprios.

### Decisão/entrega
- Zoom de raiz (`font-size` no `<html>`) + tokens de tipografia convertidos
  de `px` para **`rem`**: fonte, entrelinha e espaçamento crescem juntos.
  Base do sistema capturada uma vez (o zoom nunca se acumula); 100 % remove a
  propriedade.
- Bônus de IA como `--oh-ai-font-scale`, aplicado num bloco **no fim** de
  `globals.css` (a ordem vence a regra original) sobre as superfícies
  geradas pela IA.
- IPC espelhando a opacidade: `settings_preview_font_scales` /
  `settings_set_font_scales` + evento `font_scales_preview`, autosave com
  debounce de 200 ms e flush ao desmontar; `settings_set` publica o preview.
- `useFontScale(enabled)` no `App`; `enabled = route !== "overlay" &&
  route !== "ask"`.
- Auditoria de estouro: `font-size` em `px` → `rem` (7 ocorrências) e
  alturas de controles **com texto** → `rem` (`.session-pill`,
  `.composer__input`, `.composer__remove`); ícones continuam em `px`.

### Verificação
- `pnpm test` **453 ✓** (70 arquivos) · `tsc --noEmit` ✓.
  Guardas novas: `src/styles/font-scale.test.ts` (6) e
  `useFontScale.test.tsx` (6).
- `cargo test` **657 ✓** (10 ignorados) · `cargo clippy --all-targets` sem
  avisos · `cargo fmt --check` ✓.
- Faixas travadas nos dois domínios (0,75–1,6 / 1,0–1,5), NaN → default e
  settings legadas sem os campos carregam em 100 %.

## 2026-10-28 — Reordenar as fontes de IA no arraste (ISS-086, task 17)

- **IDs:** ISS-086, BR-044, BR-045, T17
- **Refs:** [`issues.md`](issues.md) ISS-086,
  [`tasks/17-task.md`](tasks/17-task.md),
  [`architecture/ai-sources.md`](architecture/ai-sources.md) §ordem/fallback

### Pedido
- "O drag das seleções de IA nas configurações funciona para arrastar, mas
  **não é possível reordenar**."
- A regra é a **ordem de execução/fallback**: a primeira fonte cadastrada é a
  primária; se falhar, cai na segunda, com **circuit breaker na sessão**.

### Diagnóstico
- A regra já existia no core: `LlmRouter::candidates` percorre
  `settings.ai_sources` na ordem e `complete_for` pula circuito aberto; o
  `AiSourceHealthRegistry` é **em memória** (por sessão), 3 falhas/60 s, e
  401/403 abre na hora. `sanitize_sources`/`ai_sources_save` preservam a ordem.
  Nada a mudar no Rust.
- O defeito era de **runtime**: no WebKitGTK o `DropTargetGt4` aborta o arraste
  sem dados carregados, então `dragstart` dispara (o card "arrasta") mas
  `dragover`/`drop` nunca chegam ao DOM. O código não chamava
  `DataTransfer.setData` — bug [WebKit 265857][webkit-265857], ainda NEW. Os
  testes existentes passavam porque disparavam os eventos na mão.

### Decisão/entrega
- `setData("text/plain", id)` + `effectAllowed`/`dropEffect` = `move` +
  `setDragImage` no card; `dragleave` limpa o destaque do alvo.
- Ordenação movida para o domínio puro: `moveSource` (arraste),
  `moveSourceBy` (↑/↓) e `makePrimarySource`; lista idêntica **não** chama
  `onChange`.
- Alça virou botão focável com `↑`/`↓` (alternativa ao mouse e plano B se o
  WebKitGTK voltar a atrapalhar); l10n em pt/en.
- Regressão: o teste exige `setData` e é o que falha sem a correção.

### Verificação
- `pnpm test` **537 ✓** (82 arquivos) · `tsc --noEmit` ✓.
  Guardas novas: `settings.test.ts` (5 casos de ordem) e
  `AiSourcesSettings.test.tsx` (5, incluindo a regressão do `setData`).
- Mutação conferida: removendo o `setData`, o teste novo falha.
- Verificação manual (arrastar + salvar + reiniciar no binário Linux) pendente.

[webkit-265857]: https://bugs.webkit.org/show_bug.cgi?id=265857

## 2026-10-28 — Circuit breaker rígido e prazos curtos por escopo (task 18)

- **IDs:** BR-045, T18
- **Refs:** [`../business-rules.md`](../business-rules.md) BR-045,
  [`architecture/ai-sources.md`](architecture/ai-sources.md) §4.1/§4.1.1,
  [`tasks/18-task.md`](tasks/18-task.md)

### Pedido
- Depois de validar a reordenação: "o circuit breaker tem que ser mais
  rigoroso, pois exigimos resposta rápida de IA. Coloca um timeout mais curto,
  e basta uma falha para ativar o segundo modelo por 1 min."

### Decisão/entrega
- `FAILURE_THRESHOLD` **3 → 1**: qualquer falha (timeout, 5xx, 429, resposta
  vazia) abre o circuito por **60 s** e a chamada seguinte começa na reserva.
  `Degraded` fica reservado (não é produzido com o limiar em 1).
- Prazos por escopo (`application/llm_timeouts.rs`): **15 s** no caminho
  rápido (Default, agente, sugestões) e **60 s** em resumo/tradução. O prazo
  viaja em `CompletionRequest.timeout` e é aplicado no adaptador HTTP
  (`request_timeout`); prazo explícito do chamador vence a política.
- Validação de chave (roteador e serviço de fontes) usa o prazo rápido.
- Dica da aba "Fontes de IA" passa a dizer que a reserva assume **por 1 min**.

### Verificação
- `cargo test` **861 ✓** (11 ignorados) · `cargo clippy --all-targets` sem
  avisos · `cargo fmt --check` ✓.
- `pnpm test` **537 ✓** · `tsc --noEmit` ✓.
- Regressão conferida por mutação: com `FAILURE_THRESHOLD = 3`, os testes
  `one_failure_opens_the_circuit` e
  `one_failure_already_skips_the_primary_on_the_next_call` falham.
