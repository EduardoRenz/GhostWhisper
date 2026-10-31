# PRD — Gravação da sessão (janela/tela + áudio em MP4)

- **Versão:** 0.1
- **Data:** 2026-10-21
- **Status:** **v1 encerrada em 2026-10-24** por decisão do usuário (tasks 03 e
  10 canceladas com escopo parcial; o restante está em ISS-049/052/053/055)
- **Relacionados:** [`../business-rules.md`](../business-rules.md),
  [`memory.md`](memory.md), [`issues.md`](issues.md),
  [`architecture/recording.md`](architecture/recording.md),
  [`tasks/_tasks.md`](tasks/_tasks.md) · PRDs anteriores:
  [`../mvp/prd.md`](../mvp/prd.md), [`../session/prd.md`](../session/prd.md),
  [`../notch/prd.md`](../notch/prd.md),
  [`../chat/prd.md`](../chat/prd.md),
  [`../session/architecture/session-journal.md`](../session/architecture/session-journal.md)

---

## 1. Objetivo

Gravar **a janela da reunião (ou a tela) e todo o áudio da sessão** num único
arquivo compacto, para que o usuário possa **rever a reunião depois** — ler o
que passou na tela, ouvir os dois lados e pular direto ao trecho que interessa.

A gravação é **propriedade da sessão** (BR-065): nasce com ela, pausa e retoma
quantas vezes o usuário quiser, e termina junto com ela. Não existe gravação
avulsa, em background ou sem sessão.

Em uma frase: *a sessão que já transcreve e sugere passa também a **lembrar da
tela** — e o fim da sessão entrega um arquivo, não um arquivo perdido.*

## 2. Problema

- **A transcrição não mostra o que estava na tela.** Metade do contexto de uma
  reunião é visual: slide, gráfico, erro, trecho de código. Hoje o GhostWhisper
  guarda texto e sugestões, mas não a imagem — e o `ScreenCapture` existente
  (`ports/screen_capture.rs`) é um **snapshot JPEG ≤1280 px** para o LLM, não
  uma fonte de vídeo.
- **Não há como revisar a reunião depois.** O Histórico lista falas e gera
  Markdown; quem faltou à reunião (ou quem quer reconferir um número) não tem
  onde olhar.
- **Gravar "na mão" é caro e frágil.** Ferramentas externas (OBS, `ffmpeg` na
  mão) exigem configuração, competem com o Whisper e produzem arquivos grandes;
  um crash no meio deixa um MP4 ilegível.
- **O áudio do app não serve para gravar.** O pipeline entrega **16 kHz mono**
  depois de AEC, ganho 8× e noise gate (`adapters/audio/microphone_front_end.rs`)
  — o que é ótimo para ASR e ruim para assistir. A gravação precisa da fonte
  **crua**, em 48 kHz (BR-069).

## 3. Público-alvo e casos de uso

Mesmas personas do MVP ([`../mvp/prd.md`](../mvp/prd.md) §3). Casos de uso:

- **CU-01 — Rever o que passou na tela:** o usuário lembra que houve um slide
  com o cronograma e quer reler; abre o arquivo no ponto certo.
- **CU-02 — Ouvir os dois lados:** a gravação traz microfone **e** áudio do
  sistema mixados, sem depender de ter transcrito tudo.
- **CU-03 — Pausar o que não deve ser gravado:** durante um intervalo, o usuário
  pausa a gravação e retoma depois; o arquivo final tem só o que importa.
- **CU-04 — Fechar a sessão e levar o material:** ao parar, o modal mostra
  duração, tamanho e itens processados, e permite baixar o Markdown ou abrir a
  pasta da gravação.
- **CU-05 — Escolher o custo:** quem só quer ouvir grava **somente áudio**
  (FLAC); quem precisa ler a tela grava vídeo em 1080p a 8 fps.

## 4. Histórias de usuário e critérios de aceite

### US-01 — Botão de câmera no notch

Como usuário, quero **um botão discreto de câmera** ao lado de Sugestões para
começar, pausar e retomar a gravação da sessão.

**Critérios de aceite**

