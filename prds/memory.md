# Memory — GhostWhisper

Nota histórica do projeto: decisões tomadas, descobertas e o que foi feito, em
ordem cronológica. Serve como lembrete para retomar contexto.

---

## 2026-10-17 — Kickoff e engenharia reversa do Perssua 0.28.0

### Contexto
Início do projeto GhostWhisper a partir da análise do binário
`Perssua-0.28.0-arm64.AppImage`.

### Descobertas da análise (estática, sem execução)
- O AppImage é um **ELF arm64** (host da análise é x64 → nada foi executado) com
  um **squashfs** embutido no offset `0x304B0`, contendo um app **Electron**.
- Estrutura: `resources/app.asar` (código), `resources/app.asar.unpacked`
  (módulos nativos), `resources/audio-driver` (driver virtual baseado no
  **BlackHole**, macOS CoreAudio).
- **Main process legível** (`electron/*.js`, ~120 arquivos com nomes/comentários).
  Renderer **minificado e sem source maps**.
- Fluxo: overlay Electron transparente/frameless/always-on-top → servidor
  **Express HTTPS local** (127.0.0.1:3500/3501) servindo o React buildado;
  bridge via `preload.js` (1,2 MB).
- Dados: **better-sqlite3** com tabelas `sessions`, `messages`, `transcriptions`
  e FTS5.
- Backend original: **Firebase** (`us-central1-strawberry-cb687.cloudfunctions.net`),
  auth browser/loopback PKCE, remote config, PostHog + Sentry.
- Recursos nativos: `whisper.cpp` + **Silero VAD** (`ggml-silero-v5.1.2.bin`,
  MIT), `llama.cpp`, MCP SDK, Codex OAuth.
- **Stealth/invisibilidade** usa `setContentProtection(true)` (macOS/Windows);
  **no Linux é no-op**. E recurso é **premium-gated**.
- Produtos white-label: `full`, `cody`, `notetaker`, `reuniao` (sistema de
  "flavors" com pruning de recursos no build).

### Causas de quebra no Linux (evidências no binário)
1. Sem exclusão de janela da captura de tela no Linux.
2. Áudio do sistema via `getDisplayMedia` (instável); PipeWire capturer
   desabilitado em X11 (`linuxAudioCaptureFlags.js`).
3. Wayland: workaround inteiro (`linuxWaylandSafeWindow.js`) e fallback
   obrigatório para XWayland (`ozone-platform=x11` em `main.js`).
4. `whisper-bin`/`llama-bin`/`audio-driver` ausentes no build arm64 Linux.

### Decisões de projeto tomadas com o usuário
- Escopo do MVP: **Linux-first** e focado.
- Modelo: **local-only / BYO keys** (sem backend próprio).
- Stack: **Tauri (Rust) + React**.
- Documentação **spec-driven** em `prds/`.

### Entregue neste dia
- Estrutura `prds/` com `business-rules.md`, `memory.md`, `mvp/prd.md`,
  `mvp/architecture/`, `mvp/tasks/`.
- PRD do MVP com tech spec e primeiras tarefas.
- README inicial com stack, instalação e configuração.

### Pendências / próximos passos
- Executar a Tarefa 00 (bootstrap do projeto).
- Validar captura de áudio do sistema (monitor de sink) nas distros-alvo.
- Spike de Wayland (`gtk-layer-shell`) e de output virtual para "invisibilidade".

---

## 2026-10-17 — Regras de engenharia e arquitetura hexagonal

### O que foi feito
- Criadas as regras de engenharia em [`.agents/rules.md`](../.agents/rules.md):
  arquitetura hexagonal (ports & adapters), injeção de dependência, SOLID,
  testabilidade e nomenclatura sem prefixo `I`.
- Criado o [`AGENTS.md`](../AGENTS.md) na raiz com as premissas e regras.
- Registradas as decisões em `business-rules.md`: **BR-011** (hexagonal + DI),
  **BR-012** (SOLID), **BR-013** (testabilidade), **BR-014** (nomenclatura).
