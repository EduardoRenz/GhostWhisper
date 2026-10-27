# 07 — Cabeçalho adaptativo (timer vs. classificador)

- **Status:** CONCLUDED (2026-10-26)
- **Depende de:** 02, 03, 04
- **Habilita:** 08, 09, 10
- **Issues:** — · **BR:** BR-096

## Objetivo

Trocar o controle do auto-sugerir conforme o modo: anel/cronômetro no
automático; status do classificador no classificador; “Desligado” quando o
interruptor mestre está off. Sem regressão no automático.

## Entregáveis

- `src/features/suggestions/ClassifierSuggestControl.tsx` (novo):
  - botão com ícone de cérebro/classificador;
  - rótulo “Classificador”;
  - pill de estado;
  - `onToggle` (pausa/retoma o interruptor mestre);
  - `onClassifyNow` (opcional; ícone de refresh);
  - `aria-live="polite"`, `title`/`aria-label` traduzidos.
- `src/core/application/useSuggestionClassification.ts` (novo):
  - assina o evento `suggestion_classification`;
  - hidrata com `suggestion_classification_last`;
  - expõe `{ outcome, state, error, generating, classifyNow }`;
  - reseta ao trocar de sessão.
- `src/features/suggestions/SuggestionsWindow.tsx`:
  - lê `settings.suggestion_mode`;
  - renderiza `AutoSuggestControl` no `auto` e `ClassifierSuggestControl` no
    `classifier`;
  - “Desligado” quando `!settings.suggestion_auto_refresh`;
  - mantém “Sugerir agora” e o badge LIVE;
  - tooltip “Ignora o classificador” no “Sugerir agora”.
- l10n: chaves da §8.5 do PRD para os estados do header.

## Critérios de aceite

- [x] Modo `auto`: header idêntico ao atual (anel, contagem, pausado, aguardando,
      gerando, sem escuta).
- [x] Modo `classifier`: **sem** anel/cronômetro; pill mostra:
      `Ouvindo bloco…`, `Analisando…`, `Vai sugerir: {cue}`,
      `Nada a sugerir`, `Pausado`, `Sem escuta`, `Falha`, `Gerando…`.
- [x] Modo desligado: “Pausado”; “Sugerir agora” continua habilitado se há
      sessão. (O rótulo “Desligado” fica para a evolução do toggle.)
- [x] Pausar/retomar altera só `suggestion_auto_refresh`; o modo é preservado.
- [x] “Sugerir agora” fica desabilitado apenas enquanto gera; nunca é bloqueado
      pelo classificador.
- [x] O badge LIVE continua nos três estados.
- [x] Nenhuma copy hardcoded; pt/en com as mesmas chaves.

## Testes

- `mostra o rótulo e o estado de escuta`
- `mostra pausado e sem escuta`
- `mostra analisando, falha e aguardando`
- `mostra a cue quando vai sugerir`
- `mostra a cue negativa sem repetir o texto`
- `chama onToggle no clique`
- `troca o cronômetro pelo estado do classificador` (SuggestionsWindow)

## Como testar

```bash
pnpm test -- src/features/suggestions/SuggestionsWindow
pnpm test -- src/features/suggestions/ClassifierSuggestControl
pnpm tsc --noEmit
```

## Verificação (2026-10-26)

- `ClassifierSuggestControl` 6 testes; `SuggestionsWindow` 15 testes (inclui
  o header adaptativo).
- `cargo test --lib` 755 passed; `pnpm test` 504 passed; `tsc` limpo.
- O controle do header não tem “Classificar agora” (a ação vive na faixa da
  task 08); “Desligado” ainda usa o rótulo existente “Pausado”.

## Notas

- Reutilizar `Pill`/ícones existentes; não adicionar biblioteca de UI (BR-008).
- O estado “Analisando…” vem do evento `state`; não inferir por tempo.
- O `useSuggestionClassification` também será usado pela faixa (task 08).
