# Arquitetura — Drill-hover (ativação por permanência)

Especificação do **drill-hover** (dwell) do notch: acionar **Resumo** e
**Sugestões** sem clique, com preenchimento progressivo e confirmação flutuante.
Contrato em **BR-036**, critérios em **US-11** (`../prd.md`).

---

## 1. Por que uma janela própria

O notch é uma janela Tauri de **900×72** com a barra de ações. Um elemento
`position: absolute` flutua sobre o layout, mas **não ultrapassa os limites da
janela nativa**: um popover acima/abaixo do botão seria clipado pelo SO.

Decisão: a confirmação vive em uma **janela Tauri `dwell`**, pequena e transparente,
pré-criada e **reutilizada**. A janela do notch **não** redimensiona nem se move.
É o mesmo padrão de superfícies separadas já usado por `suggestions`, `history` e
`chat`.

Alternativas descartadas:

- **Expansão temporária** da janela do overlay: funciona, mas no Linux pode
  piscar/remappear a cada hover e reafirmar stealth.
- **Canvas fixo maior** (overlay ~200 px transparente): evita a segunda janela, mas
  cria uma área transparente fixa maior — risco de aparecer em compartilhamento de
  tela, justamente no cenário stealth.

## 2. Máquina de estados

```
idle
  │ pointerenter no gatilho (dwell_enabled)
  ▼
arming  ── fill 0→100% em dwell_ms ──▶ abre a confirmação
  │ pointerleave antes de 100%                 │
  └──────────────▶ idle (reset imediato)       ▼
                                            confirming
                                              │ pointer no "Sim"
                                              ▼
                                      confirming (fill do Sim)
                                              │ dwell_ms ou clique
                                              ▼
                                         confirmado → ação
```

- **Estágio 1 (gatilho):** o fill é uma animação **CSS** (`animation` com
  `--dwell-ms`), não um loop de `requestAnimationFrame`. Sair remove a classe e o
  fill volta a zero na hora.
- **Estágio 2 (confirmação):** o botão **"Sim"** tem o mesmo fill/tempo; clique
  confirma imediatamente.
- **Clique no gatilho** executa a ação direto (sem confirmação) e cancela o fill.
- **Desligado** (`dwell_enabled = false`): o hover não arma; clique continua.

### 2.1 Cancelamento com grace (antiacidente)

O timer de dispensa vive na **janela de confirmação**, não no overlay:

- A confirmação fica aberta enquanto o cursor estiver **no gatilho** ou **na janela
  de confirmação**.
- Saiu dos dois (e **não** está sobre o "Sim"): inicia um timer de `dwell_ms`.
  - voltar ao gatilho ou entrar/mover na confirmação → **pausa/limpa** o timer;
  - expirar → fecha e reseta.
- Sobre o "Sim", o timer de dispensa **não** corre; o dwell de confirmação continua.
- `Escape` fecha imediatamente.

O overlay informa o estado do gatilho pelo comando `dwell_trigger_hover`
(relay no core) e a janela de confirmação pausa o timer ao receber
`pointerenter`. **Além dos eventos de ponteiro**, a janela amostra a **posição
global do cursor** a cada ~90 ms (`cursorPosition` + geometria da janela) e usa
essa amostra como fonte de verdade para pausar o grace e iniciar o "Sim". Isso
cobre o caso em que o webview mantém um estado de hover obsoleto depois de ser
escondido/mostrado de novo (só funcionava na primeira abertura).

## 3. Fluxo entre as janelas

```
overlay (React)                         core (Rust)                  dwell (React)
────────────────                        ───────────                  ─────────────
DwellButton arma (CSS)
  └─ após dwell_ms: DwellProvider
       request(action, rect)
       invoke dwell_show(rect, ms) ───▶ DwellPopoverController
                                         posiciona + mostra ───────▶ recebe contexto
overlay guarda pendingAction
       ◀── evento dwell_closed ───────  hide + emit
  confirmed? executa pendingAction
```

- **A ação pendente** fica no overlay (`DwellProvider`), não no Rust: qualquer botão
  pode passar o próprio `onActivate`; a janela só responde **"Sim"** ou cancela.
- **Posicionamento** no core: o overlay manda o retângulo do gatilho relativo à
  viewport; o adaptador soma a posição da janela overlay, converte para físico e
  posiciona a janela `dwell` acima/abaixo, usando a **área útil** do monitor.
- **Eventos:**
  - `dwell_show` (comando) — abre a confirmação;
  - `dwell_hide` (comando) — fecha sem resultado (reset);
  - `dwell_result { confirmed }` (comando, da janela `dwell`) — fecha e emite
    `dwell_closed { confirmed }` ao overlay;
  - `dwell_trigger_hover { entered }` (comando do overlay → core) — o core reemite
    `dwell_trigger` à janela `dwell`; o grace local pausa/retoma;
  - `dwell_context { dwell_ms, token, x, y, width, height, scale }` (evento core →
    `dwell`) — duração do estágio 2, reset local e **geometria real** da janela
    para o poll do cursor.

## 4. Componentes e camadas

