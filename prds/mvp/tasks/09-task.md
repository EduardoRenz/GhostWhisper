# 09 — Atalhos globais + stealth básico

- **Status:** CONCLUDED
- **Depende de:** 01, 07
- **Habilita:** —

## Objetivo
Registrar atalhos globais para operar o overlay sem sair da reunião e implementar
o "stealth" básico (sem barra de tarefas / sem título).

## Entregáveis (implementados)
- Domínio: `domain/shortcuts.rs` (`ShortcutAction`, `ShortcutBinding`,
  `ShortcutConflict`, `default_bindings`).
- Porta: `ports/shortcut_registry.rs` (`GlobalShortcutRegistry`).
- Adaptador: `adapters/shortcuts/tauri_global_shortcut_registry.rs`.
- IPC: `ipc/shortcuts_commands.rs` (`shortcuts_get`, `shortcuts_active`,
  `shortcuts_set`, `shortcuts_catalog`).
- Settings: campos `stealth` e `shortcuts` (persistidos); `settings_set` reaplica
  atalhos/stealth; `overlay_set_stealth` persiste.
- Composition root: registra atalhos e aplica stealth no boot; `dispatch_shortcut`.
- Frontend: seção de atalhos + checkbox de stealth em `SettingsPage.tsx`;
  `useInsightFeed` (evento `insight`) e painel expandido automático.

## Ações (padrões)
| Ação | Atalho padrão |
|------|---------------|
| Mostrar/ocultar overlay | `CommandOrControl+Shift+O` |
| Iniciar/encerrar sessão | `CommandOrControl+Shift+S` |
| Alternar stealth | `CommandOrControl+Shift+H` |
| Pedir sugestão | `CommandOrControl+Shift+I` |

## Critérios de aceite
- [x] Atalhos funcionam globalmente (X11) — registrados no boot sem conflitos.
- [x] Mostrar/ocultar não rouba foco (usa `overlay.toggle()`, janela `focus:false`).
- [x] Stealth remove da barra de tarefas (`set_skip_taskbar` + título vazio) e é
  persistido.
- [x] Conflito de atalho é reportado (`ShortcutConflict`) e não quebra o app.

## Evidências
- `cargo test` → **77 testes** (domínio de atalhos: defaults, ids/labels únicos,
  serialização; settings persistem atalhos/stealth).
- Smoke run (X11): os 4 atalhos registraram **sem** aviso de conflito.
- `pnpm test` → **10 testes**; `pnpm build` ok.
- `clippy`/`fmt` limpos.

## Notas / decisões
- Ações resolvem o `AppState` em tempo de execução (`app.state::<AppState>()`),
  evitando ciclo de construção entre o registro de atalhos e o estado.
- `Pedir sugestão` gera o insight no core e o publica pelo evento `insight`; a UI
  abre o painel expandido e o exibe.
- O adaptador Tauri não tem teste unitário (exige `AppHandle`); é verificado pelo
  smoke run. Validação manual em GNOME/KDE e do Wayland (portal) é recomendada.
- Atalhos são remapeáveis (persistidos em `settings.json`); acelerador vazio
  desregistra a ação.