- Criado `prds/mvp/architecture/engineering-guidelines.md` com catálogo de portas,
  exemplo de porta/adaptador, casos de uso e estratégia de testes.
- Reestruturada a `architecture.md` para o layout hexagonal
  (`domain`/`application`/`ports`/`adapters`/`ipc` e composition root em `lib.rs`).
- Atualizados os caminhos das tarefas 00–09 e dos docs `audio-pipeline.md` e
  `overlay.md` para o novo layout.

### Convenções fixadas
- Porta: `TranscriptionsRepository`; adaptador local:
  `LocalTranscriptionsRepository`; adaptador de tecnologia:
  `OpenAiChatBotProvider`; dobra de teste: `InMemoryTranscriptionsRepository`.
- Adaptadores só são instanciados no composition root (`src-tauri/src/lib.rs`).
- `domain`/`application` não fazem I/O; toda porta tem dobra de teste.

### Pendências / próximos passos
- Iniciar a Tarefa 00 criando o esqueleto conforme o layout hexagonal.

---

## 2026-10-17 — Tarefa 00 (bootstrap) concluída

### O que foi feito
- Scaffold **Tauri v2 + React/Vite/TS** criado com o layout hexagonal:
  `src-tauri/src/{domain,application,ports,adapters,ipc}` + `state.rs`/`error.rs`
  e composition root em `src-tauri/src/lib.rs`.
- Frontend: `src/App.tsx` (roteia por rótulo de janela), `src/App.test.tsx`,
  `src/styles/globals.css` e esqueleto `src/core/*` e `src/adapters/ipc`.
- Tauri: `tauri.conf.json` com a janela `overlay` (transparente, frameless,
  always-on-top, skip-taskbar), plugins `opener`/`store`/`global-shortcut`,
  `capabilities/default.json` mínimo.
- Rust: `AppState`, `GhostWhisperError` (thiserror + Serialize), comando `ipc::ping`
  e módulos de camada. `scripts/package-linux.sh` (stub) e `.gitignore`.

### Verificações
- `pnpm install` ok; `pnpm test` → 1 teste passou; `pnpm build` (tsc + vite) ok.
- Núcleo puro (`domain/application/ports/state/error`) compilou, passou em
  `cargo test` e `cargo clippy` (sem warnings), validado em crate isolado por
  falta de libs de sistema.
- `cargo fetch` resolveu todas as dependências do app Tauri.
- `pnpm tauri info` validou a config (`framework: React`, `bundler: Vite`).
- `cargo fmt --all --check` limpo.

### Bloqueio de ambiente (registrado)
Este host (Ubuntu x64, sem `sudo`) não tem `webkit2gtk-4.1`, `rsvg2`, `alsa`,
`libpulse` — necessário para buildar/rodar o app Tauri. Comando de instalação
documentado na Tarefa 00 e no README. Toolchain Rust instalada via rustup
(cargo/rustc 1.99.1, rustfmt, clippy).

### Corrigido durante a implementação
- Aviso `dead_code` no campo reservado de `AppState` (anotado com `#[allow]`).

### Pendências / próximos passos
- Iniciar a Tarefa 01 (janela overlay) — não depende das libs de áudio.
- Quando houver `sudo`/libs, validar `pnpm tauri dev` abrindo a janela.

### Atualização (mesmo dia) — ambiente destravado e Tarefa 00 verificada
- Usuário instalou as libs do Tauri (`webkit2gtk-4.1`, `rsvg2`, `alsa`, `pulse`,
  etc.). `pnpm tauri info` agora marca tudo ✔.
- Gerado ícone (`assets/icons/icon-source.png` → `src-tauri/icons/` via
  `pnpm tauri icon`), exigido pelo `generate_context!`.
- Corrigido o path do comando Tauri: `generate_handler![ipc::ping::ping]`
  (o helper `__cmd__ping` vive no módulo `ipc::ping`).
- Verificação completa: `cargo test` (3 testes), `cargo clippy` limpo,
  `cargo fmt --check` limpo, `cargo build` ok e smoke run do binário
  (`exit=124`, ficou ativo sem erros). Frontend: `pnpm test`/`pnpm build` ok.
