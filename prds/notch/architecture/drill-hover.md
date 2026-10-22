# Arquitetura — Drill-hover (ativação por permanência)

Especificação do **drill-hover** (dwell) do notch: acionar **Resumo** e
**Sugestões** sem clique, com preenchimento progressivo **e um único estágio**.
Contrato em **BR-036**, critérios em **US-11** (`../prd.md`).

---

## 1. Decisão: um estágio, sem confirmação

O botão opt-in se preenche durante `dwell_ms`; ao completar, **executa a ação
diretamente**. Não existe popup "Sem clique", botão "Sim", segunda janela nem
segundo timer.

Motivos:

- o fluxo de dois estágios com uma janela Tauri separada se mostrou instável
  (hover atravessando janelas, cursor obsoleto, reset ao mover dentro do "Sim");
- o preenchimento já é o feedback e sair antes do fim evita acionamento
  acidental;
- menos superfícies nativas, menos permissões e menos estado compartilhado.

O clique continua sendo o fallback imediato e acessível (Enter/Espaço).

## 2. Máquina de estados

```
idle
  │ pointerenter no botão (dwell_enabled && sessão ativa)
  ▼
arming  ── fill 0→100% em dwell_ms ──▶ executa a ação (uma vez)
  │ pointerleave/pointercancel/cancelamento
  └──────────────▶ idle (zera o fill)
```

- **Estágio único:** o fill é uma animação CSS (`data-arming` +
  `--dwell-ms`), sem loop de `requestAnimationFrame`.
- **Saída antes do fim:** `stop()` limpa o timeout e remove o fill; nada executa.
- **Ao completar:** a ação roda uma vez; permanecer no botão depois não repete.
  Só um novo `pointerenter`/movimento após um `stop` rearma.
- **Clique:** executa na hora e cancela o timer.
- **Desligado** (`dwell_enabled=false`) ou sem sessão ativa: o hover não arma;
  clique continua.
- **`start()` idempotente:** movimento dentro do botão não reinicia a contagem.

## 3. Componentes e camadas

```
src/core/application/useDwellTimer.ts
  timer puro de permanência (start/stop, cleanup, disabled cancela)

src/components/dwell/DwellProvider.tsx
  contexto enxuto: enabled (settings + sessão ativa) e durationMs

src/components/dwell/DwellButton.tsx
  botão opt-in: fill/linha + clique imediato + execução ao completar o dwell
  data-arming (CSS) · aria-pressed opcional · disabled

src/components/dwell/DwellButton.test.tsx
src/features/overlay/ActionBarDwell.test.tsx
  regressões de timer, saída, clique e travessia interna
```

- `DwellButton` é **genérico**: recebe `onActivate`; não conhece
  "Resumo"/"Sugestões".
- `DwellProvider` não guarda ação pendente nem estado de hover entre janelas.
- Não há porta/adaptador/IPC de dwell, janela Tauri `dwell` nem permissão
  `core:window:allow-cursor-position`.

## 4. Integração no notch

- `NotchTabs.tsx`: **Resumo** e **Sugestões** usam `DwellButton`; os demais
  botões continuam controles normais.
- `Overlay.tsx`: envolve a `ActionBar` com `DwellProvider`
  (`settings.dwell_enabled`, `settings.dwell_ms`, `active={state.listening}`).
- `DwellButton` usa `useDwellTimer(durationMs, onActivate, enabled && !disabled)`.
- Ao completar, a ação é a mesma do clique (`onOpenSummary`/
  `onToggleSuggestions`).

## 5. Configuração

| Campo | Padrão | Faixa | Onde |
|---|---|---|---|
| `dwell_enabled` | `true` | bool | Ajustes → "Expandir Notch por Dwell (Hover)" |
| `dwell_ms` | `500` | 300–2000 (passo 100) | Ajustes → duração (exibida em segundos) |

- Persistidos no `Settings` (JSON), **sem segredos** e **sem mudança de schema**.
- `sanitized()` faz clamp no load; `validate()` recusa valor fora da faixa no save.
- A duração é lida no início do timer; mudanças valem no próximo hover.

## 6. Visual e acessibilidade

- Fill do botão: véu cyan suave (`--oh-dwell-fill`) + linha inferior de 2 px
  (`--oh-dwell-line`), `border-radius: inherit`, `pointer-events: none`.
- Filhos de `.dwell-target` têm `pointer-events: none`; mover sobre ícone/texto
  mantém o botão como alvo do ponteiro.
- **Clique sempre funciona**; teclado usa o clique.
- Sem som/toast; `prefers-reduced-motion` mantém o tempo e remove animações.

## 7. Testes

- **Hook:** `useDwellTimer` (fake timers, `start` idempotente, stop, disabled).
- **Botão:** completa → ação uma vez; sai antes → não executa; clique imediato;
  movimento interno não reinicia; desligado/sem sessão não arma.
- **ActionBar:** só Resumo/Sugestões têm `dwell-target`; permanecer executa a
  ação; clique cancela o timer.
- **Remoção:** sem testes de popup/janela/segundo timer — a infraestrutura foi
  removida do projeto.
- **Manual:** hover em Resumo/Sugestões, saída antes do fim, clique, settings
  desligado e reduced motion.
