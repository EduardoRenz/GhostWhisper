# 05 — Engrenagem/Ajustes, renomear Chat→Transcrição e acabamento

- **Status:** CONCLUDED
- **Depende de:** 02, 03, 04
- **Habilita:** 06
- **Issues:** — · **BR:** BR-032, BR-033

## Objetivo

Fechar a nomenclatura e o acabamento: **engrenagem** abrindo as Configurações,
**"Chat" → "Transcrição"** em toda a UI, aba ativa consistente com o tray e ajustes
de **responsividade/acessibilidade** das abas e da barra.

## Entregáveis

- Textos/rótulos: cabeçalho e título da janela → **Transcrição**; remover
  ocorrências de "Chat" voltadas ao usuário (`README.md` quando citar a janela,
  `ChatHeaderActions`, tray).
- `src/features/overlay/ActionBar.tsx` — engrenagem chama `openSettingsWindow`
  (mesmo handler de hoje).
- `src/features/panel/PanelTabs.tsx` / `globals.css` — responsivo (janela estreita:
  abas com ícone + rótulo curto) e foco/teclado; `prefers-reduced-motion`.
- Tray (`src-tauri/src/lib.rs`) — "Abrir resumo" abre a janela `chat` na aba Resumo;
  rótulos coerentes com o novo nome.
- Ajustes de `docs`: emitir mapa dos rótulos afetados (para o `memory.md` da 06).

## Critérios de aceite

- [ ] A UI não usa mais "Chat" para a superfície; o termo é **Transcrição**
      (mantendo o **label técnico** da janela `chat` — CA-04.2).
- [ ] A engrenagem abre as Configurações em qualquer estado (CA-02.2, RF-07).
- [ ] Em janela estreita, as abas continuam legíveis e clicáveis (ícone + rótulo).
- [ ] Navegação por teclado entre abas funciona; foco visível (RNF-04).
- [ ] `prefers-reduced-motion` desliga animações novas.
- [ ] `cargo test`, `clippy`, `fmt`, `pnpm test`, `tsc`, `pnpm build` verdes.

## Como testar

1. Percorra a UI (notch, janela, tray, Configurações) e aponte qualquer "Chat"
   restante voltado ao usuário — deve ser "Transcrição".
2. Clique na engrenagem em repouso e em sessão ativa: abre as Configurações.
3. Reduza a janela até o mínimo (360×320): abas legíveis, sem overflow.
4. Navegue pelas abas só com o teclado.

Ponto de parada: usuário aprova nomes e acabamento.

## Riscos / rollback

- Renomear pode afetar testes que buscavam "Chat" → atualizar seletores por
  `aria-label`/`role`, mais estáveis.
- Overflow em janela estreita → testar no mínimo antes de fechar.

## Verificação

- Automática: `cargo test` · `clippy` · `fmt` · `pnpm test` · `tsc` · `pnpm build`.
- Manual: os 4 passos acima.
- Observações do usuário: (aguardando)

## Resultado (2026-10-18)

- Textos: título da janela e aba = **Transcrição**; tray "Abrir transcrição";
  README reescrito (notch em dois estados, abas, idioma na hora, custo do resumo).
- Engrenagem abre as Configurações em qualquer estado (já no `ActionBar` da task 02).
- Acessibilidade: `aria-label` em todos os ícones, foco visível (`:focus-visible`),
  `role="tablist"`/`aria-selected`/navegação por teclado, `prefers-reduced-motion`.
- Verificação: `pnpm test` 155 ✓ · `tsc` ✓ · `pnpm build` ✓.