```
src/core/domain/dwell.ts
  DEFAULT_DWELL_MS = 500 · MIN_DWELL_MS = 300 · MAX_DWELL_MS = 2000
  clampDwellMs(ms) · dwellLabel(ms)  // "0,5 s"

src/core/application/useDwellTimer.ts
  timer puro de permanência (start/stop/reset, cleanup)

src/components/dwell/DwellButton.tsx
  botão opt-in: fill + linha + clique imediato + armar no hover
  data-arming (CSS) · aria-haspopup="dialog" · disabled

src/components/dwell/DwellPopover.tsx
  conteúdo da janela `dwell`: "Sem clique" + "Sim" + badge da duração

src/components/dwell/DwellProvider.tsx
  contexto do overlay: enabled/ms, pendingAction, request/cancel/click e
  repasse do hover do gatilho (dwell_trigger_hover)

src/features/dwell/DwellPopoverWindow.tsx
  rota `dwell`; timer do "Sim"; **grace local** (pausa ao entrar na janela);
  **poll da posição global do cursor** (~90 ms); Escape; clique; `dwell_result`

src/adapters/ipc/cursor.ts
  `cursorPosition` + geometria do `dwell_context` → `inside`/`overConfirm`
  (matemática pura testável)

src-tauri/src/ports/dwell_popover.rs
  trait DwellPopoverController { show(anchor, dwell_ms); hide(); context() }

src-tauri/src/adapters/dwell/tauri_dwell_popover.rs
  janela `dwell`: posicionar pela work area, mostrar/esconder, guardar ms

src-tauri/src/ipc/dwell.rs
  dwell_show · dwell_hide · dwell_result · dwell_context
```

- `DwellButton` é **genérico**: recebe `onActivate` e usa o `DwellProvider` do
  overlay. Para aplicar a outro botão, basta envolvê-lo; nada de lógica duplicada.
- O `DwellProvider` não conhece "Resumo"/"Sugestões": só recebe callbacks.
- A janela `dwell` é **pré-criada** em `tauri.conf.json` (`visible: false`,
  transparente, sem foco/taskbar), então o popover abre sem criar webview.

## 5. Posicionamento

- Âncora = retângulo do gatilho em CSS px, relativo à janela do overlay.
- O adaptador soma `overlay.outer_position()` (físico), aplica o `scale_factor`,
  centraliza horizontalmente e escolhe:
  - **acima** se `espaço_acima ≥ altura_popover + gap` (preferência);
  - senão **abaixo**;
  - se nenhum lado couber, usa o lado com mais espaço e faz clamp na work area.
- Tamanho lógico fixo da janela: **260×60** (pill compacta + sombra), gap de **8 px**.
  A janela é transparente; o pill é desenhado dentro.
- A work area vem de `Monitor::work_area()` (Tauri 2.12+), respeitando painéis e
  taskbar. Função pura em Rust (`domain::geometry`) para os testes.

## 6. Configuração

| Campo | Padrão | Faixa | Onde |
|---|---|---|---|
| `dwell_enabled` | `true` | bool | Ajustes → "Ativação por permanência (drill-hover)" |
| `dwell_ms` | `500` | 300–2000 (passo 100) | Ajustes → duração (exibida em segundos) |

- Persistidos no `Settings` (JSON), **sem segredos** e **sem mudança de schema**.
- `sanitized()` faz clamp no load (settings antigas ficam no padrão/faixa);
  `validate()` recusa valor fora da faixa no save.
- A duração é lida no início de cada estágio; mudanças valem no próximo hover.

## 7. Visual e acessibilidade

- Fill do gatilho: véu cyan suave (`--oh-dwell-fill`) + linha inferior de 2 px
  (`--oh-dwell-line`), `border-radius: inherit`, `pointer-events: none`.
- Confirmação: pill de vidro (`--oh-glass-card`), borda cyan, sombra float; entrada
  com `popoverIn` (220 ms). `prefers-reduced-motion`: sem glow/pulso, mantendo o
  tempo e o fill.
- `aria-haspopup="dialog"` no gatilho; a janela é `role="dialog"` com
  `aria-label="Confirmar ação"`; o "Sim" é um botão normal.
- **Clique sempre funciona**; teclado usa o clique (Enter/Espaço). O drill-hover é
  um atalho adicional, não a única via.
- **Sem som/toast:** o áudio da reunião seria captado.

## 8. Testes

- **Puros:** `clampDwellMs`/`dwellLabel`; posicionamento puro em Rust
  (`dwell_popover_position`: acima/abaixo/clamp).
- **Hook/componentes:** `useDwellTimer` (fake timers), `DwellButton` (hover arma,
  saída cancela, clique imediato, desligado), `DwellPopoverWindow` ("Sim" por tempo e
  por clique, Escape).
- **Provider:** `dwell_show` chamado ao armar; `dwell_closed` confirmado executa a
  ação; cancelado não executa; hover do gatilho é repassado (`dwell_trigger_hover`).
- **Popover:** grace local pausa ao entrar na janela; fecha após `dwell_ms` fora
  das duas áreas; "Sim" por tempo/clique; Escape; **poll do cursor** cobre hover
  obsoleto na segunda abertura.
- **Cursor:** a janela recebe a geometria **real** do core no `dwell_context`;
  `cursorSampleFrom` é puro (físico → CSS, janela/Sim). Permissão apenas
  `core:window:allow-cursor-position`.
- **Notch:** só Resumo/Sugestões usam `DwellButton`; clique continua imediato; os
  demais botões não mudam.
- **Rust:** `Settings` (default, legado, clamp/validate), posicionamento e
  `AppState` com a nova porta.
- **Manual:** acima/abaixo perto das bordas, stealth/click-through, troca da duração,
  desligado, reduced motion.
