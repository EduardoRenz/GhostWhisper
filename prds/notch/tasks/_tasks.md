# Tasks — PRD Notch

Índice das tarefas deste PRD. Cada tarefa tem um arquivo `NN-task.md`. A execução
só começa após a revisão da PRD pelo usuário.

**Legenda de status:** `PENDING` · `IN PROGRESS` · `CONCLUDED` · `CANCELED`

| # | Resumo | Status |
|---|--------|--------|
| 01 | Pacote de ícones SVG + `IconButton` + badges de atalho | CONCLUDED |
| 02 | Notch em dois estados (repouso/ativo) por ícones, ação verde + atalho | CONCLUDED |
| 03 | Painel da sessão em abas (Transcrição \| Resumo) e aposentadoria da janela `summary` | CONCLUDED |
| 04 | Seletor de idioma na barra inferior + troca em tempo real no ASR | CONCLUDED |
| 05 | Engrenagem/Ajustes, renomear Chat→Transcrição e acabamento (responsivo/acessibilidade) | CONCLUDED |
| 06 | Documentação (BR/memory), verificação e pacotes | CONCLUDED |
| 07 | Padronizar a UI com o design system "Obsidian HUD" | CONCLUDED |
| 08 | Notch ativo: switcher, waveform e play/pause contextual | CONCLUDED |
| 09 | Settings do drill-hover (ligado/duração) | CONCLUDED |
| 10 | Fundação reutilizável do drill-hover + janela de confirmação | CONCLUDED |
| 11 | Notch: Resumo e Sugestões com drill-hover | CONCLUDED |
| 12 | Validação visual, documentação e pacote (drill-hover) | CONCLUDED |
| 13 | Histórico no trailing e ícone de recolher | CONCLUDED |
| 14 | Notch sempre à frente (reafirmar always-on-top — ISS-043) | CONCLUDED |
| 15 | Notch encosta no topo (tamanho real da janela — ISS-044) | CONCLUDED |
| 16 | Notch não se move ao salvar settings de outras janelas (ISS-045) | CONCLUDED |
| 17 | Notch com largura dinâmica + menu de canais visível (ISS-047) | CONCLUDED |
| 18 | Dwell do "Sim" contínuo ao mover dentro do botão (ISS-048) | CONCLUDED |
| 19 | Drill-hover em um estágio (sem confirmação "Sim") | CONCLUDED |

## Dependências

```
01 ─┬─▶ 02 ─┐
    └─▶ 03 ─┼─▶ 05 ─▶ 06
04 ─────────┘
02,03,05 ─▶ 07 ─▶ 08
09 ─▶ 10 ─▶ 11 ─▶ 12 ─▶ 13
10,11 ─▶ 18
```

- 01 é a base (ícones + `IconButton` + badges) e não muda comportamento.
- 02 (notch) e 03 (abas) dependem de 01; 02 e 03 são independentes entre si.
- 04 (idioma em tempo real) é independente de 01–03 (núcleo Rust + barra inferior);
  para a barra, depende de 03 existir (é a barra inferior da aba Transcrição).
- 05 depende de 02/03/04 (é o acabamento de nomes e acessibilidade sobre eles).
- 06 fecha a PRD (docs, verificação e pacotes).
- 07 (design system) depende de 02/03/05: re-skina as superfícies já entregues.
  Tokens em [`../architecture/design-tokens.md`](../architecture/design-tokens.md).
- 14 (ISS-043) é independente: reafirma o empilhamento da janela.
- 15 (ISS-044) depende de 14: corrige o tamanho/posição da barra dentro da janela.
- 16 (ISS-045) depende de 11/14/15: estabiliza a geometria do notch e dos
  painéis contra saves de outras janelas.
- 17 (ISS-047) depende de 11/15: menu de canais abaixo da barra e largura pelo
  conteúdo.
- 18 (ISS-048) depende de 10/11: robustez do segundo estágio do drill-hover
  (histórico; substituído pela 19).
- 19 substitui o fluxo de dois estágios: o botão principal executa a ação ao
  completar o dwell, sem janela/popup "Sim".
- 08 (notch ativo) depende de 07: aplica a **estrutura** do protótipo (switcher,
  waveform, play/pause contextual) sobre os tokens.

## Definição de pronto (geral)

- Critérios de aceite da tarefa atendidos e roteiro "Como testar" executado.
- `cargo test`, `cargo clippy`, `cargo fmt --check` limpos.
- `pnpm test`, `tsc --noEmit` e `pnpm build` limpos (quando houver UI).
- Arquivo da tarefa atualizado para `CONCLUDED` e `_tasks.md` refletindo o status.
- `memory.md` deste PRD atualizado com `IDs:`/`Refs:` (BR-017).
- `business-rules.md` atualizado quando a tarefa criar/alterar regra
  (`BR-032`…`BR-034`, escrita única — R14).