- **Tarefa 00 fechada como CONCLUDED.**

---

## 2026-10-17 — Tarefa 01 (overlay) concluída

### O que foi feito
- **Portas:** `OverlayWindowController`, `PointerSource`, `ClickThroughController`.
- **Adaptadores:** `TauriOverlayWindow`, `CursorClickThroughController`
  (thread de amostragem do cursor + `set_ignore_cursor_events`),
  `X11PointerSource` (`XQueryPointer`) e `NullPointerSource` (fallback Wayland).
- **Domínio:** `domain/geometry.rs` com `Point`/`Rect`/`should_pass_through`
  (lógica pura, 5 testes).
- **IPC:** `overlay_toggle`, `overlay_set_stealth`, `overlay_set_click_through`,
  `overlay_set_interactive_regions`.
- **Composition root:** `lib.rs` monta os adaptadores no `.setup` e injeta em
  `AppState` (agora guarda as portas).
- **Frontend:** `Overlay.tsx`, `useInteractiveRegions`, `adapters/ipc/client.ts`.
- Dep nova: `x11rb` (consulta de cursor X11).

### Verificação
- `cargo test` → **9 testes**; `cargo clippy`/`cargo fmt` limpos;
  `cargo build` + smoke run ok (`exit=124`). `pnpm test`/`pnpm build` ok.

### Limitações
- Click-through automático só em X11; Wayland usa `NullPointerSource`.
- Verificação visual de hover/clique requer sessão gráfica manual.

---

## 2026-10-17 — Tarefa 02 (captura de microfone) concluída
- Portas `AudioSource`/`AudioConsumer`, domínio `AudioDevice`/`TARGET_SAMPLE_RATE`.
- `pcm_resampler` (downmix + reamostragem linear para 16 kHz; 6 testes),
  `CpalMicrophoneSource` (thread dedicada; f32/i16/u16), `LevelMeter`
  (evento `audio_level`).
- IPC: `audio_list_devices`, `audio_start`, `audio_stop`, `audio_is_running`.
- `AppState` ganhou `Arc<dyn AudioSource>`; composition root updated.
- Dep `cpal`; `profile.dev.incremental = false` (economia de disco).
- Verificação: `cargo test` → 16 testes; clippy/fmt limpos; build + smoke ok.
- Limitação: sem dispositivo de áudio no host; enumeração/nível não exercitados
  de ponta a ponta.

## 2026-10-17 — Teste manual do overlay (feedback do usuário)
- Usuário rodou o app e viu a interface (ActionBar + botão Stealth).
- **Bug registrado:** clique no botão não capturou de forma confiável →
  [ISS-001](mvp/issues.md) (click-through/posição do cursor). Fica no radar;
  será endereçado na Tarefa 07.

## 2026-10-17 — Follow-up 5 (uma linha e depois nada + retorno de download)
- **Relato do usuário:** "só escreve uma linha de transcrição, depois não mais";
  e ao trocar para um modelo maior, nenhum aviso de download.
- **Bug 1 (crítico) — três causas somadas no pipeline de streaming:**
  - o **preview decodificava o turno inteiro** a cada ~900 ms (custo crescente →
    saturação do ASR); agora decodifica só a **janela de 2 s**;
  - o **corte no teto** devolvia `buffered_ms` (buffer inteiro, 30–60 s) em vez do
    teto; agora corta **em `max_ms`**;
  - o **throttle do VAD** comparava milissegundos absolutos e travava após
    consumir um turno; agora conta **amostras de crescimento**.
  - `process()` ainda decodificava cada span direto (contornando o streaming) —
    unificado em `consume + tick`; `stop()` agora **fecha o turno em aberto**.
- **Bug 2 — retorno de download:** `preload()` em background no início da sessão
  + evento `asr_status` (preparing/ready/failed) + banner no overlay
  (`AsrStatusBanner`, hook `useAsrStatus`). `AsrNotReady`/`AsrBusy` fazem o
  pipeline aguardar em vez de travar; `try_lock` elimina o **deadlock** do ASR
  após timeout.
