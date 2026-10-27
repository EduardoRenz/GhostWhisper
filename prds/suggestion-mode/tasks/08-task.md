# 08 — Faixa “Última classificação”

- **Status:** CONCLUDED (2026-10-26)
- **Depende de:** 03, 04, 07
- **Habilita:** 09, 10
- **Issues:** — · **BR:** BR-095

## Objetivo

Colocar na janela de Sugestões uma faixa persistente que sempre mostra a última
classificação do classificador — inclusive quando a decisão foi **não sugerir** —
com cue, confiança, evidência, bloco, backend e latência.

## Entregáveis

- `src/features/suggestions/LastClassificationStrip.tsx` (novo):
  - uma linha compacta + expansão;
  - cue (badge colorido + label traduzido);
  - decisão (`vai sugerir` / `não sugerir`) e motivo;
  - confiança (percentual ou 0,00–1,00);
  - evidência (quote curta, truncada);
  - bloco (`mm:ss–mm:ss · N turnos`);
  - backend/modelo/latência;
  - tempo relativo (“há 8s”);
  - ações “Classificar agora” e “Tentar de novo”;
  - link “Ajustar nas Configurações”.
- `src/features/suggestions/ClassificationDetails.tsx` (opcional; pode ser
  interno ao strip) para a parte expandida.
- `SuggestionsWindow.tsx`: renderizar a faixa abaixo do header, antes do
  cartão/estado vazio.
- `useSuggestionClassification`: garantir `classifyNow` e `retry`.
- l10n: chaves da §8.5 do PRD.

## Critérios de aceite

- [x] Com `suggestion_show_classification=true`, a faixa é sempre renderizada,
      mesmo sem sugestão atual e mesmo em modo automático.
- [x] Estado vazio (classificador nunca rodou): texto explicativo + hint do
      bloco.
- [x] Modo automático: “Modo automático: o classificador está desligado.” +
      link para Configurações.
- [x] Sucesso: cue, confiança, evidência, bloco, backend/modelo/latência e
      tempo relativo.
- [x] Decisão negativa: mostra cue/decisão sem abrir sugestão. O motivo
      detalhado (limiar/cooldown/dedupe) entra no diagnóstico da task 10.
- [x] Erro/timeout: mostra o erro + “Tentar de novo”; não apaga a última válida.
- [x] “Classificar agora” chama o IPC, mostra loading e atualiza a faixa sem
      gerar sugestão.
- [x] Trocar de sessão limpa a faixa.
- [x] Expandir/colapsar não desloca o cartão de forma brusca (scroll do corpo).
- [x] Nenhum texto de transcrição no log/estado global além da evidência curta.

## Testes

- `mostra o estado vazio e permite classificar`
- `no modo automático aponta para as configurações`
- `mostra erro e tenta de novo`
- `mostra cue, decisão, confiança e tempo`
- `expande os detalhes com evidência, bloco e backend`
- `mostra decisão negativa sem abrir sugestão`

## Como testar

```bash
pnpm test -- src/features/suggestions/LastClassificationStrip
pnpm test -- src/core/application/useSuggestionClassification
pnpm tsc --noEmit
```

## Verificação (2026-10-26)

- `LastClassificationStrip` 6 testes; `SuggestionsWindow` cobre o strip no
  header; `useSuggestionClassification` hidrata/atualiza por evento.
- `cargo test --lib` 755 passed; `pnpm test` 504 passed; `tsc` limpo.
- O motivo detalhado da decisão (cooldown/limiar) é publicado pelo gate na
  task 10; a faixa já mostra cue/decisão/confiança.

## Notas

- A faixa é **diagnóstico**, não um segundo cartão de sugestão: mantenha-a
  compacta.
- Evidência é dado (não traduzir); cue é traduzida por chave.
- Não usar `dangerouslySetInnerHTML`; a evidência é texto puro.
- O botão “Classificar agora” é útil também no modo automático para testar o
  backend; se o modo automático estiver ativo, ele classifica sem mudar o modo.
