# Issues — PRD Assistentes

Registry de bugs e problemas deste PRD. Convenções em
[`../../.agents/rules.md`](../../.agents/rules.md) (`ISS-NNN` é canônico por
arquivo de PRD; a numeração **continua** a dos PRDs anteriores para o ID ser
único no repositório). Próximo ID livre: **ISS-062**.

| ID | Resumo | Severidade | Status |
|---|---|---|---|
| ISS-060 | Escolha do agente de fábrica zerada na primeira carga após a atualização | Média | FIXED |
| ISS-061 | Notch escondido atrás de barra de rolagem quando o conteúdo cresce | Alta | FIXED |
| ISS-062 | Resposta rápida some de forma intermitente com modelos rápidos/streaming | Média | FIXED |

---

## ISS-060 — Escolha do agente de fábrica zerada na primeira carga após a atualização

- **Severidade:** Média · **Status:** FIXED
- **Sintoma:** um usuário com a "Reunião" (ou outro agente de fábrica)
  selecionado voltaria ao agente **Padrão** na primeira abertura depois da
  atualização, sem ter pedido — a escolha era perdida uma vez, apesar de o
  agente continuar existindo.
- **Causa raiz:** `Settings::sanitized()` roda na **carga**, antes da
  semeadura (`assistant_modes_seeded`), e `selected_mode_exists()` passou a
  reconhecer apenas o agente protegido e a lista do usuário. Como a lista
  ainda estava vazia na primeira carga, a seleção caía para `default` **antes**
  de o boot materializar os agentes de fábrica.
- **Correção:** `selected_mode_exists()` aceita um id de fábrica enquanto
  `!assistant_modes_seeded` (a semeadura ainda vai acontecer); depois da
  semeadura, a lista é a fonte da verdade e um agente excluído não volta.
- **Testes (R16):**
  `domain::settings::tests::sanitized_keeps_a_factory_agent_selected_before_the_seed`
  (falharia no código anterior) e
  `domain::settings::tests::sanitized_drops_a_factory_agent_deleted_after_the_seed`.
- **Arquivos:** `src-tauri/src/domain/settings.rs`,
  `src-tauri/src/domain/assistant.rs`
- **Task:** [`02-task.md`](tasks/02-task.md)

---

<!--
Modelo:

## ISS-061 — Título curto

- **Severidade:** Alta/Média/Baixa · **Status:** OPEN/FIXED
- **Sintoma:** …
- **Causa raiz:** …
- **Correção:** …
- **Arquivos:** …
- **Task:** [`NN-task.md`](tasks/NN-task.md)
-->

## ISS-061 — Notch escondido atrás de barra de rolagem quando o conteúdo cresce

- **Severidade:** Alta · **Status:** FIXED
- **Sintoma:** ao iniciar a sessão (ou abrir o seletor), a barra do notch
  ficava **cortada e escondida**, com barra de rolagem na janela do overlay.
- **Causa raiz:** a medição da largura passou a observar a pilha
  (`.overlay__stack`, BR-080), e a pilha tinha `max-width: 100%`. Quando o
  conteúdo crescia além da largura atual da janela, o CSS clampava a pilha à
  janela; a medição devolvia a largura clampada e a janela nunca crescia —
  ciclo "barra cresce → clampa → não cresce → rolagem". Como `html/body` não
  travavam rolagem, a barra de rolagem aparecia e escondia o notch.
- **Correção:** a pilha voltou a ser `width: max-content` **sem `max-width`**
  (a janela acompanha o conteúdo de verdade); a classe `.overlay-window` (posta
  pelo `Overlay`) trava `overflow: hidden` em `html/body/#root` **só na janela
  do overlay** — nenhuma barra de rolagem pode esconder o notch.
- **Teste (R16):** `src/styles/overlay-layout.test.ts` — falharia no CSS
  anterior (max-width na pilha e ausência do lock de overflow).
- **Arquivos:** `src/styles/globals.css`, `src/features/overlay/Overlay.tsx`
- **Task:** [`11-task.md`](tasks/11-task.md)

---

## ISS-062 — Resposta rápida some com modelos rápidos/streaming

- **Severidade:** Média · **Status:** FIXED (2026-10-28)
- **Sintoma:** ao testar DeepSeek e Mercury, o bloco **Resposta rápida** (a
  versão resumida) às vezes não aparecia; a resposta completa saía normal. O
  comportamento variava entre modelos e entre gerações.
- **Causa raiz:** não era contrato do provider. O rascunho *fast-first* é uma
  chamada **não-streaming**; a completa usa streaming. A flag `full_emitted`
  marcava o primeiro delta da completa e **descartava** o rascunho que
  terminasse depois. A UI reforçava o descarte com um `Set` de gerações que já
  tinham recebido a completa. Modelos mais rápidos/streaming chegavam primeiro
  e a rápida desaparecia; modelos mais lentos deixavam a rápida aparecer. Os
  logs não mostravam falha do rascunho, confirmando que ele era descartado, não
  rejeitado.
- **Correção:** a rápida é independente da completa. O core publica `quick`
  quando o passe rápido termina, mesmo depois da completa. A UI aceita a
  `quick` atrasada da **mesma** geração e ignora apenas a de uma geração já
  substituída (por `generation_id` + `at_ms`). A rápida continua efêmera e não
  limpa a completa.
- **Testes (R16/R23):**
  `application::suggestion_service::tests::a_late_quick_draft_still_fills_the_quick_block`,
  `core/application/useSuggestions.test.tsx` — “mostra a rápida que chega
  depois da completa da mesma geração” e “ignora a rápida de uma geração
  antiga depois de uma completa mais nova”.
- **Arquivos:** `src-tauri/src/application/suggestion_service.rs`,
  `src/core/application/useSuggestions.ts`
- **Task:** [`21-task.md`](tasks/21-task.md)
