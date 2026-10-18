# GhostWhisper

> Copiloto de reunião em tempo real, **open-source**, **Linux-first** e **local-only**.
> Ouve o áudio da reunião, transcreve, traduz e sugere respostas — numa janela
> overlay discreta que roda separada da reunião.

GhostWhisper é uma reimplementação independente inspirada no comportamento do Perssua,
construída do zero para funcionar bem no Linux. Não usa conta, não envia telemetria
e não exige assinatura: você traz sua própria chave de API (BYO key) ou roda modelos
locais.

---

## Por que existe

O app original é problemático no Linux: captura de áudio do sistema instável,
dependência de XWayland, problemas de janela/permissão e recursos "premium"
(como ficar invisível no compartilhamento de tela) bloqueados. O GhostWhisper mira
resolver o núcleo com qualidade no Linux e ser totalmente aberto.

## Stack

| Camada        | Tecnologia |
|---------------|------------|
| App shell     | **Tauri v2** (Rust) |
| Frontend      | React + Vite + TypeScript |
| Áudio (mic)   | `cpal` (ALSA/PulseAudio/PipeWire) |
| Áudio (sistema)| PipeWire / PulseAudio monitor de sink |
| VAD + ASR     | `whisper-rs` (whisper.cpp) + Silero VAD |
| LLM           | HTTP direto (OpenAI / Gemini / OpenRouter / custom) ou `llama.cpp` local |
| Persistência  | SQLite (`rusqlite`, FTS5) |
| Segredos      | Secret Service (libsecret) via `keyring` |
| Overlay       | Janela transparente Tauri; X11/XWayland (click-through por cursor) |

## Estado

**MVP concluído** (tarefas 00–10 em
[`prds/mvp/tasks/_tasks.md`](prds/mvp/tasks/_tasks.md)). Fluxo completo:
captura (mic/sistema) → VAD + Whisper local → transcrição ao vivo → tradução e
insights (BYO key) → persistência e busca. O MVP está descrito em
[`prds/mvp/prd.md`](prds/mvp/prd.md); a decisão sobre Wayland está no
[`wayland-spike.md`](prds/mvp/architecture/wayland-spike.md).

## Requisitos

- Rust (stable) + `cargo`
- Node.js 20+ e `pnpm` (ou `npm`)
- Dependências de sistema Tauri v2 (Linux):
  - Debian/Ubuntu: `libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev libasound2-dev libpulse-dev`
  - Fedora: `webkit2gtk4.1-devel openssl-devel alsa-lib-devel pulseaudio-libs-devel`
- Para áudio do sistema: PipeWire ou PulseAudio com monitor de sink disponível.

## Instalação (usuário final)

Alvos de distribuição do MVP: **`.deb`** e **AppImage** (X11/XWayland).

```bash
# .deb (Ubuntu/Debian) — instala e roda sem sudo no uso diário
sudo apt install ./GhostWhisper_0.1.0_amd64.deb
# ou:
sudo dpkg -i GhostWhisper_0.1.0_amd64.deb && sudo apt-get -f install

# AppImage — sem instalar
chmod +x GhostWhisper_0.1.0_amd64.AppImage
./GhostWhisper_0.1.0_amd64.AppImage
```

Dependências de runtime declaradas pelo `.deb`: `libwebkit2gtk-4.1-0`,
`libgtk-3-0`, `libasound2t64 | libasound2` (ALSA/`cpal`) e `libpulse0`.

> **Wayland:** o MVP roda em **X11/XWayland** (BR-007). Wayland nativo é
> **no-go** por ora (sem click-through por cursor, atalhos globais e ancoragem
> sem `gtk-layer-shell`); veja o
> [spike](prds/mvp/architecture/wayland-spike.md).

## Desenvolvimento

```bash
pnpm install
pnpm tauri dev      # abre o overlay em modo desenvolvimento
pnpm test           # testes do frontend (vitest)
cargo test --manifest-path src-tauri/Cargo.toml   # testes do core
```

### Empacotamento

```bash
scripts/package-linux.sh            # .deb + AppImage (release)
scripts/package-linux.sh deb        # apenas .deb (não baixa AppImage tools)
```

Artefatos em `src-tauri/target/release/bundle/{deb,appimage}/`.

### Estrutura do repositório

```
GhostWhisper/
├── README.md
├── prds/                     # especificação (spec-driven)
│   ├── business-rules.md
│   ├── memory.md
│   └── mvp/
│       ├── prd.md
│       ├── architecture/
│       └── tasks/
├── src/                      # frontend React (overlay, settings, sessão)
├── src-tauri/                # core Rust (áudio, ASR, LLM, DB, overlay)
└── assets/                   # modelo Silero VAD, ícones
```