- **Verificação:** 114 testes Rust (3 novos de regressão) + 18 frontend (4 novos);
  E2E JFK real verde (preview→final, beam search); clippy 0, fmt/tsc limpos;
  `.deb` 8,10 MiB e AppImage 85,97 MiB reconstruídos.

## 2026-10-17 — Follow-up 4 (streaming de transcrição, inspirado no Perssua)
- **Análise (REA):** extraí o `app.asar` do `Perssua 0.27.5` (Electron) e mapeei
  `electron/local-whisper` (12 arquivos, 104 findings). Ele usa: buffer de turno
  cap 20/60 s, **preview** guloso revisável + **passe final** com beam search,
  gate de fala por RMS, filtro de alucinação, timeout por áudio e lock de GPU.
  A captura dele (getDisplayMedia/CoreAudio) **não** é portável; mantivemos
  `.monitor`/pactl (melhor no Linux).
- **Implementado em Rust (ISS-007):**
  - `SpeechQuality::{Preview,Final}` na porta `SpeechToText`; preview=guloso,
    final=beam(5);
  - pipeline com **buffer de turno** (teto 60 s), **gate de fala sustentada**
    (RMS≥0.008 por 160 ms) e **filtro de alucinação** (frases-modelo, `[...]`,
    símbolos, repetição adjacente);
  - `WhisperSpeechToText` com **timeout por pedido** (30 s / 15 s por s de áudio
    / teto 120 s) rodando em thread isolada (`Arc<Mutex<..>>`, sem ponteiros);
  - `SerialisingSpeechToText`: **uma decodificação por vez** entre canais;
  - telemetria de **RTF** por passe nos logs.
- **UI:** preview em itálico discreto; final substitui pelo mesmo id `{session}-t{n}`
  (upsert no hook `useLiveTranscription`); só o final vai ao banco.
- **Verificação:** 112 testes Rust + 14 frontend; integração JFK passa com beam
  search ativo; clippy/fmt/tsc limpos; `.deb` (8,09 MiB) e AppImage (85,95 MiB)
  reconstruídos.

## 2026-10-17 — Follow-up 3 (lentidão/travamento + teste de áudio)
- **ISS-005 (crítico):** `TranscriptionPipeline` tomava o **buffer inteiro** a
  cada ciclo; se o ASR ficava mais lento que o tempo real, o próximo bloco era
  maior → **espiral** (um `whisper_full` gigante, sem logs) → "para de
  transcrever". Reescrito: janela de 2 s com **corte em silêncio**, teto de 10 s
  para fala contínua, **teto de buffer de 30 s** (backpressure), **watchdog** de
  ausência de áudio e logs de diagnóstico por janela. Whisper com
  `n_threads = núcleos/2` (2 canais não brigam por CPU).
- **ISS-006:** nova aba **"Teste de áudio"** (canal + dispositivo + nível ao
  vivo), desabilitada durante sessão ativa; sem pré-download de modelo.
- Verificação: **102 testes Rust** + 11 frontend; clippy/fmt limpos; integração
  real transcreve o JFK ("country"); boot ok; `.deb` (8,07 MiB) e AppImage
  (85,94 MiB) reconstruídos.

## 2026-10-17 — Follow-up 2 (3 ajustes do teste do AppImage)
- **ISS-002 (Ajustes só abria 1×):** janela `settings` era destruída ao fechar.
  Agora `CloseRequested → prevent_close()+hide()`; `settings_open` faz
  `unminimize+show+set_focus` e loga falhas; `alwaysOnTop`; erro visível no overlay.
- **ISS-003 (`gpt-6-luna` 400):** provider recusava `max_tokens`. Em
  `400 unsupported_parameter`, renomeia `max_tokens → max_completion_tokens` e
  remove outros recusados, reenviando (até 3×). Teste de integração com mock de
  sequência (400→200).
