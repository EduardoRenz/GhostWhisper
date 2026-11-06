# 07 — Hardening, teste manual e docs

- **Status:** CONCLUDED (validado manualmente pelo usuário em 2026-11-05)
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
  11. Clique nas janelas do GhostWhisper (ISS-099) → com Chrome focado, clicar
      10× na AskBar/Sessão/Configurações: a pill mantém `🌐 chrome · ...`;
      repetir com terminal (mantém o terminal) e com a área de trabalho.
  12. Troca de programa/aba (ISS-100) → Chrome aba A → aba B / outro app:
      o contexto **muda** (nunca fica no anterior); trocar para um app sem
      árvore → `Sem contexto` (não o app antigo); perguntar imediatamente
      após a troca não envia o texto da janela anterior.
  13. App sem a11y (ISS-101) → terminal → Chrome **sem**
      `--force-renderer-accessibility`: após ~300 ms o terminal sai do
      contexto (pill "Sem contexto"/Chrome sem árvore); clicar direto na
      AskBar vindo do terminal **preserva** o contexto.
- Revisão de logs do adaptador (grep de conteúdo) e do ask.
- `cargo test`, `cargo clippy -D warnings`, `pnpm test`, `tsc --noEmit`
  verdes.
- Docs: referência no README (seção de contexto de tela) + entrada no
  `memory.md` do PRD com IDs.
- `tasks/_tasks.md` → `CONCLUDED`.

## Critérios de aceite

- [x] Os 13 cenários manuais passam e estão registrados (aprovado pelo usuário
      em 2026-11-05).
- [x] Nenhum texto de contexto em log após a bateria.
- [x] Pipeline completo verde.
- [x] `memory.md` e `issues.md` atualizados (desvios → ISS novo).
