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
| 07 | Janela de Chat própria (movível e redimensionável) | CONCLUDED |
| 08 | Janela de Histórico (sessões anteriores) | CONCLUDED |
| 09 | Transporte: toggle único (start/pause) e stop único | CONCLUDED |
| 10 | Casca "Transcrição ao Vivo": cabeçalho, abas e rodapé | CONCLUDED |
| 11 | Bolhas em largura total: Microfone/Outros, hora e Streaming | CONCLUDED |
| 12 | Ação de IA por mensagem e handoff de contexto para Sugestões | CONCLUDED |
| 13 | Sugestão manual no Sugestões (contexto por parâmetro) | CONCLUDED |
| 14 | Documentação, verificação e pacotes (v0.2) | CONCLUDED |
| 15 | Concorrência e ciclo de vida das sugestões (single-flight e janela) | CONCLUDED |
| 16 | Intercalação por sobreposição e grip padrão | CONCLUDED |

> A **14** é a última na ordem de execução, embora apareça antes da **15** na
> tabela (mantém o fechamento junto das demais tasks). A ordem válida é a do
> grafo abaixo.

## Ordem e dependências

```
01 ─▶ 02
 └─▶ 03 ─▶ 04
        └─▶ 05
02 ─▶ 06
01 ─▶ 07 ─▶ 08
03 ─▶ 09 ─▶ 10 ─▶ 11 ─▶ 12 ─▶ 13 ─▶ 15 ─▶ 14
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
- A **10** e a **11** são a v0.2 visual (casca e cartões, BR-040); a 10 vem antes
  para fixar a geometria da janela.
- A **12** liga o botão de IA ao handoff de contexto (BR-041); a **13** consome o
  contexto no Sugestões e gera a sugestão manual. A 13 depende da 12.
- A **15** entra depois da 13 para fechar a concorrência: uma geração por vez no
  serviço, sugestão em andamento, janela aberta/fechada e prioridade do manual
  (BR-042).
- A **14** fecha: documentação, regras, verificação e pacotes. Só começa com
  10–13 e 15 concluídas.
