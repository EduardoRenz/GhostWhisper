# Memory — PRD Gravação da sessão

Histórico cronológico deste PRD (entradas `## YYYY-MM-DD — Título`, com linha
`IDs:`; convenções em [`../../.agents/rules.md`](../../.agents/rules.md)).

---

## 2026-10-21 — PRD criada (gravar a sessão em vídeo)

- **IDs:** BR-065, BR-066, BR-067, BR-068, BR-069, BR-070, BR-071,
  ISS-049, ISS-050, ISS-051, ISS-052, ISS-053 ·
  **Refs:** [`prd.md`](prd.md),
  [`architecture/recording.md`](architecture/recording.md),
  [`tasks/_tasks.md`](tasks/_tasks.md)

### Pedido

O usuário pediu para avaliar a complexidade de **gravar a tela em MP4 com todos
os áudios** e, em seguida, fechou o escopo: o objetivo é **consultar a reunião
depois** (ler o que estava na tela), em **Full HD no máximo**, com **arquivo
compacto** e **sem perda de performance**. Mandou dois mockups: o **botão de
câmera** no notch (último, vermelho quando grava, com pausa/retomada) e o modal
**"Sessão Finalizada"**.

### Medições que fecharam os padrões (2026-10-21)

Máquina: **i5-6400, 4 núcleos, 1920×1080, X11**, ffmpeg 7.0.2, x264.

| Config | CPU (1 núcleo) | % da máquina |
|---|---|---|
| só captura 10 fps | 2,8% | 0,7% |
| 1080p 5 fps `ultrafast` CRF30 | 8,0% | 2,0% |
| 1080p 8 fps `veryfast` CRF27 | ~19% | ~4,8% |
| 1080p 10 fps `ultrafast` CRF30 | 15,7% | 3,9% |
| 1080p 10 fps `veryfast` CRF26 | 23,8% | 6,0% |
| 1080p 30 fps `ultrafast` CRF30 | 47,6% | 11,9% |
| 720p 10 fps (escala) | 10,7% | 2,7% |

Conclusões: **FPS é o custo** (~13 ms/frame a 1080p); **reduzir resolução não
economiza** e piora a leitura; `mpdecimate` não acompanha (6× mais lento que o
tempo real); `-window_id` funciona (1888×1006 a 8 fps, ~20% de um núcleo).
Tamanho: tela estática 5–15 MB/h; reunião típica estimada em 40–200 MB/h;
áudio AAC 96k = 43 MB/h.

### Decisões fechadas com o usuário

| Tema | Decisão |
|---|---|
| Vínculo | Gravação **é** da sessão; não existe sem ela (BR-065) |
| Botão | Último item de `NotchTabs`, depois de Sugestões; vermelho quando grava |
| Pausa | Pausa a gravação sem parar a sessão; pausar a sessão pausa a gravação |
| Fim da sessão | Modal próprio com duração, mídia, itens, abrir local, baixar, descartar, nova sessão |
| Settings | **Nova** aba "Gravação" (6ª), com defaults da máquina de referência |
| Objetivo da qualidade | Ler a tela, não cinema: nativo · 8 fps · CRF 27 (BR-066) |
| Crash | Segmentos MKV + remux `-c copy` no fim, com progresso (BR-067) |
| Áudio | Mic + sistema **mixados**, cru (sem AEC/ganho/gate do ASR) (BR-069) |
| Alvo | **Janela da reunião** por padrão; o overlay fica visível e fora do vídeo (BR-070) |
| Privacidade | Local-only, indicador vermelho, nada em log (BR-071) |

### Por que "janela" e não "tela inteira"

O Linux **não** tem API de exclusão de janela da captura (BR-006; diferente de
`WDA_EXCLUDEFROMCAPTURE`/`NSWindowSharingNone`). Capturar a **janela** é o único
caminho em X11 que mantém o overlay visível **e** fora do arquivo. O usuário
pediu exatamente isso: usar o app normalmente, sem ele aparecer na gravação.
Trade-off registrado em ISS-052 (popups e troca de janela ficam de fora) e
ISS-053 (resize).

### Alternativas descartadas

- **Esconder o overlay durante a gravação:** o usuário pediu o contrário.
- **Compositor próprio (XComposite manual):** reimplementar o compositor para
  excluir uma janela é frágil e caro.
- **Output virtual dedicado:** só em wlroots; GNOME/KDE não expõem
  (`wayland-spike.md` §2).
- **KDE Plasma 6.6+ "Hide from Screencast":** único caminho real de exclusão por
  compositor, mas é Wayland + portal, fora da v1 (BR-007).
- **`mpdecimate` para dedup:** não acompanha em tempo real a 1080p.

### Pendências

- **ISS-049** bloqueia o empacotamento: sidecar GPL vs dependência de sistema vs
  build LGPL com VAAPI (Q-01).
- **ISS-050**: segundo cliente no microfone em ALSA puro.
- **ISS-051**: Wayland nativo sem gravação.
- **ISS-053**: resize gera dimensões diferentes entre segmentos (reencode no
  fim).
