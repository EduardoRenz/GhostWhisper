# Issues — PRD Assistentes

Registry de bugs e problemas deste PRD. Convenções em
[`../../.agents/rules.md`](../../.agents/rules.md) (`ISS-NNN` é canônico por
arquivo de PRD; a numeração **continua** a dos PRDs anteriores para o ID ser
único no repositório). Próximo ID livre: **ISS-062**.

| ID | Resumo | Severidade | Status |
|---|---|---|---|
| ISS-060 | Escolha do agente de fábrica zerada na primeira carga após a atualização | Média | FIXED |
| ISS-061 | Notch escondido atrás de barra de rolagem quando o conteúdo cresce | Alta | FIXED |

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
