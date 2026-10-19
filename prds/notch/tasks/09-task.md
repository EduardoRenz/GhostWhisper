# 09 — Settings do drill-hover (ligado/durada)

- **Status:** CONCLUDED
- **Depende de:** —
- **Habilita:** 10
- **Issues:** — · **BR:** BR-036

## Objetivo

Adicionar as configurações do drill-hover ao `Settings` (Rust + TS) e expô-las em
**Configurações → Ajustes**: ligar/desligar e escolher a duração. Ainda **sem**
comportamento de hover na UI.

## Escopo

- `dwell_enabled: bool` — padrão **`true`**.
- `dwell_ms: u32` — padrão **500**, faixa **300–2000** (passo 100 na UI).
- `sanitized()` faz clamp no load; `validate()` recusa fora da faixa no save.
- UI: checkbox **"Ativação por permanência (drill-hover)"** + campo de duração
  (exibido em segundos, `0,5 s`), desabilitado quando o recurso está desligado;
  hint curto (sem clique; aplica a Resumo/Sugestões; 0,5 s padrão).
- O `Save` existente persiste (sem IPC novo).

## Arquivos

- `src-tauri/src/domain/settings.rs` — campos, constantes, default, sanitize,
  validate e testes (default, JSON antigo, clamp, inválido).
- `src/core/domain/settings.ts` — tipo, `DEFAULT_DWELL_MS`, `MIN_DWELL_MS`,
  `MAX_DWELL_MS`, `DWELL_STEP_MS`, `defaultSettings`.
- `src/features/settings/SettingsPage.tsx` — aba **Ajustes**.
- `src/features/settings/SettingsPage.test.tsx` (se aplicável).

## Critérios de aceite

- [ ] Config antiga (sem os campos) carrega com **ligado / 500 ms**.
- [ ] Valores fora da faixa são **clampados** no load e recusados no save.
- [ ] Alterar e salvar persiste; reiniciar mantém.
- [ ] Desligar não muda o clique (ainda sem hover).
- [ ] `cargo test`, `pnpm test`, `tsc --noEmit` verdes.

## Como testar

1. Abrir Configurações → Ajustes; conferir ligado e `0,5 s`.
2. Mudar para `1,2 s`, salvar, fechar e reabrir; conferir persistência.
3. Desligar, salvar; a duração fica desabilitada e o valor é preservado.
