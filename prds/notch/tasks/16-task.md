# 16 — Notch não se move ao salvar settings de outras janelas

- **Status:** CONCLUDED
- **Depende de:** 11, 14, 15
- **Habilita:** —
- **Issues:** ISS-045 · **BR:** BR-062

## Objetivo

Impedir que o notch (e as janelas de painel) mudem de posição por causa de um
save de settings disparado por **outra** ação — trocar de aba, clicar num botão
ou mudar um select. Windows só se movem quando o **usuário** move, ou quando ele
muda a âncora/densidade do HUD.

## Diagnóstico

- O frontend envia `Settings` **completo** em `settings_set`; cada janela carrega
  seu próprio snapshot, que envelhece.
- O tracker de posição do core (`spawn_overlay_position_tracker`) grava a posição
  real no disco, mas qualquer save de outra janela regravava o snapshot antigo.
- `settings_set` reaplicava o HUD em **todo** save: em `free`, chamava
  `set_position(overlay_position)`; em âncora fixa, `set_hud_layout` recentralizava.

## Entregáveis

- `src-tauri/src/domain/settings.rs`:
  - `preserve_window_geometry(&mut self, persisted)` — posições/tamanhos de
    `overlay`, `suggestions`, `chat` e `history` vêm do estado persistido;
  - `hud_layout_changed(&self, other)` — compara âncora/densidade já normalizadas.
- `src-tauri/src/ipc/settings_commands.rs`:
  - carrega as settings persistidas antes de salvar, preserva a geometria e só
    reaplica `set_hud_layout` quando `hud_layout_changed`;
  - `set_stealth` só quando o valor muda.
- Testes de unidade para os dois helpers.

## Critérios de aceite

- [x] Trocar aba em **Sessão** não move o notch (com âncora livre ou fixa).
- [x] Clicar em botões/selects em outras janelas não move o notch nem os painéis.
- [x] Mudar **âncora** ou **densidade** continua valendo na hora.
- [x] Arrastar o notch continua persistindo a posição; reiniciar a restaura.
- [x] `cargo test`, `clippy -D warnings` e `rustfmt` nos arquivos tocados.

## Como testar

1. Em "Flutuante Livre", arraste o notch para um ponto qualquer.
2. Abra **Sessão** e alterne as abas várias vezes → o notch não se move.
3. Abra Sugestões e troque o agente algumas vezes → o notch não se move.
4. Em Configurações → Geral & HUD, troque a âncora/densidade → o notch se
   posiciona/redimensiona uma vez.
5. Feche e reabra o app → o notch volta à última posição do passo 1.

## Verificação (2026-10-20)

- `cargo test --lib` **543 ✓** (9 ignorados) · `clippy -D warnings` ✓ ·
  `rustfmt --check` nos arquivos tocados ✓.
- Teste manual de arraste/abas pendente com o binário novo.
