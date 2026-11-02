# 03 — Settings, card em Geral & HUD e l10n

- **Status:** CONCLUDED (2026-11-01)
- **Depende de:** 02
- **Habilita:** 04, 05
- **Issues:** — · **BR:** BR-105

## Objetivo

Adicionar a **feature flag** `screen_context_enabled` (padrão desligado) ao
`Settings` (Rust + TS), o **card "Contexto de tela"** na aba Geral & HUD com
toggle, aviso de privacidade e linha de status, e toda a l10n.

## Entregáveis

- `src-tauri/src/domain/settings.rs`: `screen_context_enabled: bool` com
  `#[serde(default)]` (default `false`).
- `src/core/domain/settings.ts`: campo + default `false`.
- Command `screen_context_status` → `"disabled" | "available" | "unavailable"`
  (sem instanciar adaptador quando desligada).
- `src/features/settings/HudSettings.tsx`: card "Contexto de tela" após o de
  Privacidade (toggle + descrição com aviso + linha de status).
- l10n `src/core/l10n/locales/{pt,en}.ts`: chaves `snake_case`
  (`hud_screen_context`, `hud_screen_context_privacy_warning`,
  `hud_screen_context_status_*`, etc.).
- Testes: `HudSettings.test.tsx` (toggle default off, salva, status).

## Critérios de aceite

- [ ] Settings antigos (JSON sem o campo) carregam `false`.
- [ ] Card aparece em Geral & HUD; toggle reflete e grava a flag.
- [ ] Aviso de privacidade visível no card (BR-105).
- [ ] `screen_context_status` devolve `disabled` sem tocar na porta quando a
      flag está desligada.
- [ ] Todo texto novo em pt **e** en; zero hardcoded.

## Testes

- `old_settings_default_to_disabled`
- `toggle_persists_flag`
- `status_endpoint_never_touches_port_when_disabled`
