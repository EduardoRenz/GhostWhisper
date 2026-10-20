# 10 — Domínio e settings do HUD (opacidade, ancoragem, densidade)

- **Status:** CONCLUDED
- **Depende de:** 09
- **Habilita:** 11, 12
- **Issues:** — · **BR:** BR-062

## Objetivo

Modelar as três configurações novas do HUD no domínio Rust e no espelho do
frontend, com validação no save e sanitização no load, e alargar a faixa do
drill-hover para a do mockup (100 ms–1,5 s).

## Entregáveis

- `src-tauri/src/domain/hud_layout.rs` (novo):
  - `NotchAnchor` (`TopCenter`/`TopRight`/`Free`) e `HudDensity`
    (`Compact`/`Standard`/`Expanded`) com `parse_*`, `as_str` e `all()`;
  - `HudDensity::window_height()` → 62 / 72 / 84 px lógicos;
  - `anchor_position(anchor, work_area, window_size, margin) -> Option<(i32, i32)>`
    (`None` em `Free`);
  - testes puros de geometria e de altura.
- `src-tauri/src/domain/settings.rs`:
  - campos `overlay_opacity: f32`, `notch_anchor: String`, `hud_density: String`
    (`#[serde(default = …)]`);
  - constantes `DEFAULT_/MIN_/MAX_OVERLAY_OPACITY` (0,75 / 0,2 / 1,0),
    `NOTCH_ANCHORS`, `HUD_DENSITIES`;
  - `MIN_DWELL_MS = 100`, `MAX_DWELL_MS = 1_500` (padrão continua 500);
  - `validate()`: opacidade finita e na faixa; âncora/densidade no catálogo;
  - `sanitized()`: clamp da opacidade e fallback das strings inválidas.
- `src/core/domain/settings.ts`:
  - tipos `NotchAnchor`/`HudDensity`, listas e rótulos (`NOTCH_ANCHOR_LABELS`,
    `HUD_DENSITY_LABELS`), presets de opacidade;
  - campos no `Settings` e no `defaultSettings()`;
  - `MIN_DWELL_MS = 100`, `MAX_DWELL_MS = 1500`.
- `src/core/domain/settings.test.ts`: defaults e clamp.

## Critérios de aceite

- [x] Settings antigas (sem os três campos) carregam com os padrões.
- [x] `overlay_opacity` fora da faixa é **recusada** no save e **clampada** no load.
- [x] `notch_anchor`/`hud_density` desconhecidos caem no padrão.
- [x] `anchor_position` centraliza, encosta à direita e devolve `None` em `Free`.
- [x] `HudDensity::window_height()` devolve 62/72/84.
- [x] Dwell aceita 100–1500 ms (100 e 1500 válidos; 99 e 1501 recusados).
- [x] `cargo test` (519 ✓), `cargo clippy -D warnings` ✓, arquivos tocados
      formatados, `pnpm test` (339 ✓) e `tsc` ✓.

## Como testar

1. Abrir um `settings.json` antigo (sem os campos) → app abre com 75 % /
   Topo Central / Padrão.
2. Salvar `overlay_opacity = 0.1` direto no IPC → erro de validação.
3. Rodar os testes de `hud_layout` e de `domain::settings`.

## Verificação (2026-10-20)

- `domain/hud_layout.rs` com testes de geometria (centro, direita, origem/área
  útil de monitor secundário, `Free`, alturas 62/72/84 e parse).
- `domain/settings.rs`: defaults, settings legadas, clamp de opacidade, reset de
  âncora/densidade desconhecidas e validação recusando fora do contrato.
- Frontend: tipos, listas, rótulos e `clampOverlayOpacity` cobertos em
  `settings.test.ts`; `dwell.test.ts` atualizado para o teto de 1,5 s.
