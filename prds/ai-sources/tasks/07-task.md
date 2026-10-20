# 07 — Casca da tela de Configurações

- **Status:** CONCLUDED
- **Depende de:** —
- **Habilita:** 08, 09

## Objetivo

Repaginar a tela de Configurações: cabeçalho próprio, barra de abas com
ícones, conteúdo em *cards* e **rodapé fixo** com um único "Salvar Alterações"
(BR-047). O conjunto de abas **não** muda (as 5 atuais).

## Entregáveis

- `src/features/settings/SettingsPage.tsx`:
  - header (ícone, título, subtítulo, `Esc`, botão `✕`);
  - tablist com 5 abas (ícone + rótulo) — `role="tab"`/`aria-selected`;
  - corpo rolável com as seções em cards;
  - rodapé fixo: nota "Todas as alterações são sincronizadas localmente",
    "Restaurar Padrões" e "Salvar Alterações";
  - **rascunho único** (uma carga de `getSettings`), `dirty` e confirmação ao
    restaurar padrões.
- `src/styles/globals.css`: classes novas com os tokens `--oh-*`.
- `src-tauri/tauri.conf.json`: janela `settings` 1020×720 (mín. 760×560).
- Fechar a janela: `Esc` e `✕` (o `✕` esconde a janela, como já acontece ao
  fechar pelo gerenciador).

## Critérios de aceite

- [x] As 5 abas aparecem com ícone e rótulo.
- [x] O rodapé mostra "Salvar Alterações" e "Restaurar Padrões".
- [x] "Salvar Alterações" chama `saveSettings` **uma vez** com o rascunho.
- [x] "Restaurar Padrões" pede confirmação antes de zerar o rascunho.
- [x] Não há mais botão de salvar dentro das abas (exceto o de atalhos, que
      registra no SO).
- [x] `pnpm test` e `tsc --noEmit` limpos.
