# 08 — Commit em blocos por prefixo autoritativo (experimento)

- **Status:** PENDING (não implementado)
- **Depende de:** 03 (reuso do final/VAD), 04 (fila e métrica de commit)
- **Habilita:** bolhas menores que já nascem **concluídas** (sem split de exibição)
- **Issues:** ISS-074 · **BR:** BR-102 (contexto de produto no PRD `chat`)
- **Refs:** [`../issues.md`](../issues.md) ISS-074,
  [`../architecture/precision-and-performance.md`](../architecture/precision-and-performance.md)
  §3–§4, [`../../chat/prd.md`](../../chat/prd.md) BR-102

## Objetivo

Avaliar, por **A/B opt-in**, se é possível commitar um turno em **blocos
menores** durante fala contínua sem perder qualidade de transcrição — de modo
que cada bolha do chat já seja uma fala **finalizada**, não um corte de exibição.

## Contexto

O PRD `chat` removeu o split de 180 caracteres (ISS-073/BR-102): uma fala
finalizada é uma bolha. Bolhas menores só voltam se cada uma for um commit real.
Hoje as fronteiras seguras são pausas: 1 s de silêncio, o teto de 25 s (corte
forçado) ou o experimento de pontuação forte + 0,7 s, rejeitado na ISS-058.

## Desenho proposto

1. Decodificar o buffer do turno em **qualidade final** (beam) quando ele passar
   de um alvo (ex.: `chunk_target_ms`, 8–12 s) — em vez de esperar o silêncio.
2. Aproveitar os `start_timestamp`/`end_timestamp` dos segmentos retornados para
   escolher o **primeiro prefixo completo** (todos os segmentos, exceto o último,
   que pode estar cortado).
3. Commitar o prefixo como um turno final normal (mesma persistência, mesma
   linha temporal, mesmo gate de sugestões) e **manter** o áudio a partir do fim
   do último segmento commitado, com **sobreposição** de ~0,5–1 s, para o próximo
   bloco.
4. Deduplicar a emenda: o texto da sobreposição é re-decodificado e o prefixo já
   commitado não pode voltar no bloco seguinte.
5. Tudo atrás de env (`GHOSTWHISPER_CHUNK_COMMIT_MS=0` desligado por padrão);
   métrica de `commit_ms` e de palavras por bloco nos logs.

## Critérios de aceite (do experimento)

- [ ] A/B manual com o mesmo áudio contínuo: nenhuma palavra perdida ou repetida
      na emenda dos blocos.
- [ ] p95 do commit por bloco ≤ ~2,5 s (incluindo fila/ASR) no hardware alvo.
- [ ] Nenhum aumento perceptível de erros de fronteira em relação ao baseline
      de 1 s de silêncio.
- [ ] Default **off**; ligar só depois de A/B aprovado.
- [ ] Testes puros da decisão de corte (R17) antes de qualquer ligação.

## Não-objetivos

- Ligar o experimento por padrão sem A/B.
- Cortar áudio no meio de palavra ou por tempo fixo sem alinhamento a segmento.
- Mudar a UI do chat: ela já renderiza uma bolha por turno final (BR-102).
