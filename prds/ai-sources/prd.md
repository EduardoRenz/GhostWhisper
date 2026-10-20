# PRD — Fontes de IA múltiplas e Configurações repaginadas

- **Versão:** 0.1
- **Data:** 2026-10-19
- **Status:** Implementado (tasks 01–09 `CONCLUDED`)
- **Relacionados:** [`../business-rules.md`](../business-rules.md),
  [`memory.md`](memory.md), [`issues.md`](issues.md),
  [`architecture/ai-sources.md`](architecture/ai-sources.md),
  [`architecture/settings-redesign.md`](architecture/settings-redesign.md),
  [`tasks/_tasks.md`](tasks/_tasks.md), PRDs anteriores:
  [`../mvp/prd.md`](../mvp/prd.md), [`../assistant/prd.md`](../assistant/prd.md)

---

## 1. Objetivo

Duas entregas que andam juntas:

1. **Fontes de IA múltiplas** — o usuário passa a cadastrar **várias** fontes de
   IA (provedor + modelo + chave), a reordenar para escolher qual é a
   **primária**, e a contar com **fallback automático** (com *circuit breaker*)
   quando a primária falhar. **DeepSeek** entra como provedor de primeira
   classe.
2. **Tela de Configurações repaginada** — a tela atual (5 abas, um seletor de
   provider, um campo de chave, um botão "Salvar" por aba) vira uma casca nova:
   cabeçalho próprio, barra de abas com ícones, conteúdo em *cards* e um
   **rodapé fixo** com `Salvar Alterações` / `Restaurar Padrões`.

Além disso, o modelo de dados fica **preparado** para a PRD seguinte: vincular
uma fonte específica a um agente ou a uma tela (rota por escopo). O vínculo em
si **não** tem UI nesta entrega.

## 2. Problema

Hoje o GhostWhisper tem **uma** fonte de IA. `Settings` guarda `provider`,
`model` e `base_url`; o `SecretStore` guarda **uma** chave por *tipo* de
provider (`llm.openai`, `llm.gemini`, …); e o `LlmRouter` resolve sempre essa
única configuração. Disso decorrem três problemas reais:

- **Sem resiliência.** Se a OpenAI estiver com rate limit (429) ou a chave
  estiver inválida (401), a sugestão, o resumo e a tradução **falham** — não há
  para onde cair.
- **Sem comparação.** O usuário não pode deixar duas contas do mesmo provedor
  (ex.: uma pessoal e uma da empresa) nem trocar de modelo sem apagar a
  configuração anterior.
- **Uma chave por tipo trava o uso.** Duas fontes OpenAI (modelos diferentes)
  disputariam a mesma entrada do keyring.

A tela de Configurações reflete a mesma limitação: um `<select>` de provider, um
`<input>` de modelo, um bloco de chave — e nenhum sinal de saúde (latência,
conexão, fallback).

## 3. Público-alvo e casos de uso

Mesmas personas do MVP ([`../mvp/prd.md`](../mvp/prd.md) §3), com foco em:

- **CU-01 — Nunca ficar sem sugestão:** a fonte primária falha (429/401/timeout)
  e o GhostWhisper responde pela próxima fonte da lista, sem interromper a
  sessão.
- **CU-02 — Escolher a melhor fonte por momento:** reunião com a OpenAI no
  limite de uso → o usuário promove o Gemini a primário (um clique) e volta a
  ordem depois.
- **CU-03 — Várias contas do mesmo provedor:** cadastrar "OpenAI · Pessoal"
  (`gpt-4o-mini`) e "OpenAI · Empresa" (`gpt-4.1`) com **chaves diferentes**,
  cada uma no keyring.
- **CU-04 — Entender o estado:** ver na própria tela qual fonte está conectada,
  qual está em fallback e qual está falhando, com a latência medida.
- **CU-05 — Usar o DeepSeek:** cadastrar o DeepSeek com a chave dele, sem
  precisar de `base_url` manual.
- **CU-06 — Configurar sem medo:** um único "Salvar Alterações" no rodapé;
  "Restaurar Padrões" com confirmação explícita.

## 4. Escopo

### 4.1 Entregue

- Cadastro, edição, exclusão e **reordenação** de fontes de IA; a ordem da
  lista é a ordem de prioridade e a **primeira é a primária**.
- **Chave por fonte** no keyring (`llm.source.{id}`), com versão mascarada,
  "Validar chave", "Trocar chave" e "Remover chave".
- **Fallback automático** em todas as chamadas de IA (sugestões, resumo
  incremental, tradução ao vivo) com **circuit breaker** por fonte.
- **DeepSeek** como provedor de primeira classe.
- Sinal de saúde por fonte na tela (conectada + latência / degradada / em
  fallback / falhou).
- Tela de Configurações repaginada com salvamento global.
- Rotas por escopo **no modelo de dados** (fonte vinculada por agente ou por
  capacidade), sem UI de vínculo.

### 4.2 Fora do escopo (PRD futura)

- UI para vincular uma fonte a um agente ou a uma tela.
- **Anthropic**: a API `/v1/messages` não é compatível com OpenAI e exige um
  adapter nativo (formato de request/response e de streaming diferentes).
  Entra em PRD própria.
- Endpoint local como fonte de LLM (`custom` continua existindo para
  Ollama/LM Studio/vLLM, com `base_url` manual).
- Múltiplas chaves para a **mesma** fonte (uma chave por fonte, só).
- Sincronização das configurações entre máquinas (BR-002).

## 5. Regras de produto

As regras canônicas vivem em [`../business-rules.md`](../business-rules.md):

