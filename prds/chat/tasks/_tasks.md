# Tasks — PRD Chat

Índice de tarefas deste PRD. Cada tarefa tem um arquivo `NN-task.md`.

**Legenda de status:** `PENDING` · `IN PROGRESS` · `CONCLUDED` · `CANCELED`

| # | Resumo | Status |
|---|--------|--------|
| 01 | Conversa em bolhas na linha temporal (corrige ISS-018) | CONCLUDED |
| 02 | Sugestões da IA na mesma conversa + histórico em bolhas | CONCLUDED |
| 03 | Transporte por ícones: play / pause / stop e "Iniciar escuta" | CONCLUDED |
| 04 | Pílula de sessão (tempo + confirmação) e bolinha de status | CONCLUDED |
| 05 | Documentação (BR-021/022), verificação e pacotes | CONCLUDED |

## Ordem e dependências

```
01 ─▶ 02
 └─▶ 03 ─▶ 04
        └─▶ 05
```

- A **01** corrige o bug de linha temporal e é pré-requisito de todo o resto.
- A **02** depende da 01 (usa o mesmo `ChatView`).
- A **03** é independente da 02; a **04** estende a 03 (mesma pílula/estado).
- A **05** fecha: regras de negócio, docs, verificação e pacotes.
