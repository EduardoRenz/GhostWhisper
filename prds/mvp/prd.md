# PRD — GhostWhisper MVP

- **Versão:** 0.1
- **Data:** 2026-10-17
- **Status:** Implementado (tasks 00–23 `CONCLUDED`)
- **Autor:** Equipe GhostWhisper
- **Relacionados:** [`../business-rules.md`](../business-rules.md),
  [`memory.md`](memory.md), [`issues.md`](issues.md),
  [`architecture/architecture.md`](architecture/architecture.md),
  [`tasks/_tasks.md`](tasks/_tasks.md)

---

## 1. Objetivo

Entregar um copiloto de reunião **open-source, Linux-first e local-only** que:

1. captura o áudio da reunião (microfone e/ou áudio do sistema),
2. transcreve em tempo real localmente (whisper.cpp),
3. traduz ao vivo (opcional),
4. sugere respostas/insights a partir de prompts (BYO key de LLM),
5. exibe tudo numa **janela overlay discreta**, separada da reunião, que não
   rouba foco nem polui a apresentação,
6. persiste a sessão localmente (SQLite).

Sem conta, sem assinatura, sem telemetria.

## 2. Problema

Ferramentas de copiloto em tempo real são majoritariamente macOS/Windows. No Linux
sofrem de: captura de áudio do sistema instável, dependência de XWayland, problemas
de janela/permissão e recursos essenciais (ex.: ficar invisível no compartilhamento
de tela) atrás de paywall. O usuário Linux não tem uma alternativa confiável,
privada e aberta.

## 3. Público-alvo e casos de uso

### Personas
- **Dev/entrevistado:** precisa de ajuda durante entrevistas técnicas.
- **Vendas/CS:** precisa responder objeções em calls.
- **Apresentador/executivo:** quer apoio discreto durante reuniões.
- **Usuário privacidade-consciente:** quer IA local e sem nuvem.

### Casos de uso (CU)
- **CU-01 — Iniciar sessão de escuta:** usuário abre o overlay e inicia a captura
  de microfone.
- **CU-02 — Capturar áudio de uma call:** usuário seleciona o monitor de saída
  (ou sink virtual) para captar ambos os lados da conversa.
- **CU-03 — Ver transcrição ao vivo:** o texto aparece segmentado por falante
  (ME/THEY) enquanto fala.
- **CU-04 — Tradução ao vivo:** transcrição é traduzida para o idioma escolhido.
- **CU-05 — Pedir um insight:** usuário invoca um atalho/prompt e recebe sugestão
  contextualizada com base na transcrição (+ tela, opcional/futuro).
- **CU-06 — Discretude:** overlay permanece no topo, sem barra de tarefas e sem
  roubar foco.
- **CU-07 — Revisar histórico:** usuário consulta sessões e transcrições passadas.
- **CU-08 — Configurar provider:** usuário insere a chave de API do LLM escolhido.

## 4. Estórias de usuário (com critérios de aceite)

- **US-01** — Como usuário, quero **iniciar a escuta do microfone com um clique**
  para não depender de configuração complexa.
  - Aceite: botão inicia captura em < 1 s; indicador de nível de áudio visível;
    texto começa a aparecer em < 3 s de fala contínua.
- **US-02** — Como usuário, quero **capturar o áudio da reunião (saída do sistema)**
  para ouvir o outro lado.
  - Aceite: seleciono o dispositivo/monitor nas configurações; a transcrição
    inclui o áudio do sistema rotulado como "THEY".
- **US-03** — Como usuário, quero **transcrição local** para não depender de nuvem.
  - Aceite: funciona offline com modelo baixado; nenhum áudio sai da máquina.
- **US-04** — Como usuário, quero **sugestões de resposta** contextuais.
  - Aceite: ao acionar, recebo uma resposta coerente com a transcrição em < 5 s
    (provider cloud) e posso copiar.
- **US-05** — Como usuário, quero **traduzir a transcrição ao vivo**.
  - Aceite: escolho o idioma e vejo a tradução acompanhar a transcrição.
- **US-06** — Como usuário, quero que o overlay **não roube foco** ao clicar.
  - Aceite: clicar no overlay não tira o foco do app da reunião; áreas não
    interativas são click-through.
- **US-07** — Como usuário, quero **esconder o overlay** com um atalho global.
  - Aceite: atalho mostra/oculta; em "stealth", não aparece na barra de tarefas.
- **US-08** — Como usuário, quero **rever sessões anteriores**.
  - Aceite: lista de sessões com busca em texto (FTS5).

## 5. Escopo

### Dentro do MVP
- Captura de microfone.
- Captura de áudio do sistema (monitor de sink PipeWire/Pulse; helper de sink
  virtual quando necessário).
