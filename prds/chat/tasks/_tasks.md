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
| 06 | Teto da bolha de fala (continuação em bolha seguinte) | CONCLUDED |
| 07 | Janela de Chat própria (movível e redimensionável) | PENDING |
| 08 | Janela de Histórico (sessões anteriores) | PENDING |
| 09 | Transporte: toggle único (start/pause) e stop único | PENDING |

## Ordem e dependências

```
01 ─▶ 02
 └─▶ 03 ─▶ 04
        └─▶ 05
02 ─▶ 06
01 ─▶ 07 ─▶ 08
03 ─▶ 09
```

- A **01** corrige o bug de linha temporal e é pré-requisito de todo o resto.
- A **02** depende da 01 (usa o mesmo `ChatView`).
- A **03** é independente da 02; a **04** estende a 03 (mesma pílula/estado).
- A **05** fecha: regras de negócio, docs, verificação e pacotes.
- A **06** é um ajuste posterior pedido pelo usuário no teste, em cima da 02
  (teto da bolha de fala, ver ISS-031).
- A **07** tira a conversa do overlay para uma janela própria (BR-026) e simplifica
  o overlay; a **08** depende dela (o painel que hospedava o histórico sai).
- A **09** revisa o transporte das 03/04 (toggle único + stop único, BR-022).
