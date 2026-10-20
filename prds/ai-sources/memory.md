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
- `cargo test` (430), `cargo clippy`, `cargo fmt --check`.
- `pnpm test` (282), `tsc --noEmit`, `pnpm build`.
- Tela conferida em navegador headless (5 abas).