- [ ] **CA-01.1** O botão é o **último item** do grupo de abas do notch
      (`NotchTabs`), depois de **Sugestões**, separado por um divisor sutil —
      como no mockup.
- [ ] **CA-01.2** O botão **só existe com sessão ativa**. Sem sessão, o grupo de
      abas não é renderizado e não há gravação possível (BR-065).
- [ ] **CA-01.3** Estados visuais:
      - **desligada:** ícone de câmera em cor neutra, `aria-pressed="false"`;
      - **gravando:** fundo/aro **vermelho**, ícone de câmera com ponto `REC`
        pulsante, `aria-pressed="true"`;
      - **pausada:** vermelho **vazado** (sem pulso), ícone de *play* pequeno
        sobreposto, `aria-pressed="false"`;
      - **indisponível** (sem encoder/drive): desabilitado com *tooltip*
        explicando o motivo.
- [ ] **CA-01.4** Clique alterna **iniciar → pausar → retomar**. Pausar a
      gravação **não** pausa a sessão nem a transcrição; pausar a sessão
      **pausa** a gravação automaticamente (BR-065).
- [ ] **CA-01.5** O botão tem `data-interactive` (click-through do overlay
      continua funcionando fora dele) e nunca rouba o foco da janela.
- [ ] **CA-01.6** O botão fica desabilitado durante `busy` e enquanto a
      gravação está **finalizando**.
- [ ] **CA-01.7** *Tooltip* contextual: `Gravar esta sessão (1080p · 8 fps)`,
      `Pausar gravação`, `Retomar gravação`, `Gravação pausada com a sessão`.
- [ ] **CA-01.8** O estado da gravação é **do backend** (novo evento
      `recording_state`), não um `useState` local: fechar e reabrir o overlay
      mantém o vermelho.
- [ ] **CA-01.9** Atalho opcional **Ctrl+Shift+R** para iniciar/pausar/retomar,
      registrado no catálogo de atalhos e exibido em badge no *tooltip*.

### US-02 — Gravação amarrada à sessão, à prova de crash

Como usuário, quero que a gravação acompanhe a sessão e que um fechamento
inesperado não destrua o que já foi gravado.

**Critérios de aceite**

- [ ] **CA-02.1** A gravação só pode ser iniciada com sessão ativa; parar a
      sessão **finaliza** a gravação (concatena e publica o resumo).
- [ ] **CA-02.2** `recording_auto_start` (padrão **desligado**) inicia a
      gravação junto com a sessão. Ligado, toda sessão nova já grava.
- [ ] **CA-02.3** Cada pausa **fecha o segmento** e cada retomada **abre outro**.
      Segmentos são gravados em **Matroska** (`.mkv`), reproduzível mesmo
      truncado; nada de MP4 aberto durante a captura.
- [ ] **CA-02.4** No `stop`, os segmentos são concatenados com **`-c copy`**
      (sem re-encode) e o resultado é remuxado para **MP4** com `+faststart`.
- [ ] **CA-02.5** Se o app morrer no meio, os segmentos permanecem no disco e a
      sessão fica marcada como `interrupted` no boot; o próximo `stop`/abertura
      oferece **recuperar** (concatena o que existe).
- [ ] **CA-02.6** A gravação **não** pode derrubar a transcrição: falha do
      `ffmpeg`, disco cheio ou encoder indisponível viram estado `failed` com
      mensagem, e a sessão continua (RNF-08).
- [ ] **CA-02.7** As configurações de gravação são **congeladas no início da
      sessão**: mudar FPS/resolução no meio não afeta a sessão corrente (os
      segmentos precisam ser concatenáveis).
- [ ] **CA-02.8** Só existe **uma** gravação ativa por vez (a sessão é única).
- [ ] **CA-02.9** Espaço em disco: exige **≥ 2 GB** livres para iniciar; avisa
      com **< 1 GB**; encerra a gravação com `failed` em **< 250 MB**.
- [ ] **CA-02.10** Pausar a gravação **não** insere preto nem silêncio: o
      arquivo final é a soma dos trechos (corte seco entre eles).

### US-03 — Modal "Sessão Finalizada"

Como usuário, ao encerrar a sessão quero **um resumo** com o que foi produzido e
os caminhos para levar o material.

