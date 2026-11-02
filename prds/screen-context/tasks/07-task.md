# 07 — Hardening, teste manual e docs

- **Status:** PENDING
- **Depende de:** 06
- **Habilita:** —
- **Issues:** — · **BR:** BR-105, BR-106

## Objetivo

Fechar o PRD: bateria de testes manuais nos cenários reais, revisão de
privacidade, docs e atualização de `memory.md`/`business-rules.md` se houver
desvio.

## Entregáveis

- **Teste manual** (registrar resultado em `memory.md`):
  1. Chrome com doc aberta → pill `🌐 chrome · título`, tooltip com texto da
     página; pergunta sobre o conteúdo responde certo (CU-01);
  2. VS Code → pill com arquivo (CU-02);
  3. terminal sem a11y → `Sem contexto de tela` (CU-04);
  4. flag desligada → sem pill, sem conexão D-Bus (CA-04.1);
  5. Wayland (Ubuntu 26.04/GNOME 50) → foco segue sem X11 (RNF-05);
  6. print + flag → os dois contextos na resposta;
  7. App que nega a11y → `DeniedAccess` vira snapshot vazio, sem erro;
  8. checkbox desmarcado (BR-106 v4/ISS-096) → **um** clique em qualquer
     parte da pill alterna (liga/desliga sempre, sem precisar mirar na
     caixa); desmarcado mostra `⏸ Contexto desativado` e **sem** consulta ao
     AT-SPI; remarcar relê e volta a `🌐`; preferência persiste após
     reiniciar.
  9. UI fluida (ISS-097) → abrir a barra com o contexto ativo não trava a
     digitação/cursor; `Enter` responde na hora mesmo se a leitura AT-SPI
     demorar (comando em thread pool).
  10. Alternância repetida (ISS-098) → ligar/desligar o contexto 5+ vezes e
      religar: o contexto volta em todas; sem acúmulo de threads/leituras
      (log do adaptador sem timeouts repetidos).
- Revisão de logs do adaptador (grep de conteúdo) e do ask.
- `cargo test`, `cargo clippy -D warnings`, `pnpm test`, `tsc --noEmit`
  verdes.
- Docs: referência no README (seção de contexto de tela) + entrada no
  `memory.md` do PRD com IDs.
- `tasks/_tasks.md` → `CONCLUDED`.

## Critérios de aceite

- [ ] Os 10 cenários manuais passam e estão registrados.
- [ ] Nenhum texto de contexto em log após a bateria.
- [ ] Pipeline completo verde.
- [ ] `memory.md` e `issues.md` atualizados (desvios → ISS novo).
