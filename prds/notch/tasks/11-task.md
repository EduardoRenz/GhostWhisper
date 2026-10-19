# 11 — Notch: Resumo e Sugestões com drill-hover

- **Status:** CONCLUDED
- **Depende de:** 09, 10
- **Habilita:** 12
- **Issues:** ISS-035 · **BR:** BR-036

## Objetivo

Aplicar o drill-hover **somente** em **Resumo** e **Sugestões** no notch ativo,
mantendo o clique imediato. O restante do switcher/ações não muda.

## Escopo

- `features/overlay/NotchTabs.tsx`: Resumo/Sugestões passam a usar `DwellButton`
  com o mesmo visual do switcher; os demais continuam botões normais.
- `features/overlay/ActionBar.tsx`/`Overlay.tsx`: `DwellProvider` envolvendo a barra;
  `pendingAction` para `onOpenSummary` e `onToggleSuggestions`.
- Comportamento:
  - clique no item → ação imediata;
  - hover → fill → confirmação `dwell` → "Sim" (dwell/clique) → ação;
  - `dwell_enabled=false` → hover não arma;
  - ao encerrar a sessão/ocultar overlay → fecha a confirmação e limpa a ação.
- `DwellProvider` usa `settings.dwell_ms` e `settings.dwell_enabled` (`useSettings`).
- O fill/estado aberto do item não pode re-renderizar o `ActionBar` por quadro.
- Testes: `ActionBar`/`NotchTabs` com fake timers + provider mockado; verificar
  opt-in, clique imediato, confirmação e reset.
- Atualizar `notch-ui.md`/`prd.md` se algum detalhe mudar na prática.

## Critérios de aceite

- [ ] Só **Resumo** e **Sugestões** têm fill/dwell (CA-11.7).
- [ ] Clique executa na hora (CA-11.3).
- [ ] Hover de `dwell_ms` abre a confirmação; "Sim" executa **uma vez** (CA-11.1/2).
- [ ] Grace de fechamento e `Escape` (CA-11.5).
- [ ] Desligado nos Ajustes, o hover não faz nada (CA-11.6).
- [ ] Confirmação acima/abaixo sem redimensionar/mover o notch (CA-11.4).
- [ ] Sem som/toast (CA-11.8).
- [ ] Testes verdes.

## Como testar

1. Iniciar sessão; pairar em Resumo → fill 0,5 s → confirmação → "Sim" → aba abre.
2. Clicar em Resumo → aba abre na hora, sem confirmação.
3. Repetir em Sugestões (abrir e fechar a janela).
4. Sair do gatilho/confirmação e voltar dentro de 0,5 s; depois deixar expirar.
5. Desligar o drill-hover nos Ajustes; o hover não deve abrir nada.
6. Mover o notch para o topo/rodapé da tela; conferir acima/abaixo.