**Critérios de aceite**

- [ ] **CA-03.1** Ao **parar a sessão pelo usuário**, abre a janela
      `session-end` (≈560×400, sempre no topo, sem moldura, centralizada), com
      o casco de header padrão (BR-064): selo de ícone, **"Sessão Finalizada"** e
      fechar no canto direito.
- [ ] **CA-03.2** Cartões de resumo, no layout do mockup:
      - **Duração total** — `14 min 32 seg` (tempo da sessão);
      - **Mídia** — com gravação: `214 MB (MP4 · 1080p · 8 fps)`; modo somente
        áudio: `28.4 MB (FLAC)`; sem gravação: `Sem gravação`.
- [ ] **CA-03.3** Linha **Itens processados**: `18 falas · 4 sugestões IA`,
      contados da sessão.
- [ ] **CA-03.4** Ações:
      - **Abrir local** (secundário) — revela o arquivo no gerenciador; só
        aparece quando existe arquivo;
      - **Baixar Sessão** (primário, ciano) — mesmo fluxo de export Markdown do
        Histórico (`saveSessionMarkdown`), com nome sugerido
        `título — AAAA-MM-DD.md`.
- [ ] **CA-03.5** Rodapé: **Descartar gravação** (apaga o arquivo, com
      confirmação em dois toques; a sessão e a transcrição **permanecem**) e
      **Nova Sessão** (inicia outra sessão com os mesmos canais).
- [ ] **CA-03.6** **Progresso de finalização**: enquanto o concat/remux roda
      (> 800 ms), o cartão de mídia vira uma barra `Finalizando gravação… 62%`;
      as ações ficam desabilitadas até `ready`. Sem gravação, o modal não mostra
      progresso.
- [ ] **CA-03.7** Se a finalização falhar, o cartão mostra o erro e mantém
      **Abrir local** apontando para os segmentos (nada é apagado).
- [ ] **CA-03.8** O modal **não** aparece em: exclusão de sessão, encerramento
      de sessão órfã no boot, ou `stop` interno do serviço — só na ação
      explícita do usuário.
- [ ] **CA-03.9** `Esc` e o **X** fecham o modal sem descartar nada.
- [ ] **CA-03.10** Com a sessão finalizada, o overlay volta ao estado de repouso
      (sem abas, sem botão de câmera).

### US-04 — Configurações de gravação

Como usuário, quero **escolher o custo e a qualidade** da gravação, com padrões
que já funcionam bem na minha máquina.

**Critérios de aceite**

- [ ] **CA-04.1** Nova aba **"Gravação"** em Configurações, entre **Transcrição
      & Áudio** e **Assistente** (a mais próxima do assunto: é mídia capturada,
      não IA).
- [ ] **CA-04.2** Cards:
      - **O que gravar** — `Vídeo + áudio` (padrão) ou `Somente áudio`;
        `Gravar ao iniciar a sessão` (padrão desligado);
        `Alvo padrão`: **Janela ativa** (padrão) ou `Tela inteira`.
      - **Vídeo** (oculto no modo áudio) — resolução, FPS, qualidade, encoder.
      - **Áudio** — bitrate AAC (modo vídeo) ou aviso `FLAC sem perda · mono
        48 kHz` (modo áudio).
      - **Arquivo** — container final (`MP4`/`MKV`; MKV pula o remux), pasta de
        destino, `Manter os segmentos`.
      - **Custo estimado** — `≈ 40–200 MB por hora` (atualiza com as opções) e
        `Encoder: VAAPI (hardware)` ou `x264 (software)`.
- [ ] **CA-04.3** Padrões (calibrados na máquina de referência — i5-6400,
      1920×1080, BR-066):

      | Opção | Padrão |
      |---|---|
      | Modo | Vídeo + áudio |
      | Alvo padrão | Janela ativa |
      | Resolução | Nativa da tela |
      | FPS | **8** |
      | Qualidade | Equilibrada (CRF 27) |
      | Preset | `veryfast` |
      | Encoder | Automático (VAAPI se houver) |
      | Áudio | AAC **96 kbps**, 48 kHz |
      | Container | MP4 |
      | Pasta | `$XDG_VIDEOS_DIR/GhostWhisper` (fallback `~/Vídeos/GhostWhisper`) |
      | Gravar ao iniciar | Desligado |
      | Manter segmentos | Desligado |