- **ISS-004 (idiomas):** seletores **Auto/PT/EN** em origem e destino. Whisper
  agora honra `source_language` (`set_language`) **e** `whisper_model` (lidos das
  settings na fábrica, a cada sessão). Destino `auto` (ou igual à origem) **não
  traduz**. `validate()` restringe idiomas; `Settings::sanitized()` corrige
  configs antigas no load. Prompts usam nomes legíveis ("português"/"inglês").
- Modelos whisper seguem **sob demanda** (baixados só ao iniciar sessão com o
  modelo escolhido) — sem pré-download; UI mostra tamanho aproximado.
- Verificação: **94 testes Rust** (+14) + **11 frontend**; clippy/fmt limpos; boot
  loga "click-through via X11 input-shape"; `.deb` (8,07 MiB) e AppImage
  (85,92 MiB) reconstruídos.

## 2026-10-17 — Follow-up pós-MVP (5 correções do teste do AppImage)
- **Click-through definitivo (ISS-001):** `adapters/overlay/x11_input_shape_click_through.rs`
  aplica `SHAPE/ShapeRectangles (kind=Input)` = união dos rects `[data-interactive]`
  (xid via `raw-window-handle`); sem polling/atraso. Polling vira fallback.
- **Configurações não abriam:** ACL negava `window.show/set_focus`; adicionado o
  comando Rust `settings_open`. Idem `overlay_start_drag`.
- **Tray sempre visível:** feature `tray-icon` + `TrayIconBuilder` (menu: overlay,
  sessão, configurações, sair); tray não-fatal no boot. `.deb` já traz
  `libayatana-appindicator3-1` (o Tauri adiciona).
- **Arraste + posição (opção A):** handle na ActionBar; `settings.overlay_position`
  persistida e restaurada; janela para de reancorar pela base após o arraste.
- **Áudio duplo (mic + sistema):** `SessionController::start(&[AudioChannel])`,
  `SpeechEngineFactory` (Whisper **por canal**, 2 contextos), `audio_level` com
  canal, `session_state.channels`, janela de transcrição **6 s → 3 s**; UI com
  "Ambos/Mic/Sistema" e dois medidores.
- Verificação: 80 testes Rust + 10 frontend; clippy/fmt limpos; boot loga
  "click-through via X11 input-shape"; `.deb` (8,06 MiB) e AppImage (85,92 MiB)
  reconstruídos.
- Obs. ambiente: `GTK_PATH` vazando do VS Code (snap) quebra o binário; rodar de
  terminal limpo.

## 2026-10-17 — Tarefa 10 (empacotamento + spike Wayland) concluída — MVP completo
- Bundle: `tauri.conf.json` com ícones e deps de runtime; `.deb` (7,92 MiB,
  `open-sua`, Depends `libasound2t64|libasound2, libpulse0, libwebkit2gtk-4.1-0,
  libgtk-3-0`) e AppImage (85,64 MiB, extração ok). `scripts/package-linux.sh`
  aceita `deb` ou `deb,appimage`. Release roda.
- Spike `prds/mvp/architecture/wayland-spike.md`: **Wayland nativo NO-GO**
  (sem gtk-layer-shell, sem cursor global → quebra click-through, atalhos exigem
  portal, output virtual só em wlroots); **invisibilidade NO-GO** (BR-006).
- README: estado (MVP concluído), instalação real, atalhos, aviso Wayland.
- Verificação: 77 testes Rust + 10 frontend; clippy/fmt ok; bundles inspecionados.
- Pendências: matriz de instalação em VM (Ubuntu 24.04/Fedora 40) e validação
  manual em GNOME/KDE.

## 2026-10-17 — Tarefa 09 (atalhos globais + stealth) concluída
- Domínio `ShortcutAction`/`ShortcutBinding`/`ShortcutConflict`; porta
  `GlobalShortcutRegistry`; adaptador `TauriGlobalShortcutRegistry`
  (`tauri-plugin-global-shortcut`).
- Settings ganharam `stealth` e `shortcuts` (persistidos); `settings_set` reaplica
  e devolve conflitos; `overlay_set_stealth` persiste; boot aplica atalhos+stealth.
