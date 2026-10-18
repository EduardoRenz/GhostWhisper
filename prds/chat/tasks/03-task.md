# 03 — Transporte por ícones: play / pause / stop

- **Status:** CONCLUDED
- **Depende de:** 01
- **Habilita:** 04

## Objetivo

Substituir os botões de texto ("Pausar/Retomar/Parar") pela **pílula de ícones** do
Perssua — play, pause e stop juntos, com o inativo esmaecido — mantendo o seletor
de canais e "Nova sessão".

## Entregáveis

- `components/TransportIcons.tsx`: SVG próprios (`PlayIcon`, `PauseIcon`,
  `StopIcon`) — sem Material Icons (BR-008).
- `SessionControls`: pílula `[▶][⏸][⏹]` (`gap:2px; padding:2px; radius:999px`;
  botão 28×28; `:disabled{opacity:.4}`; hover vermelho no stop) com `aria-label` e
  tooltip em PT.
- Estados: parado → pílula **"Iniciar escuta"** (play + rótulo, cobre "Start
  Listening"/"Restart" do Perssua); gravando → ▶ esmaecido; pausado → ⏸ esmaecido.
- Seletor de canais sempre habilitado (BR-019) e "Nova sessão" preservados.

## Critérios de aceite

- [ ] Parado: "Iniciar escuta" inicia com os canais escolhidos.
- [ ] Gravando/pausado: cada ícone chama a ação certa e o inativo fica esmaecido.
- [ ] Stop só existe com sessão aberta (inclusive pausada).
- [ ] Nenhum rótulo de texto nos três controles de transporte.

## Verificação

- `SessionControls.test.tsx`: matriz de estados (parado/gravando/pausado) e a ação
  de cada botão; seletor habilitado nos três estados.

## Resultado (2026-10-18)

- `TransportIcons.tsx`: SVG próprios (`PlayIcon`, `PauseIcon`, `StopIcon`,
  `SparkIcon`).
- Matriz de estados coberta em `SessionControls.test.tsx` (8 casos): parado →
  "Iniciar escuta" com os canais escolhidos; gravando → play esmaecido, pause/stop
  ativos; pausado → play ativo, pause esmaecido; troca de canais nos dois estados;
  "Nova sessão" reaproveita os canais; tudo desabilitado com operação em curso; os
  botões da pílula não têm texto (só ícone).
