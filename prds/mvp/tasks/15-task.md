# 15 — Monitor atrasado do microfone ("ouvir minha voz")

- **Status:** PENDING
- **Depende de:** 14
- **Habilita:** —

## Objetivo
Dar retorno **audível** ao calibrar o microfone: ouvir a própria voz com um
atraso fixo confirma na hora qual dispositivo está captando e se o nível está
bom — sem depender de números no painel.

## Motivo (UX)
O "teste de áudio" mostra barra e RMS, mas o usuário ainda precisa julgar se
aquilo é fala boa. Ouvir a própria voz é o retorno mais direto possível.

## Desenho proposto
- **Porta nova:** `ports/audio_player.rs` (`play(&self, samples: &[f32])` +
  `stop()`), com adaptador `adapters/audio/cpal_output.rs` (saída `cpal`) — o
  projeto hoje só tem caminho de **entrada**; não existe saída.
- **Wiring:** instanciado no composition root (`lib.rs`) e guardado em
  `AppState`; usado **somente** pelo painel de teste.
- **Atraso fixo (ring buffer):** ~300–600 ms. Sem atraso, alto-falante gera
  realimentação (microfonia). O ring buffer desacopla entrada e saída; o atraso
  ideal é o maior confortável (evita o loop acústico).
- **Ganho de monitor independente** e baixo (ex.: 1×–2×), separado do ganho do
  ASR — não precisa (nem deve) ser o mesmo valor.
- **UI:** toggle "Ouvir minha voz" em `AudioTestPanel`, habilitado apenas
  durante o teste e apenas no canal de microfone.

## Cuidados
- **Fone de ouvido** é obrigatório: com caixa ligada o loop é acústico e nenhum
  atraso digital resolve. Avisar na UI.
- **Nunca logar áudio** (regra do projeto) — o monitor é local, nada sai da
  máquina (BR-002).
- Encerrar a saída junto com o teste (nunca deixar um stream de saída pendurado
  ao sair das configurações).
- Se a porta/adapter `cpal` de saída se mostrar instável sob PipeWire, avaliar
  `libpulse-simple-binding` (já é dependência do projeto).

## Critérios de aceite (a definir na implementação)
- [ ] Ouvir a voz com atraso perceptível e estável, sem realimentação em fone.
- [ ] Toggle só durante o teste de microfone; saída encerrada ao parar.
- [ ] Ganho de monitor independente do ganho do ASR.
- [ ] Nenhum log de áudio; nada de rede.
- [ ] Testes: ring buffer (unidade), delegação do adaptador (fake de saída).

## Referência
`AudioTestPanel.tsx` (nível), BR-018, task 14.