- Ações: mostrar/ocultar, iniciar/encerrar sessão, alternar stealth, pedir
  sugestão (publica evento `insight`).
- IPC `shortcuts_get/active/set/catalog`; UI de atalhos + stealth nas settings;
  `useInsightFeed` abre o painel ao receber `insight`.
- Verificação: 77 testes Rust + 10 frontend; smoke run registrou os 4 atalhos no
  X11 sem conflitos; clippy/fmt ok. Próxima: 10 (empacotamento + spike Wayland).

## 2026-10-17 — Tarefa 08 (tradução ao vivo + insights) concluída
- Portas `Translator`/`InsightGenerator`; `LlmController::complete` reintroduzido.
- Casos de uso `TranslationService` (prompt por idioma, temperatura 0) e
  `InsightService` (contexto da sessão, limite de 4.000 chars); IPC
  `llm_translate`/`llm_insight`.
- Frontend: `TranslationView` (lado a lado + copiar), `InsightsPanel`
  (sugerir/copiar/limpar), hooks `useTranslation` (fila serial)/`useInsight`/
  `useSettings`, util `clipboard`.
- Verificação: 74 testes Rust (inclui fluxo real settings→segredo→fábrica→HTTP→
  prompt com servidor OpenAI-compatível local) + 8 frontend; clippy/fmt ok.
- Streaming de resposta adiado (respostas completas). Próxima: 09 (atalhos/stealth).

## 2026-10-17 — Tarefa 07 (UI do overlay) concluída + ISS-001 corrigido
- UI: hooks `useLiveTranscription`/`useAudioLevel`/`useSessionState`, adaptadores
  IPC de eventos/sessão/overlay, componentes `Overlay`/`ActionBar`/`LevelMeters`/
  `TranscriptView`/`SessionControls` e tema do overlay.
- Core: eventos `audio_level` e `session_state`; `EventPublisher` estendido;
  pipeline emite nível; sessão emite estado; `set_surface`
  (actionbar/expanded/hidden) + IPC `overlay_set_surface`.
- **ISS-001 FIXED**: entrada imediata na região, saída debounced (3 amostras),
  margem de 8px, polling 30ms, interativo por padrão; regiões republicadas via
  MutationObserver.
- Verificação: 66 testes Rust (4 do controlador) + 6 frontend; clippy/fmt ok;
  smoke run com duas janelas. Próxima: 08 (tradução/insights).

## 2026-10-17 — Tarefa 06 (settings + providers LLM / BYO key) concluída
- Domínio `Settings`/`ProviderKind` e tipos de LLM; portas `ChatBotProvider`,
  `ChatBotProviderFactory`, `SettingsStore`, `SecretStore`, `LlmController`.
- Adaptadores: `FileSettingsStore` (settings.json sem segredos),
  `KeyringSecretStore` (Secret Service) + fallback `InMemorySecretStore`,
  providers OpenAI/Gemini/OpenRouter/custom (via helper OpenAI-compatível) e
  `DefaultChatBotProviderFactory`; `LlmRouter` + `prompts.rs` na aplicação.
- IPC `settings_*` (chave nunca devolvida); UI `SettingsPage.tsx` + janela
  `settings`.
- HTTP com `ureq` (síncrono) em vez de `reqwest`; chave do Gemini em cabeçalho.
- Verificação: 55 testes; keyring real (ignored) ok; endpoint custom validado com
  servidor HTTP local (inclui caso 401 sem vazar a chave); settings.json sem
  segredos. Próxima: 07 (UI do overlay).

## 2026-10-17 — Tarefa 05 (persistência SQLite) concluída
- Schema (base no Perssua + extensões) com FTS5, triggers e `schema_version`.
- Portas `SessionRepository`/`TranscriptionsRepository` + `Clock`; adaptadores
  SQLite (`connection`, `LocalSessionRepository`, `LocalTranscriptionsRepository`)
  e dobras em memória.
- `SessionService` persiste sessão (start/end); `TranscriptionPipeline` persiste
  cada segmento; IPC `db_*` (list/get/session/search).