- [ ] **CA-04.4** Faixas válidas: alvo `janela|tela`; FPS `5|8|10|15|30`; resolução `nativa`,
      `1280×720`, `854×480`; qualidade `compacta` (CRF 30), `equilibrada`
      (27), `nítida` (24); áudio `64|96|128|160` kbps; container `mp4|mkv`.
- [ ] **CA-04.5** Valores inválidos ou fora da faixa são **normalizados** no
      backend (como `clamp_mic_gain`), nunca quebram a desserialização de
      configurações antigas (`#[serde(default)]`).
- [ ] **CA-04.6** O seletor de pasta usa o `tauri-plugin-dialog` e recusa
      caminho sem permissão de escrita.
- [ ] **CA-04.7** Se o encoder escolhido não estiver disponível, o card mostra
      o motivo e o **Salvar** mantém a escolha, mas a gravação cai para
      `x264` com aviso (RNF-09).
- [ ] **CA-04.8** Alterar configurações **não** afeta uma sessão em andamento
      (CA-02.7); a UI mostra "aplica-se à próxima sessão".

### US-05 — Consultar depois

Como usuário, quero achar e abrir a gravação depois sem procurar no
gerenciador de arquivos.

**Critérios de aceite**

- [ ] **CA-05.1** O **Histórico** mostra um selo `Vídeo`/`Áudio` nas sessões que
      têm gravação e uma ação **Abrir gravação** no detalhe.
- [ ] **CA-05.2** A sessão guarda `recording_path` e `started_at_ms`; o player
      externo abre no início da gravação (não no início da sessão) quando há
      diferença.
- [ ] **CA-05.3** O export Markdown inclui, no cabeçalho, uma linha
      `Gravação: <caminho> (<tamanho>)` quando existir.
- [ ] **CA-05.4** Excluir a sessão no Histórico apaga a gravação junto
      (confirmação explícita já existente ganha o aviso "o vídeo também será
      apagado").

### US-06 — O GhostWhisper não aparece na gravação

Como usuário, quero **usar o app normalmente, com o overlay visível**, e que ele
**não apareça** no vídeo gravado.

**Critérios de aceite**

- [ ] **CA-06.1** O alvo padrão é a **janela da reunião** (`x11grab
      -window_id`), não a root window: o overlay e o resto do desktop ficam de
      fora (BR-070).
- [ ] **CA-06.2** O clique principal inicia na **janela ativa**
      (`_NET_ACTIVE_WINDOW`), sem perguntar. O clique secundário
      (botão direito / *chevron*) abre o **seletor de janelas** (lista com app +
      título) com a opção **"Tela inteira"**, e a escolha vale para a sessão.
- [ ] **CA-06.3** Se a janela ativa for o próprio GhostWhisper ou não houver
      janela, cai para **tela inteira** com aviso no *tooltip* do botão.
- [ ] **CA-06.4** O alvo pode ser **trocado no meio da gravação** (clique
      direito → seletor): o segmento atual fecha e outro abre no mesmo arquivo.
      Se as dimensões mudarem, a finalização reencoda com `scale`+`pad` para o
      tamanho do primeiro segmento (CA-06.6). A opção **"Seguir a janela
      ativa"** faz a troca automaticamente (1 s de espera; ignora janelas
      menores que 320×240).
- [ ] **CA-06.5** Janela **fechada ou minimizada** encerra a gravação como
      `interrupted`, preserva os segmentos e mantém "Abrir local" no modal.
- [ ] **CA-06.6** Janela **redimensionada** entre segmentos não perde a
      gravação: fecha o segmento e abre outro; na finalização, se as dimensões
      divergirem, reencoda com `scale`+`pad` mostrando `Otimizando gravação…`.
- [ ] **CA-06.7** "Tela inteira" é escolha explícita e **inclui** o overlay —
      a UI avisa isso no seletor.
