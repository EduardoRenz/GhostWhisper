# 20 — Trocar os canais ouvidos a qualquer momento

- **Status:** CONCLUDED
- **Depende de:** 16
- **Habilita:** —

## Objetivo
O usuário reportou: *"por que não é permitido mais trocar os canais a ouvir? isso
deveria ser sempre possível, inclusive sem precisar pausar ou parar"*.

O seletor (Ambos / Microfone / Sistema) ficava **desabilitado** durante a captura —
comportamento herdado de antes das sessões, quando trocar de canal só era possível
encerrando e começando outra sessão. Era contrassenso: se a sessão é a unidade de
continuidade (BR-019), mudar **o que** se capta não deveria interrompê-la.

## Diferença entre pausar e parar (esclarecida na UI)
- **Pausar** = suspende a captura, **mantém a sessão aberta** (mesmo id, sem
  `ended_at`); "Retomar" volta em um clique, sem passar pelo histórico.
- **Parar** = **encerra** a sessão (`ended`, com `ended_at`); para voltar nela, só
  pelo **Histórico → Continuar sessão** (que reabre o mesmo id).

Nos dois a captura para e o turno em aberto é fechado (flush). A diferença é o
**estado final da sessão**.

## Entregáveis
- `ports/session_controller.rs` + `SessionService::set_channels(&[AudioChannel])`:
  - **gravando**: para os canais que saíram, liga os que entraram (mesma sessão,
    **mesma origem de tempo**) e publica o novo estado;
  - **pausada**: só atualiza a escolha; o retomar captura o novo conjunto;
  - **sem sessão**: no-op (a escolha é da UI, usada no próximo "Iniciar");
  - canal que falha não derruba os outros (mantém os que funcionam, reporta o
    estado real) — e se **nenhum** sobreviver, a sessão é encerrada em vez de
    ficar "gravando" sem áudio;
  - lista vazia é recusada (sem afetar a captura atual).
- IPC `session_set_channels`; `useSessionState.setChannels`; adapter `sessionSetChannels`.
- `SessionControls`: seletor **sempre habilitado**; com sessão aberta ele reflete
  os canais realmente captados (ou os escolhidos, na pausa) e trocar ali aplica na
  hora. Sem sessão, escolher só define o próximo "Iniciar". "Nova sessão"
  reaproveita os canais em uso.
- `ActiveSession` passou a guardar `channels` (desejados) + `live`
  (`ActiveChannel { channel, pipeline }`), com ordem preservada na exibição.

## Critérios de aceite
- [x] Trocar de canal com a sessão gravando **não** encerra nem reinicia a sessão.
- [x] Trocar de canal com a sessão **pausada** atualiza a escolha e o retomar usa
      a nova.
- [x] Canal que sai para de captar; o que entra começa a captar; os mantidos
      seguem sem interrupção.
- [x] Falha de um canal não derruba os demais; sem nenhum canal vivo, a sessão é
      encerrada (nunca "gravando" sem áudio).
- [x] A troca aparece na UI (medidores/rótulos) pelo evento de estado.
- [x] `cargo test`/`clippy`/`fmt` e `pnpm test`/`tsc` verdes.

## Verificação (2026-10-18)
- Rust: adiciona canal sem encerrar (mesma sessão, `ended_at` nulo), remove canal
  e para **só** ele, troca um pelo outro, atualiza escolha na pausa e o retomar
  usa a nova, no-op sem sessão, lista vazia recusada (captura intacta), falha
  parcial preserva o canal que funciona, e nenhum canal sobrevivente encerra a
  sessão com estado "sem sessão".
- Frontend: seletor habilitado gravando **e** pausado, chama `onChannelsChange`
  com o conjunto certo, reflete os canais da sessão, e "Nova sessão" reaproveita
  os canais em uso.

## Pendências
- Escolha de **dispositivo** por canal dentro da sessão (hoje a sessão usa o
  padrão; a escolha de dispositivo existe só no teste de áudio).
