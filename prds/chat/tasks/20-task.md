# 20 — Corte de sobreposição conservador (fim das bolhas de 1–5 caracteres)

- **Status:** CONCLUDED
- **Depende de:** 16 (intercalação por sobreposição), 19 (uma fala = uma bolha)
- **Habilita:** —
- **Issues:** ISS-075 · **BR:** BR-043 (revisada)
- **Refs:** [`../../business-rules.md`](../../business-rules.md) BR-043,
  [`../issues.md`](../issues.md) ISS-075,
  [`../architecture/transcription-panel.md`](../architecture/transcription-panel.md) §3.4

## Objetivo

Impedir que o corte de sobreposição transforme uma fala em bolhas de 1–5
caracteres. O corte continua existindo para intercalar quem falou no meio, mas só
quando a fala é longa, a interrupção é real e todas as partes continuam legíveis.

## Causa (sessão real `911fcf10…`, 17:15)

24 falas finais viravam **30 bolhas** na tela. O `splitSpeechSpans` cortava em
**qualquer** início do outro canal:

- `"e 79."` → `"e"` (1c) + `"79."` (3c);
- `"o homem pisou na lua pela primeira vez."` → `"…primeira"` + `"vez."` (4c);
- `"Ei, ei, ei!"` → `"Ei,"` + `"ei, ei!"`;
- `"Beleza! Seguim!"` → `"Beleza!"` + `"Seguim!"`;
- `"vamos lá então vamos começar"` (28c, 4s) → 2 bolhas de 13–14c.

Nada disso era erro de ASR: o texto no banco estava inteiro.

## Regra entregue

- Só divide fala com **≥ 80 caracteres e ≥ 6 s**.
- Só considera interrupção do outro canal com **≥ 1,5 s** (interjeição curta não
  intercala).
- Depois de calcular as partes, exige **≥ 20 caracteres e ≥ 3 palavras** em
  **todas** elas; senão mantém a fala inteira.
- Vale no ao vivo (`buildChatTimeline`) e no histórico (`bubblesFromTimeline`),
  porque a regra vive em `splitSpeechSpans`.

## Critérios de aceite

- [x] Fala curta não é dividida só porque o outro canal começou.
- [x] Interjeição curta (< 1,5 s) não intercala.
- [x] Nenhuma parte resultante fica com < 20 caracteres ou < 3 palavras.
- [x] O caso positivo continua: fala longa + interrupção real intercala em 3
      bolhas com rótulo/hora.
- [x] Ao vivo e histórico usam a mesma regra.
- [x] `pnpm test`, `tsc --noEmit` e `pnpm build` verdes.

## Resultado (2026-10-27)

- `splitSpeechSpans` com os limiares `MIN_OVERLAP_*` no domínio.
- Simulação com os dados reais: **30 → 25** bolhas na sessão `911fcf10…` e
  **11 → 9** na `a60933ea…`; os fragmentos `"e"`, `"vez."`, `"79."`, `"Ei,"`,
  `"Beleza!"`/`"Seguim!"` deixam de existir.
- Testes anti-regressão que falhariam antes: fala curta + outro canal; interjeição
  curta; sobra sem contexto; caso positivo de intercalação.
- `pnpm test` **520 ✓** (82 arquivos) · `tsc --noEmit` ✓ · `pnpm build` ✓.