- [ ] **CA-06.8** Em Wayland (nativo **ou** via XWayland), a gravação usa o
      portal **ScreenCast**: o usuário escolhe **uma janela ou um monitor** no
      diálogo do GNOME e a fonte fica fixa até o `stop` (sem follow/troca no
      meio). CA-06.1–CA-06.7 continuam valendo no X11; no portal, a UI esconde
      o alvo interno e o "seguir a janela ativa" (BR-007/ISS-051/ISS-089).
- [ ] **CA-06.9** Com vários monitores, o seletor lista **"Todos os monitores"**
      (a tela virtual) e **cada monitor** com nome e resolução; gravar um
      monitor específico usa o offset dele (`:0.0+X,Y`). O **print de tela**
      captura o **monitor sob o cursor**, com fallback para o primário.

## 5. Requisitos não funcionais

Medições feitas em **2026-10-21** na máquina de referência (i5-6400, 4 núcleos,
1920×1080, X11), com captura real; detalhes em
[`architecture/recording.md`](architecture/recording.md) §8.

- **RNF-01 — Custo de CPU (teto):** a gravação não pode passar de **25% de um
  núcleo** nos padrões (1080p · 8 fps · `veryfast`), o que equivale a **~6% da
  máquina de 4 núcleos**. Medido: 23,8% a 10 fps, ~19% a 8 fps.
  `-threads 2` é obrigatório; o encoder nunca pode ocupar os 4 núcleos.
- **RNF-02 — Impacto no ASR:** o throughput de transcrição não pode cair mais
  de **10%** com a gravação ligada nos padrões.
- **RNF-03 — Tamanho:** reunião típica (slides + fala) **≤ 250 MB/h** nos
  padrões. Medido: tela estática 5–15 MB/h; slides com cursor ~5 MB/h.
- **RNF-04 — Sincronia A/V:** desvio ≤ **150 ms** ao fim de 1 h.
- **RNF-05 — Latência de início:** primeiro frame em **≤ 1,5 s** após o clique.
- **RNF-06 — Durabilidade:** nenhum segmento é perdido em crash; o arquivo
  final é reproduzível ou recuperável (CA-02.5).
- **RNF-07 — Isolamento:** falha de gravação nunca interrompe captura, ASR,
  sugestões ou diário (CA-02.6).
- **RNF-08 — Privacidade:** o arquivo fica **no dispositivo** (BR-002/BR-005);
  nada é enviado; indicador vermelho visível enquanto grava; nenhum frame ou
  amostra vai para log.
- **RNF-09 — Portabilidade:** ffmpeg é resolvido em três degraus — sidecar
  empacotado, `ffmpeg` do `PATH`, variável `GHOSTWHISPER_FFMPEG`. Ausência vira
  erro explícito com instrução, não *panic* (BR-007).
- **RNF-10 — Testabilidade:** a porta tem dobra (`FakeScreenRecorder`); os
  *builders* de argumentos do ffmpeg são funções puras testadas sem processo
  (BR-013). Nenhum teste unitário executa `ffmpeg`.
- **RNF-11 — Documentação única:** `BR-065`…`BR-071`, `ISS-049`…`ISS-053`,
  `NN-task` (BR-016).

## 6. Não-objetivos

- **Wayland nativo.** A v1 é X11/XWayland, como o resto do app (BR-007); em
  Wayland puro o botão fica desabilitado com explicação (ISS-051).
- **Edição, corte ou anotação** do vídeo dentro do app.
- **Transmissão** (streaming) ou compartilhamento do arquivo.
- **Múltiplas faixas de áudio separadas** no MP4 (mic e sistema vão **mixados**;
  stems ficam para um PRD futuro — ver §9).
- **Upload/cloud**, telemetria ou backend (BR-002, BR-004).
- **Legendas queimadas** no vídeo a partir da transcrição.

## 7. Riscos

