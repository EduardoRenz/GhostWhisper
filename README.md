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

> **Terminal do VS Code via snap:** o ambiente exporta `GTK_PATH`/`GIO_MODULE_DIR`/
> `LOCPATH`/`XDG_DATA_DIRS` apontando para `/snap/code` e `/snap/core20`, e o
> binário quebra ao carregar libs do snap
> (`undefined symbol: __libc_pthread_init, version GLIBC_PRIVATE`). Use
> `pnpm tauri:dev:clean` (ou rode de um terminal nativo, sem o sandbox do snap).

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
- **Conversa em bolhas:** as falas aparecem como uma conversa — `Você` à direita
  (azul) e `Interlocutor` à esquerda, **intercaladas pelo tempo da sessão**, com as
  sugestões da IA na mesma linha do tempo. O que ainda está sendo transcrito
  aparece no fim como bolha "em andamento" (com `…`) e, ao fechar o turno, entra na
  posição certa da conversa.
- **Controles:** pílula de ícones **play / pause / stop** (o ícone inativo fica
  esmaecido), a pílula de **tempo da sessão** e a bolinha de status
  (vermelha pulsando = gravando, âmbar = pausado). O ⏹ e a pílula de sessão pedem
  confirmação (**"Encerrar sessão? Sim / Não"**).
- **Pausar / Retomar:** "Pausar" suspende a captura **sem encerrar a sessão** — as
  falas ficam na tela e o relógio continua de onde parou. "Nova sessão" encerra
  e começa outra (na hora); o **⏹** encerra depois de confirmar (para voltar numa
  sessão encerrada, use *Histórico → Continuar sessão*).
- **Trocar de canal:** o seletor (Ambos / Microfone / Sistema) funciona **sempre**,
  inclusive com a sessão gravando ou pausada — a troca não encerra a sessão nem
  perde a linha temporal.
- **Ouvir minha voz:** no teste de áudio (configurações), o toggle reproduz o
  microfone com ~0,4 s de atraso — use **fone de ouvido** (com caixa de som o
  retorno realimenta o microfone). Você ouve exatamente o que o ASR recebe.
- **Linha temporal:** cada fala e cada sugestão da IA mostram o tempo desde o
  início da sessão (`mm:ss`) — inclusive no histórico e no export.
- **Histórico:** na aba *Histórico* do painel expandido ficam as sessões
  anteriores (com título, data e duração). Dá para **continuar** uma sessão,
  renomear, excluir, abrir a conversa completa (falas + sugestões em bolhas) e
  **exportar em Markdown** — pelo diálogo nativo "salvar como" ou copiando para a
  área de transferência.

### Resumo e sugestões (agentes)

Duas **janelas próprias** (movíveis e sempre no topo) complementam o overlay:

- **Resumo** (botão *Resumo* no painel/tray): resume a conversa em Markdown e se
  atualiza **a cada 30 s** enquanto a janela está aberta — sempre de forma
  **incremental**, sem chamar a IA quando não há fala nova. “Atualizar” regenera
  do zero e “Copiar” leva o Markdown. A janela guarda um **histórico de versões**
  (hora + título do tópico): clique numa anterior para ler **congelado** e use
  “Voltar ao atual” para retomar o ao vivo. O resumo também aparece no histórico e
  no export da sessão.
- **Sugestões** (botão *Sugestões* no painel/tray): olha os **últimos momentos**
  da conversa e sugere a próxima fala, no **agente** escolhido — **Reunião**,
  **Entrevista Técnica**, **LeetCode** e **System Design**, além dos seus
  **agentes personalizados**. A atualização automática (20 s, configurável) só
  chama a IA quando há fala nova; **Sugerir agora** força uma nova análise e
  **Fixar** grava a sugestão na conversa/export. A caixa **Contexto extra** aceita
  um enunciado ou trecho de código (útil no modo LeetCode, já que o app é
  áudio-only).
- **Código e Markdown:** as respostas são renderizadas com títulos, listas,
  tabelas e **código colorido** (com botão copiar); HTML bruto não é executado.
- **Janelas:** as duas são **sem moldura e arrastáveis** pelo cabeçalho. Redimensione
  pelas bordas/cantos (alças próprias, mínimo 320×240) e o **tamanho e a posição
  são lembrados** entre reinícios. Em **Wayland nativo** o redimensionamento por
  alça pode não funcionar (em X11/XWayland funciona); veja a limitação em
  [`prds/assistant/architecture/assistant-windows.md`](prds/assistant/architecture/assistant-windows.md).
- **Custo:** o automático só roda com a janela aberta e a sessão gravando; dá para
  pausar cada janela no cabeçalho e ajustar a cadência em *Configurações →
  Assistente*. Os agentes personalizados (nome, prompts e contexto) também ficam
  nessa aba.

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
