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

## Dependências

```
01 ─┬─▶ 02 ─┐
    └─▶ 03 ─┼─▶ 05 ─▶ 06
04 ─────────┘
02,03,05 ─▶ 07 ─▶ 08
```

- 01 é a base (ícones + `IconButton` + badges) e não muda comportamento.
- 02 (notch) e 03 (abas) dependem de 01; 02 e 03 são independentes entre si.
- 04 (idioma em tempo real) é independente de 01–03 (núcleo Rust + barra inferior);
  para a barra, depende de 03 existir (é a barra inferior da aba Transcrição).
- 05 depende de 02/03/04 (é o acabamento de nomes e acessibilidade sobre eles).
- 06 fecha a PRD (docs, verificação e pacotes).
- 07 (design system) depende de 02/03/05: re-skina as superfícies já entregues.
  Tokens em [`../architecture/design-tokens.md`](../architecture/design-tokens.md).
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
