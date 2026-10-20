# 05 — Perguntas prontas: settings, chips e editor

- **Status:** PENDING
- **Depende de:** 04
- **Habilita:** —
- **Issues:** — · **BR:** BR-054
- **Refs:** [`../prd.md`](../prd.md) US-04;
  [`../architecture/session-journal.md`](../architecture/session-journal.md) §5.3

## Objetivo

Permitir **salvar perguntas prontas**, **repeti-las com um clique** e
**editá-las** — o equivalente às "Prompt Suggestions" do Perssua.

## Entregáveis

- `src-tauri/src/domain/settings.rs`:
  - `SavedQuestion { id, text }`;
  - `Settings.saved_questions: Vec<SavedQuestion>` com `#[serde(default)]`
    (settings antigas continuam válidas);
  - sanitização no load: **no máximo 20** perguntas, **200 caracteres** cada,
    texto vazio removido, ids únicos.
- `src/features/journal/SavedQuestionsEditor.tsx`: listar, **remover**,
  **editar** e **resetar** para os defaults.
- `src/features/journal/AskComposer.tsx`:
  - **chips** com as perguntas salvas — clicar **preenche e envia**;
  - botão para **salvar a pergunta atual**;
  - botão para abrir o editor.
- Defaults semeados (tradução das do Perssua): *"Resumir tudo até agora"*,
  *"Listar as ações"*, *"Listar as principais ideias"*, *"O que posso falar?"*.
- `src/core/domain/settings.ts`: tipo `SavedQuestion` espelhado.

## Critérios de aceite

- [ ] Clicar num chip **envia** a pergunta e a entrada do diário fica com a
      origem **"Pergunta pronta"** (`request_source = preset`).
- [ ] Salvar a pergunta atual a adiciona aos chips (e à lista do editor).
- [ ] Remover uma pergunta some dos chips.
- [ ] **Limites**: a 21ª pergunta é recusada com aviso; o texto é truncado em
      200 caracteres **no máximo** (o excedente é recusado, não cortado em
      silêncio).
- [ ] Os quatro defaults aparecem na primeira execução.
- [ ] `pnpm test` e `cargo test --lib` verdes (sanitização no Rust,
      chips/editor no frontend).

## Notas

- Ficam nas **settings** (arquivo JSON), não no banco: são **configuração do
  usuário**, não dado de sessão (BR-054). Nenhuma porta nova — o IPC de
  settings já existe (`saveSettings`).
- O Perssua limita a **4 × 140** porque tem paywall; aqui o teto é
  **20 × 200** (desvio consciente, R12 — BR-002: sem login, sem plano).
- Sem geração automática de follow-ups pelo modelo (o Perssua tem): as
  perguntas prontas cobrem o caso na v1 (ver §9 do PRD).