- `rusqlite` bundled (FKs ativas; testes inserem sessões antes).
- Removido o repositório de `messages` (dead code sem chat; tabela mantida).
- Verificação: 33 testes; app cria o `.db` com todas as tabelas + FTS5. ISS-001
  (clique) segue aberto no radar.

## 2026-10-17 — Tarefa 04 (VAD + ASR) concluída
- Domínio de transcrição; portas `ModelStore`, `VoiceActivityDetector`,
  `SpeechToText`, `EventPublisher`, `SessionController`.
- Adaptadores: `FileModelStore` (download + SHA-256), `SileroVoiceActivityDetector`
  (VAD do whisper.cpp), `WhisperSpeechToText` (whisper-rs), `TauriEventPublisher`.
- Aplicação: `TranscriptionPipeline` (buffer→VAD→ASR→eventos) e `SessionService`.
- IPC de sessão; fiação no composition root (modelos em `app_data_dir/models`).
- `whisper-rs 0.16` traz VAD nativo (`WhisperVadContext`) — reusa o Silero ggml.
- **Verificação real:** teste de integração (ignored) baixou os modelos e
  transcreveu `jfk.wav` pelo pipeline completo (contém "country"). 30 unitários;
  clippy/fmt limpos.

## 2026-10-17 — Tarefa 03 (áudio do sistema) concluída
- Porta `VirtualSinkManager`; adaptadores `PactlVirtualSinkManager`,
  `PulseSystemAudioSource`, helper `pactl` (run/parse).
- Captura do monitor via `libpulse-simple` (servidor remixa para mono 16 kHz).
- IPC: `system_audio_*` e `system_sink_create/remove`; `AppState` ganhou
  `system_audio` e `virtual_sink`.
- Deps: `libpulse-binding`, `libpulse-simple-binding`.
- **Verificação real:** servidor PipeWire ativo; testes de integração (ignored)
  listaram monitores e capturaram áudio do monitor padrão tocando um tom via
  `paplay` (amostras > 0). `cargo test` → 21 unitários; clippy/fmt limpos.

## 2026-10-17 — Alerta de ambiente: disco raiz 100% (resolvido)
- O `/` (234G) ficou **100% cheio**; o linker morreu com Bus error na primeira
  tentativa. `~/.npm` (18G) e `~/.cache` (29G) são os maiores consumidores
  (caches do ambiente do usuário). Liberado `target/debug/incremental` (1,3G) e
  desativado o incremental no perfil dev. **Espaço segue apertado (~5G)** —
  exige limpeza antes de `tauri build`/empacotamento (Tarefa 10).

## 2026-10-17 — Renomeação: OpenSua → GhostWhisper
- **Escopo:** nome do produto em todo o código, UI, config, empacotamento e docs.
- **Mudanças:**
  - `productName` = `GhostWhisper`; `identifier` = `com.ghostwhisper.desktop`;
  - pacote/binário Rust `ghostwhisper` (lib `ghostwhisper_lib`), `OpenSuaError`
    → `GhostWhisperError`; banco `opensua.db` → `ghostwhisper.db`;
  - serviço do keyring `ghostwhisper` **com fallback de leitura** para `opensua`
    (a chave de API do usuário continua válida, sem redigitação);
  - UI: título, marca na ActionBar, Configurações, `index.html`, variáveis CSS
    `--opensua-*` → `--ghostwhisper-*`;
  - 18 documentos (`prds/`, `README.md`, `AGENTS.md`, `.agents/rules.md`) e o
    script de empacotamento.
- **Migração de dados (importante):** como o diretório de dados deriva do
  *identifier*, a renomeação criaria um diretório vazio e o app re-baixaria os
  modelos (GBs). `migrate_legacy_data_dir()` move `models/` e `settings.json`
  e renomeia o banco do diretório antigo; é idempotente e não sobrescreve dados
  existentes (3 testes cobrem os casos).
- **Verificação:** 117 testes Rust (3 novos de migração) + 18 frontend;
  clippy 0, fmt/tsc limpos. Pacotes **não** foram reconstruídos a pedido.

