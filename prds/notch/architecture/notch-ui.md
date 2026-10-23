# Arquitetura — Notch e painel em abas

Especificação técnica do **notch** (barra do overlay em dois estados, por ícones) e
do **painel da sessão em abas** (Transcrição + Resumo), além da **troca de idioma
em tempo real** e do **drill-hover** (BR-036). Paridade com o **Perssua 0.27.5**
(BR-015); desvios conscientes no §6 e evidência no §7.

---

## 1. Componentes

```
src/components/icons.tsx                 # SVGs próprios (BR-008): gear, transcript, summary,
                                         #   sparkle, history, eye, chevron, grip, language
src/components/IconButton.tsx            # botão só-ícone (title + aria-label + data-interactive)
src/components/ShortcutBadges.tsx        # accelerator → badges (Ctrl / Shift / S)
src/components/TransportIcons.tsx        # (existente) play/pause/stop

src/features/overlay/ActionBar.tsx       # dois estados (repouso/ativo), já por ícones
src/features/overlay/Overlay.tsx         # casca; assina usePanelOpen("chat"|"history")
src/features/session/SessionControls.tsx # transporte (toggle único) + canais + "Começar a ouvir"

src/features/panel/PanelTabs.tsx         # pill de abas acessível (Transcrição | Resumo)
src/features/panel/PanelWindow.tsx       # host das abas (substitui ChatWindow.tsx)
src/features/panel/TranscriptionView.tsx # conteúdo que hoje é a janela `chat`
src/features/panel/SummaryView.tsx       # conteúdo que hoje é a janela `summary`
src/features/panel/LanguageBar.tsx       # barra inferior: seletor de idioma + estado

src/components/dwell/DwellButton.tsx     # opt-in: fill + clique imediato (BR-036)
src/components/dwell/DwellProvider.tsx   # enabled + durationMs (um estágio)

src/core/domain/shortcuts.ts             # + parseAccelerator / acceleratorBadges (puro)
src/core/domain/settings.ts             # + panel_tab; - summary_position/summary_size
src/adapters/ipc/settings.ts            # session_set_language (novo)
src/adapters/ipc/panels.ts              # PanelLabel sem "summary"
```

- A **janela Tauri continua com o label `chat`** (evita migrar
  `chat_position`/`chat_size`); o que muda é o conteúdo e o título
  (`GhostWhisper — Transcrição`).
- `PanelWindow` decide a aba por `Settings.panel_tab` (`"transcription" | "summary"`,
  default `"transcription"`).

## 2. Notch em dois estados

Estado derivado de `useSessionState()`:

| Estado | Condição | Conteúdo da barra |
|---|---|---|
| **Repouso** | `!state.listening` | alça · marca · **"Começar a ouvir"** (esmeralda + badges) · ocultar · Histórico · **engrenagem (último)** |
| **Ativo** | `state.listening` | alça · marca · **switcher** (Transcrição + waveform · play/pause · Resumo · Sugestões) · **timer** · canais · stealth · **ocultar · Histórico** · **engrenagem (último)** |

- **Não** há nova `Surface`: continua `actionbar | hidden`
  (`ports/overlay_window.rs`), como já vigente desde a task 07 do PRD chat.
- A **casca** não assina `audio_level`; o waveform tem assinatura própria e escreve
  CSS vars por `ref` (sem re-render — ISS-020).
  > **Revisão (2026-10-19, chat v0.2/BR-040):** os medidores e o atalho
  > "Sugestões" saem da aba Transcrição; o waveform e o atalho ficam no notch.
  > Ver [`../../chat/architecture/transcription-panel.md`](../../chat/architecture/transcription-panel.md)
  > §2.
- Cores pelas regras da BR-035: ação primária **esmeralda** (`--oh-success`),
  foco/live **cyan** (`--oh-accent`), IA **violeta** (`--oh-ai`), "pausado" no
  `--oh-outline` (sem âmbar).

### 2.1 Notch ativo (task 08)

Layout do protótipo (sem copiar Tailwind/Material — BR-008/BR-035):

```
[grip] ● GhostWhisper │ [🎙 Transcrição ▁▃▅▂] [⏸/▶] [📄 Resumo] [✦ Sugestões] │
[● 00:14:32]  [🎙 canais] [👁 stealth] [⤡ ocultar] [🕘 Histórico] [⚙ configurações]
```

