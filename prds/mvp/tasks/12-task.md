# 12 — Configurações em abas, chave redactada e compat com providers

- **Status:** CONCLUDED
- **Depende de:** 06, 08
- **Habilita:** —

## Objetivo
Três ajustes pedidos pelo usuário:
1. Janela de configurações em **estilo abas** (referência Perssua: `settings.tabs`,
   com seções por escopo);
2. Chave de API exibida **redactada** no aba Fontes de IA (indicar que existe,
   sem expor o valor);
3. Corrigir o **400 `unsupported_value` de `temperature`** nos providers
   OpenAI-modelos novos (ChatGPT/gpt-5, o1, o3, o4).

## Entregáveis
- `features/settings/SettingsPage.tsx` — tablist (Fontes de IA, Transcrição &
  Áudio, Ajustes, Atalhos); chave mascarada `•••••••·abcd` somente leitura;
  "Trocar chave" reabre o form; AudioTestPanel na aba de Transcrição.
- `src-tauri/src/adapters/ai/openai_compatible.rs` —
  `rejected_param` reconhece `unsupported_value`/`invalid_value` (além de
  `unsupported_parameter`); `apply_rejected_param_fallback` remiti só parâmetros
  opcionais de amostragem; `temperature_supported` omite `temperature` para
  gpt-5*/o1*/o3*/o4* (Perssua: `providerParameterErrors.js` +
  `providerParameterContracts.js` + `builtInOmissions`).
- `domain/settings.rs` — `redact_secret` (regra pura de mascaramento).
- `ipc/settings_commands.rs` — `settings_key_preview` (devolve apenas a máscara).
- `styles/globals.css` — classes `settings__nav/tab/key`.

## Critérios de aceite
- [x] Abas renderizam só a seção ativa; status global no rodapé.
- [x] Chave configurada: campo readonly mascarado (últimos 4; curtas só balas).
- [x] Segredo inteiro nunca sai do backend nem entra em logs.
- [x] `temperature` não é enviada para gpt-5*/o*; erro `unsupported_value`
      para outros modelos é recuperado omitindo o parâmetro e reenviando.
- [x] `cargo test` 122 ✓ · `clippy`/`fmt` ✓ · `pnpm test` 22 ✓ · `pnpm build` ✓.

## Verificação
- Testes novos: JSON exato do erro do usuário; sequência mock 400→200;
  `omits_temperature_for_reasoning_models`; redact (normal/curta/vazia);
  4 testes do componente SettingsPage (mocks de IPC).
- Issues: ISS-011 (temperature) e ISS-012 (abas/redact) em `prds/mvp/issues.md`.

## Pendências
- (flexível) Gemini: o mesmo tratamento de omissão pode ser estendido quando
  houver relato real (Perssua aplica `builtInOmissions` para gemini-3.x).