| Risco | Mitigação |
|---|---|
| `ffmpeg` não estar instalado na máquina do usuário | sidecar empacotado + resolução em degraus + erro explícito (RNF-09) |
| Licença: o build estático com `libx264` é **GPL** | decidir licença do projeto antes de empacotar; alternativa `openh264`/VAAPI (ISS-049) |
| Dois clientes no microfone (`cpal` para ASR + `ffmpeg` para gravar) | Pulse/PipeWire permite múltiplos leitores; em ALSA puro, degradar para áudio do sistema + aviso (ISS-050) |
| CPU de 4 núcleos com Whisper + encode | teto de 25% de um núcleo, `-threads 2`, `-preset veryfast`, opção "somente áudio" (RNF-01) |
| Arquivo grande demais | CRF + 8 fps + estimativa na UI + teto de 250 MB/h (RNF-03) |
| MP4 corrompido em crash | segmentos MKV + `-c copy` no fim (CA-02.3/CA-02.5) |
| Captura por janela perde popups/diálogos e não acompanha troca de janela | "Tela inteira" explícita no seletor + aviso no *tooltip* (ISS-052) |
| Janela redimensionada gera dimensões diferentes entre segmentos | fecha o segmento e reencoda com `scale`+`pad` na finalização (ISS-053) |
| Overlay do GhostWhisper aparece no vídeo | **resolvido na v1**: alvo por janela (BR-070/US-06); "Tela inteira" avisa que inclui o overlay |
| Usuário esquece de gravar | opção "gravar ao iniciar" + botão sempre visível com sessão ativa |

## 8. Métricas de sucesso

- Uma sessão de 1 h produz um MP4 **≤ 250 MB** reproduzível em qualquer player.
- Zero sessões com arquivo ilegível por crash (RNF-06).
- Nenhuma regressão perceptível de ASR com gravação ligada (RNF-02).
- O usuário consegue, em **≤ 3 cliques** a partir do fim da sessão, abrir o
  vídeo e o Markdown.

## 9. Decisões fechadas

| Tema | Decisão | Motivo |
|---|---|---|
| Onde fica o botão | Último item de `NotchTabs`, depois de Sugestões | Mockup do usuário; é ação da sessão, não do painel |
| Alvo da captura | **Janela da reunião** (padrão), **todos os monitores** ou **um monitor** | Único jeito em X11 de manter o overlay visível e fora do vídeo (BR-070) |
| Vínculo | Gravação **é** da sessão; não existe sem ela | BR-065; evita arquivo órfão sem dono |
| Pausa | Pausa só a gravação, não a sessão; pausar a sessão pausa a gravação | Pedido do usuário ("pausar e continuar sempre") |
| Aba de settings | **Nova** aba "Gravação" (6ª) | 5 abas já cobrem IA, áudio, agente, HUD e atalhos; gravação é mídia |
| Padrões | 1080p nativo · 8 fps · CRF 27 · veryfast · AAC 96k | Medido na máquina do usuário (RNF-01/03) |
| Container durante a captura | MKV por segmento; MP4 no fim | Crash-safe; MP4 é o que o usuário compartilha |
| Encoder | Automático (VAAPI → x264) | Hardware reduz CPU a ~2–3%; software é o fallback universal |
| Áudio | Mic + sistema **mixados** numa faixa | "Todos os áudios" audíveis em qualquer player; MP4 com 2 faixas não toca as duas juntas |
| Modo áudio | FLAC mono 48 kHz (~117 MB/h) | Mockup ("28.4 MB · FLAC"); opção barata para quem só quer ouvir |
| Modal | Janela própria `session-end` | Precisa aparecer sobre qualquer janela, como o `dwell`/`capture` |

## 10. Questões em aberto

- **Q-01 (ISS-049):** empacotar `ffmpeg` como sidecar (~80 MB, GPL por causa do
  `libx264`) ou depender do `ffmpeg` do sistema? A decisão depende da licença do
  projeto (hoje "a definir: MIT/Apache").
- **Q-02:** o mockup mostra "Tamanho do áudio (FLAC)" para toda sessão. Nesta
  PRD o cartão mostra a **mídia produzida** (vídeo, áudio ou "sem gravação").
  Gravar áudio em **toda** sessão (mesmo sem o botão ligado) é escopo novo —
  candidato a PRD futura.
- **Q-03:** recuperação de gravação interrompida (CA-02.5): automática no boot
  ou sob demanda no Histórico? A task 03 entrega sob demanda; automática fica
  como melhoria.