- VAD + ASR local (whisper.cpp) com streaming.
- Overlay X11/XWayland: transparente, always-on-top, click-through dinâmico,
  skip-taskbar.
- Tradução ao vivo e insights/sugestões via BYO key (1–2 providers + custom).
- Persistência SQLite (sessões, mensagens, transcrições, FTS5).
- Atalhos globais básicos.
- Empacotamento `.deb` e AppImage.

### Fora do MVP (explícito)
- macOS/Windows.
- MCP / connected tools, Codex OAuth, LLM local (llama.cpp), captura/OCR de tela,
  e-mail, Obsidian, multi-flavors/white-label, onboarding/login.
- Wayland **sem XWayland** (o overlay depende de XWayland; atalhos, captura e
  gravação já usam portais XDG — BR-007/ISS-088/ISS-089).
- Invisibilidade garantida no compartilhamento de tela (best-effort).

## 6. Requisitos

### Funcionais (RF)
- RF-01 Capturar microfone (seleção de dispositivo).
- RF-02 Capturar áudio do sistema (seleção de monitor/sink).
- RF-03 VAD para segmentar fala.
- RF-04 ASR local streaming com timestamps.
- RF-05 Atribuição de falante ME/THEY por fonte de áudio.
- RF-06 Tradução ao vivo configurável.
- RF-07 Insights/sugestões via provider de LLM configurável.
- RF-08 Persistir sessões/transcrições com busca FTS5.
- RF-09 Atalhos globais (mostrar/ocultar, iniciar/encerrar).
- RF-10 Tela de settings (providers, chaves, idiomas, áudio, atalhos).

### Não funcionais (RNF)
- RNF-01 Latência de transcrição < 3 s (fala → texto) em CPU moderna.
- RNF-02 Consumo de CPU/GPU configurável (modelo tiny→medium).
- RNF-03 Segredos no keyring (BR-003).
- RNF-04 Sem telemetria (BR-004).
- RNF-05 Overlay estável em X11/XWayland no GNOME e KDE.
- RNF-06 Binário instalável via `.deb` e AppImage, sem `sudo` no uso diário.
- RNF-07 Degradação graciosa quando não há monitor de sink disponível.

## 7. Tech Spec (resumo)

| Área | Decisão |
|------|---------|
| Shell | Tauri v2 (Rust) + React/Vite/TS |
| Mic | `cpal` |
| Sistema | `libpulse-binding` (monitor) / PipeWire; helper `pactl` p/ sink virtual |
| PCM | `rubato` (resample) + `hound`/WAV |
| VAD | Silero via `whisper-rs` (VAD embutido) |
| ASR | `whisper-rs` (whisper.cpp) local |
| LLM | `reqwest` (OpenAI/Gemini/OpenRouter/custom) |
| DB | `rusqlite` (`bundled`, FTS5) |
| Segredos | `keyring` (Secret Service) |
| Overlay | Janela Tauri X11/XWayland; atalhos/captura/gravação por portais XDG no Wayland |
| IPC | Tauri commands + events |

Detalhamento em [`architecture/architecture.md`](architecture/architecture.md).
O core segue arquitetura **hexagonal** (ports & adapters) com inversão/injeção de
dependência, SOLID, alta testabilidade e nomenclatura sem prefixo `I`
(BR-011..BR-014; ver [`architecture/engineering-guidelines.md`](architecture/engineering-guidelines.md)).

## 8. Métricas de sucesso (MVP)
- Sessão de mic → primeira transcrição < 3 s.
- Captura de áudio do sistema funcionando em ≥ 2 distros (ex.: Ubuntu/Fedora) com
  PipeWire.
- Overlay click-through sem roubar foco em GNOME e KDE (X11).
- Zero tráfego de rede não iniciado pelo usuário (verificável).

## 9. Riscos e mitigações

| ID | Risco | Mitigação |
|----|-------|-----------|
| R-01 | Invisibilidade ao share impossível no Linux | Output virtual dedicado; documentar limite (BR-006) |
| R-02 | Wayland sem always-on-top | Resolvido em 2026-10-30: overlay em **XWayland** + portais XDG para atalhos/captura/gravação (BR-007/ISS-088/ISS-089) |
| R-03 | Captura de sistema varia (PipeWire vs Pulse) | Detecção + fallback + helper de sink virtual |
| R-04 | Transparência sem compositor | Detectar compositor; degradar para fundo opaco |
| R-05 | ASR local pesado | Perfis de modelo; opção cloud |

## 10. Dependências
- Provedores de LLM externos (opcionais).
- Modelo Silero VAD (`ggml-silero-v5.1.2.bin`, MIT) e modelos whisper.cpp.