- **Switcher** (`role="tablist"`): Transcrição/Resumo abrem o painel na aba
  correspondente (`panel_open` com `tab` → evento `panel_tab`); Sugestões alterna
  a janela. A aba ativa fica em `surface-container-highest` + texto cyan.
- **Histórico** e **ocultar** vivem no **grupo trailing**, como ícones (sem
  rótulo): Histórico antes da engrenagem; ocultar usa o ícone de **recolher**
  (duas setas diagonais), como no protótipo.
- **Waveform**: 4 barras cyan; nível vem de `audio_level` (máx. dos canais) por
  `ref`/CSS vars; pausado → estático e esmaecido; sem eventos → baixo (sem fake).
- **Play/pause**: contextual (gravando → `pause`; pausado → `play`); encerrar
  continua **só** no timer (BR-022).
- **Timer**: `HH:MM:SS` mono, tabular, dot esmeralda/outline; clique abre a
  confirmação de encerramento.
- **Canais**: popover (Ambos/Mic/Sistema) + **Nova sessão**; troca ao vivo (BR-019).
- **Menus (canais e agentes)**: janela Tauri própria (`menu`) ancorada **fora do
  notch** (abaixo da barra ou acima do pill, 6 px) e com altura dinâmica pelo
  conteúdo; seleção/Escape/clique fora fecham. A janela do notch não muda de
  tamanho nem de posição (BR-072, ISS-062/task 20).
- **Trailing**: ocultar, Histórico e, **por último**, **Configurações**
  (engrenagem) — correção do protótipo, que a omitiu.
- **Repouso** mantém a estrutura atual; só o re-skin da task 07.
- Detalhes, critérios e roteiro: [`../tasks/08-task.md`](../tasks/08-task.md).

### 2.2 Drill-hover em Resumo/Sugestões (task 11)

- **Gatilhos:** somente **Resumo** e **Sugestões**. Clique executa a ação na hora;
  permanecer com o cursor preenche o item (0,5 s por padrão) e, ao completar,
  executa a ação **diretamente**.
- **Sem confirmação intermediária:** não há janela `dwell`, popup "Sem clique",
  botão "Sim" nem segundo timer. Sair antes do fim cancela; completar executa uma
  vez.
- **Config:** `dwell_enabled` (padrão ligado) e `dwell_ms` (padrão 500 ms) nos
  Ajustes. Sem som/toast.
- Detalhes: [`drill-hover.md`](drill-hover.md) e BR-036.

## 3. Pill de abas

- Um `role="tablist"` com `role="tab"`/`aria-selected`; teclado (setas) e foco
  visível.
- Visual: pill com fundo de borda suave; a aba ativa recebe fundo preenchido, como
  os `.unified-control-button` já medidos em `chat-ui.md` §3. Os valores exatos
  vêm da extração do §7.
- Ícones da aba: **Transcrição** = balão/legenda; **Resumo** = documento.
- A aba **não** desmonta o conteúdo da outra: `TranscriptionView` e `SummaryView`
  ficam montados (escondidos por `hidden`) para não reiniciar rolagem/streaming
  (CA-03.6). O resumo só faz chamadas com a **aba Resumo ativa** (CA-05.2).

## 4. Troca de idioma em tempo real

O identificador do idioma deve valer para o **próximo passe**, sem recarregar o
modelo. Isso usa o mesmo padrão já existente para o **prompt** (que também é
parâmetro de cada passe).

### 4.1 Núcleo (Rust)

```
adapters/speech/whisper_speech_to_text.rs
  - language: String            →  language: Mutex<String>
  - fn update_language(&self, lang: impl Into<String>)   # espelha `update_prompt`
  - run_worker(): decode_with(..., &this.language_snapshot(), &this.prompt_snapshot(), ...)

adapters/speech/whisper_speech_engine_factory.rs
  - CachedEngine: remove `language` da **chave** (mantém `model`, `use_gpu`)
  - create(): ao reusar o motor em cache, chama update_language(lang) **e**
    update_prompt(decode_prompt(...))  # a regra de idioma vive no prompt

ports/speech_engine_factory.rs
  - fn update_language(&self, language: &str) -> Result<()>   # propaga ao motor em cache

ports/session_controller.rs
  - fn set_language(&self, language: &str) -> Result<()>      # persiste + propaga

application/session_service.rs
  - set_language(): Settings.source_language = lang → store.save()
                     → speech_factory.update_language(map_whisper_language(lang))
                        (+ update_prompt(decode_prompt(whisper_prompt, lang)))

ipc/session_commands.rs
  - #[tauri::command(async)] session_set_language(language: String) -> Result<()>
```