- **BR-044** — Fontes de IA múltiplas e ordenadas; a primeira é a primária.
- **BR-045** — Fallback automático com circuit breaker em todas as chamadas de IA.
- **BR-046** — DeepSeek como fonte de primeira classe.
- **BR-047** — Configurações repaginadas com salvamento global.
- **BR-048** — Rotas de IA por escopo (modelo pronto, UI depois).

Regras pré-existentes que continuam valendo: BR-002 (sem backend), BR-003 (BYO
key no keyring), BR-005 (áudio local), BR-008 (reconstruir comportamento, não
código), BR-011/BR-013 (hexagonal + testabilidade), BR-015 (Perssua como
referência), BR-016 (documentação como fonte única).

## 6. Referência: o que o Perssua faz (BR-015, R12)

Extraído do `Perssua-0.27.5-x86_64.AppImage` → `resources/app.asar`
(`electron/settingsStore.js`, `electron/modelMigrations.js`,
`build/static/js/main.f180d025.js`, `~/.config/Perssua/settings.json`):

| Comportamento | Perssua | GhostWhisper (esta PRD) |
|---|---|---|
| Fonte ativa | `current_provider` (uma só) | lista `ai_sources` ordenada; a primeira é a primária |
| Modelo por provedor | `openai_model`, `gemini_model`, `openrouter_model`, `custom_api_model` | `AiSource.model` (um por fonte cadastrada) |
| Chave | `*_api_key` no `settings.json` (texto puro) | `llm.source.{id}` no keyring (BR-003) |
| Fonte de credencial | `openrouter_credential_source` (`trial`/`personal`) | — (não há trial no GhostWhisper) |
| Rota por capacidade | `analysis_model` + `analysis_model_source`, `translation_model` + `translation_model_source` | `Settings.ai_routes` + `AiRouteScope` (BR-048) |
| Reordenação / fallback / circuit breaker | **não existe** | **BR-044 / BR-045** (desvio) |
| DeepSeek de primeira classe | **não existe** (aparece só em `electron/preload.js` como provider de web search) | **BR-046** (desvio) |

### Desvios conscientes (R12)

1. **Lista ordenada + fallback + circuit breaker.** O Perssua tem uma fonte
   ativa só e nenhum mecanismo de degradação. O desvio tem motivo técnico
   explícito: sem fallback, um 429 da OpenAI derruba sugestão, resumo e
   tradução — e o GhostWhisper é usado **durante** a reunião, quando não há
   tempo para abrir as configurações e trocar a chave.
2. **DeepSeek de primeira classe.** No Perssua o DeepSeek entraria pelo
   `custom` (endpoint OpenAI-compatível) e exigiria `base_url` digitada à mão.
   Aqui ele é um provedor do catálogo, com `base_url` e modelo padrão.
3. **Chave no keyring, não no `settings.json`.** O Perssua grava
   `openai_api_key` no arquivo de settings em texto puro. Isso **viola BR-003**;
   a transposição troca a tecnologia mantendo o comportamento *visível* (a chave
   nunca é devolvida à UI em claro).

## 7. Arquitetura (resumo)

Detalhe em [`architecture/ai-sources.md`](architecture/ai-sources.md) e
[`architecture/settings-redesign.md`](architecture/settings-redesign.md).

```
domain/ai_source.rs        AiSource, AiSourceHealth, validação
domain/ai_route.rs         AiRouteScope, AiRoute, resolve_route()
domain/settings.rs         Settings.ai_sources / ai_routes; ProviderKind::DeepSeek
domain/assistant.rs        AssistantMode.source_id

ports/ai_source_service.rs CRUD + chaves + saúde das fontes
ports/llm_controller.rs    complete_for(scope, …)
ports/event_publisher.rs   emit_llm_source_used

application/ai_source_health.rs    registro de saúde + circuit breaker
application/ai_source_service.rs   implementação da porta
application/ai_source_migration.rs settings/chaves legadas → fontes
application/llm_router.rs           cadeia de fallback

adapters/ai/deepseek_chat_bot_provider.rs
ipc/ai_source_commands.rs

src/features/settings/AiSourcesSettings.tsx
src/features/settings/ProviderCard.tsx
src/features/settings/AddProviderForm.tsx
```

## 8. Critérios de sucesso

- Cadastrar 3 fontes (OpenAI, Gemini, DeepSeek), reordenar e ter a terceira como
  primária em menos de 30 s.
- Com a primária returning 429, a sugestão seguinte responde pela segunda
  fonte **sem erro na UI**.
- Uma fonte com 401 não é retentada a cada requisição (circuito abre).
- Duas fontes do mesmo provedor usam **chaves diferentes** no keyring.
- Settings antigas (com `provider`/`model`) continuam funcionando após o
  update, com a chave legada preservada.
- A tela de Configurações tem um único "Salvar Alterações" no rodapé.

## 9. Riscos

| Risco | Mitigação |
|---|---|
| Fallback silencioso mascarar uma configuração ruim | Evento `llm_source_used` com `is_fallback` + badge "Fallback Ativo" na tela |
| Latência acumulada quando todas as fontes caem | Circuit breaker + circuit aberto pula a fonte sem gastar timeout |
| Vazamento de chave em log/erro | Segredos só no keyring; erro de provider tipado e truncado (`ProviderStatus`) |
| Regressão em quem já tinha `provider`/`model` | Migração idempotente no boot + teste de settings legadas |
| Escopo grande demais | 9 tasks independentes, com a casca da tela (07) separada das fontes (08) |
