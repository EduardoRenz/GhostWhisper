# Business Rules — GhostWhisper

Registro de decisões e regras do produto. Cada regra tem um ID, a decisão, o
motivo e a data. Regras novas são adicionadas no fim; regras alteradas recebem
uma nota de revisão.

---

## BR-001 — Plataforma-alvo inicial: Linux
- **Decisão:** o MVP suporta Linux. macOS/Windows ficam fora do escopo do MVP.
- **Motivo:** o produto original é deficiente no Linux; é a dor que justifica o projeto.
- **Data:** 2026-10-17
- **Status:** Ativa

## BR-002 — Sem backend próprio (local-only)
- **Decisão:** o GhostWhisper não possui servidor. Não há login, conta, licença ou
  sincronização.
- **Motivo:** simplicidade, privacidade e alinhamento com open-source.
- **Consequência:** recursos de LLM dependem de chave do usuário (BYO key) ou de
  modelo local.
- **Data:** 2026-10-17
- **Status:** Ativa

## BR-003 — BYO key e segredos no keyring
- **Decisão:** chaves de API são fornecidas pelo usuário e armazenadas no keyring
  do sistema (Secret Service), nunca em arquivo de configuração em texto puro.
- **Motivo:** segurança; evitar vazamento de credenciais.
- **Data:** 2026-10-17
- **Status:** Ativa

## BR-004 — Sem telemetria por padrão
- **Decisão:** nenhuma telemetria, analytics ou crash report é enviado. Qualquer
  coleta futura deve ser opt-in explícito e documentada.
- **Data:** 2026-10-17
- **Status:** Ativa

## BR-005 — Áudio nunca sai do dispositivo sem ação do usuário
- **Decisão:** o áudio bruto é processado localmente. Só é enviado a um provider
  de nuvem o que o usuário configurar (ex.: transcrição cloud ou prompts de LLM).
- **Data:** 2026-10-17
- **Status:** Ativa

## BR-006 — Invisibilidade ao compartilhamento é "best-effort" no Linux
- **Decisão:** não prometemos paridade com `WDA_EXCLUDEFROMCAPTURE`/macOS. A
  mitigação primária é o overlay rodar em um **output virtual dedicado**; a
  documentação deve ser explícita sobre o limite.
- **Motivo:** não existe API de exclusão de janela na captura no Linux.
- **Data:** 2026-10-17
- **Status:** Ativa

## BR-007 — X11/XWayland primeiro, Wayland nativo depois
- **Decisão:** o overlay é entregue primeiro em X11/XWayland. Suporte Wayland nativo
  (via `gtk-layer-shell`) entra como spike/tarefa posterior.
- **Motivo:** janelas always-on-top/posicionamento preciso não existem no
  `xdg-shell`; reduzir risco do MVP.
- **Data:** 2026-10-17
- **Status:** Ativa

## BR-008 — Reconstrução de comportamento, não de código proprietário
- **Decisão:** o GhostWhisper se inspira em comportamento observado do Perssua
  (formato de dados, fluxos), mas não copia código-fonte, marca ou assets
  proprietários. O driver BlackHole (GPL-3) não é empacotado.
- **Motivo:** conformidade legal/licenciamento.
- **Data:** 2026-10-17
- **Status:** Ativa

## BR-009 — Modelos locais por padrão quando viável
- **Decisão:** ASR local (whisper.cpp) é o caminho padrão do MVP; cloud é opção.
- **Motivo:** privacidade e independência de provider.
- **Data:** 2026-10-17
- **Status:** Ativa

## BR-010 — Idioma e i18n
- **Decisão:** UI e documentação primárias em Português; tradução de conteúdo é
  função do produto (não confundir com idioma da interface).
- **Data:** 2026-10-17
- **Status:** Ativa

## BR-011 — Arquitetura hexagonal com inversão de dependência
- **Decisão:** o core segue `domain → application → ports → adapters`, com portas
  (traits) entre aplicação e adaptadores. `domain`/`application` nunca importam
  `adapters`. Composition root exclusivo em `src-tauri/src/lib.rs`.
- **Motivo:** testabilidade, independência de tecnologia e substituição de
  adaptadores (providers, áudio, DB) sem tocar no núcleo.
- **Detalhes:** [`.agents/rules.md`](../.agents/rules.md),
  [`engineering-guidelines.md`](mvp/architecture/engineering-guidelines.md).
- **Data:** 2026-10-17
- **Status:** Ativa

## BR-012 — Princípios SOLID
- **Decisão:** SOLID obrigatório; portas pequenas e específicas (I), extensão por
  novos adaptadores (O), contratos honrados por implementação (L).
- **Data:** 2026-10-17
- **Status:** Ativa

## BR-013 — Testabilidade como requisito
- **Decisão:** toda porta tem dobra de teste; `domain`/`application` sem I/O;
  cobertura alta no núcleo (> 80%). Ei de DI para permitir fakes.
- **Data:** 2026-10-17
- **Status:** Ativa

## BR-014 — Nomenclatura sem prefixo `I` (abstração antes da implementação)
- **Decisão:** abstrações sem prefixo (`TranscriptionsRepository`,
  `ChatBotProvider`); implementações prefixadas por contexto
  (`LocalTranscriptionsRepository`, `OpenAiChatBotProvider`,
  `WhisperSpeechToText`); dobras de teste `InMemory*`/`Fake*`.
- **Data:** 2026-10-17
- **Status:** Ativa

## BR-015 — Perssua como fonte de decisão de produto
- **Decisão:** ao decidir *como* resolver um problema, priorizar a solução já
  adotada pelo **Perssua** (produto maduro e validado). A transposição é apenas de
  tecnologia (Electron/JS/whisper.cpp → Tauri/Rust), preservando comportamento,
  nomes e valores observáveis (limiares, ordem das etapas). Desvios só com motivo
  técnico explícito e documentado (ex.: captura de áudio no Linux).
- **Motivo:** reduzir risco de reinventar heurísticas piores que as de um produto
  em produção; a referência é auditável via REA no `Perssua-*.AppImage`.
- **Detalhes:** [`.agents/rules.md`](../.agents/rules.md) §6.1 (R11–R13).
- **Data:** 2026-10-17
- **Status:** Ativa