- Reusa `domain::transcription::{map_whisper_language, decode_prompt}` — uma única
  implementação (BR-016).
- Sem sessão ativa, `set_language` só persiste; a próxima sessão já nasce no idioma.
- O motor é **um só** entre canais (contexto compartilhado), então a troca vale para
  microfone e sistema de uma vez.

### 4.2 UI

- `LanguageBar` lê `settings.source_language` e chama `session_set_language`;
  reusa `useSettings` + `reload()` para refletir na aba das Configurações.
- Mapa de rótulos já existe em `core/domain/settings.ts`
  (`SOURCE_LANGUAGE_LABELS`); a barra usa o mesmo, sem duplicar (R14).
- Estado otimista: a UI muda na hora e o erro (se houver) volta como aviso, como as
  demais chamadas de settings.

## 5. Migração (aposentar `summary`)

| Item | Antes | Depois |
|---|---|---|
| `tauri.conf.json` | janela `summary` (520×640) | **removida** |
| `capabilities/default.json` | `…, "summary", …` | sem `"summary"` |
| `domain::settings::PANEL_LABELS` | `["summary","suggestions","chat","history"]` | `["suggestions","chat","history"]` |
| `Settings` | `summary_position`, `summary_size` | **removidos** (serde ignora órfãos no load) |
| `lib.rs` | tracker/boot/tray com `summary` | itera `PANEL_LABELS`; item de tray "Abrir resumo" → **abre a janela `chat` na aba Resumo** |
| `App.tsx` | rota `summary` | removida (a rota `chat` cobre) |
| `panels.ts` | `PanelLabel` inclui `"summary"` | sem `"summary"` |
| `ChatHeaderActions` | `panelOpen("summary")` (e o botão "Sugerir") | só o atalho `panelOpen("suggestions")`; o "Resumo" é a aba ao lado e o "Sugerir" saiu (revisão 2026-10-19) |

- O painel em abas é hospedado pela janela `chat`; a rota `chat` permanece e aceita
  `panelTab` para abrir já na aba certa.
- `usePanelOpen` continua com os rótulos de **janela** (`chat`, `suggestions`,
  `history`); a **aba** é estado de `Settings`, não um painel.

## 6. Desvios conscientes do Perssua (R12)

| Desvio | Por que |
|---|---|
| Ação primária **verde** (o Perssua usa azul cheio) | evitar copiar a paleta; verde = "começar", azul fica para "ligado" |
| **Sem** play/pause/stop na barra inferior da Transcrição | o transporte vive no notch (BR-022); a barra inferior tem só o idioma |
| Ícones **SVG próprios** | BR-008: não usar a fonte Material Icons de terceiros |
| Automático do resumo só com a **aba Resumo ativa** | contenção de custo com a janela sempre aberta (revisa a BR-026) |
| "Sessão" **não** implementada | escopo desta PRD; desenho apenas reserva a aba |

## 7. Evidência no bundle do Perssua

Extração: `/tmp/opencode/perssua/squashfs-root/resources/app.asar` (de
`examples/Perssua-0.27.5-x86_64.AppImage`). Preencher aqui **na implementação**, com
a ferramenta REA, sem duplicar o que já está em
[`../../chat/architecture/chat-ui.md`](../../chat/architecture/chat-ui.md) §3/§6.

| Assunto | Arquivo no bundle | Âncora |
|---|---|---|
| Pill de abas do notch | _a registrar (task 01)_ | _classe do tablist/aba ativa_ |
| Botão "Start Listening" | _a registrar (task 02)_ | _classe + cor_ |
| Seletor de idioma da barra inferior | _a registrar (task 04)_ | _classe + comportamento_ |

> Reproduzir: `python3 /tmp/opencode/asar.py cat <caminho>` para os arquivos já
> extraídos, ou a ferramenta REA (`open_binary` → `analyze_javascript_application`)
> para extrair os módulos do renderer.
