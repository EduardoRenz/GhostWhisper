# Memory — PRD Translation

Nota histórica **deste PRD**: decisões, medições e o que foi feito, em ordem
cronológica (R15, BR-017). Entradas `## YYYY-MM-DD — Título curto`, com
`IDs:`/`Refs:` logo abaixo; a mais recente no fim. Busca é via `rg` (R14).

| Objetivo | Comando |
|----------|---------|
| Palavra-chave | `rg -n -i 'bolha' prds/translation/memory.md` |
| Entrada do dia | `rg -n '^## 2026-10-18' prds/translation/memory.md` |
| Por ID | `rg -n 'ISS-029\|BR-030' prds/translation/memory.md` |

---

## 2026-10-18 — PRD aberta e T1: tradução sob demanda na bolha

- **IDs:** ISS-029, BR-030
- **Refs:** BR-005, BR-013, [`tasks/01-task.md`](tasks/01-task.md),
  [`../chat/prd.md`](../chat/prd.md) (não-objetivo revogado)

### Contexto
Feedback do usuário durante o teste do E2: o painel de tradução abaixo da conversa
repetia o original (parecia um "segundo chat") e, com origem **Auto** e destino
**português**, o app traduzia pt→pt.

### Decisões
- Tradução **sob demanda na bolha**: ícone no canto superior direito; clique
  traduz; novo clique volta ao original. Tradução **abaixo** do original, com
  **linha tracejada** e **itálico menor**.
- Sem botão de copiar por tradução; painel duplicado **removido de vez**.
- Tradução **automática em massa** sai de cena (pode voltar amadurecida depois).
- O gate por idioma detectado (ISS-030) fica na T2.

### Implementado (T1)
- `useTranslation` virou **sob demanda** (`translate({id, text})`), com fila serial
  e cache por id; a tradução automática de todas as falas foi removida.
- `ChatBubble` ganhou o botão (`TranslateIcon`, `data-interactive`), o estado de
  toggle e a linha `chat__translation`.
- `ChatView`/`Overlay` passam traduções/pendências/`onTranslate`; `TranslationView`
  e seus estilos foram removidos.
- Guarda de identidade: tradução igual ao original (normalizada) não é exibida.

### Verificação
- `pnpm test` 122 ✓ (novos: hook sob demanda + cache, botão/toggle na bolha,
  ausência do botão com a tradução desligada) · `tsc --noEmit` ✓ · `pnpm build` ✓.
- Pendente: teste manual (visual da bolha e do toggle). T2 não iniciado.