## Configuração

A configuração é feita na tela **Settings** do app e persistida localmente:

- **Provider de LLM:** OpenAI, Gemini, OpenRouter ou endpoint custom (`base_url`).
- **API key:** guardada no keyring do sistema (Secret Service), nunca em texto puro.
- **Transcrição:** modelo whisper.cpp local (tiny/base/small/medium) ou cloud.
  O modelo escolhido é **baixado só na primeira sessão** que o usa (nada é
  baixado de antemão).
- **Idiomas:** origem e destino em **Auto / Português / English**. Fixar a origem
  melhora o reconhecimento; **Auto** deixa o Whisper detectar. Destino **Auto**
  (ou igual à origem) **não traduz**.
- **Áudio:** microfone e/ou monitor de saída do sistema (sink).
- **Atalhos globais** para mostrar/ocultar o overlay.

Nada é enviado para servidores do GhostWhisper — não existe backend próprio.

### Atalhos globais (padrões)

| Ação | Atalho |
|------|--------|
| Mostrar/ocultar overlay | `Ctrl+Shift+O` |
| Iniciar/encerrar sessão | `Ctrl+Shift+S` |
| Alternar stealth | `Ctrl+Shift+H` |
| Pedir sugestão | `Ctrl+Shift+I` |

Remapeáveis na tela de configurações; conflitos são reportados sem derrubar o app.

### Overlay

- **Arraste:** use o "⋮⋮" na barra para mover o overlay; a posição é lembrada.
- **Bandeja:** ícone sempre visível (Mostrar/ocultar, Sessão, Configurações, Sair).
  No GNOME é necessária a extensão **AppIndicator**.
- **Captura dupla:** inicie em **"Ambos"** para transcrever microfone (`Você`) e
  áudio do sistema (`Interlocutor`) ao mesmo tempo, cada um com seu próprio
  motor Whisper (contextos independentes).
- **Click-through:** regiões interativas capturam o mouse; o resto atravessa
  (input-shape por região no X11).
- **Teste de áudio:** na tela de configurações, teste microfone/áudio do sistema
  e veja o nível em tempo real antes de iniciar uma sessão.
- **Pausar / Retomar:** "Pausar" suspende a captura **sem encerrar a sessão** — as
  mensagens ficam na tela e o relógio continua de onde parou. "Nova sessão" encerra
  e começa outra; "Parar" encerra (para voltar numa sessão encerrada, use
  *Histórico → Continuar sessão*).
- **Trocar de canal:** o seletor (Ambos / Microfone / Sistema) funciona **sempre**,
  inclusive com a sessão gravando ou pausada — a troca não encerra a sessão nem
  perde a linha temporal.
- **Ouvir minha voz:** no teste de áudio (configurações), o toggle reproduz o
  microfone com ~0,4 s de atraso — use **fone de ouvido** (com caixa de som o
  retorno realimenta o microfone). Você ouve exatamente o que o ASR recebe.
- **Linha temporal:** cada fala (e cada sugestão da IA) mostra o tempo desde o
  início da sessão (`mm:ss`).
- **Histórico:** na aba *Histórico* do painel expandido ficam as sessões
  anteriores (com título, data e duração). Dá para **continuar** uma sessão,
  renomear, excluir, abrir a linha temporal completa (falas + sugestões) e
  **exportar em Markdown** — pelo diálogo nativo "salvar como" ou copiando para a
  área de transferência.

## Privacidade

- Sem login, sem telemetria, sem conta.
- Áudio e transcrições ficam no seu disco (SQLite).
- Chamadas de LLM vão direto do seu computador ao provider que você configurou.

> **Limitação conhecida (Linux):** a exclusão de janela do compartilhamento de tela
> não é garantida pelo protocolo gráfico (BR-006). O **modo stealth** sai da barra
> de tarefas e do alt-tab, mas **não esconde o overlay de uma captura de tela**: não
> existe API para isso no X11/Wayland. Em reuniões, compartilhe **uma janela** em vez
> da tela inteira, ou deixe o overlay em outro monitor. Veja a análise e a decisão em
> [`prds/mvp/architecture/wayland-spike.md`](prds/mvp/architecture/wayland-spike.md).

## Licença

A definir (candidato: MIT/Apache-2.0). GhostWhisper é um projeto independente e não é
afiliado ao Perssua.
