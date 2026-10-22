# 06 — Modal de agente (criar/editar)

- **Status:** CONCLUDED
- **Depende de:** 05
- **Habilita:** 09
- **Issues:** — · **BR:** BR-076, BR-077

## Objetivo

O **modal único** de agente: o mesmo formulário serve para **criar** e para
**editar**, com os campos do mockup — nome, ícone, cor, prompt de sugestão e
prompt de resumo — e um bloco "avançado" com o contrato de sistema, o contexto
fixo e a categoria (que já existem no modelo e não podem se perder).

Detalhe de produto: [`../prd.md`](../prd.md) §8.2/§8.3/§8.4. Desenho técnico:
[`../architecture/assistants-panel.md`](../architecture/assistants-panel.md) §2/§3.

## Entregáveis

- `src/features/settings/AgentEditorDialog.tsx` (novo)
  - `<Dialog>` com o formulário;
  - campos: **Nome** (≤ `MODE_NAME_MAX_CHARS`), **Ícone** (catálogo BR-037),
    **Cor** (tokens BR-037), **Prompt de Sugestão**, **Prompt de Resumo**;
  - bloco "avançado" (recolhível): **Categoria**, **Prompt de sistema**,
    **Contexto fixo do agente**;
  - validação no frontend: nome obrigatório e ao menos um prompt preenchido
    (mesma regra do core);
  - "Salvar" grava no rascunho (`update("custom_assistant_modes", next)`) e
    fecha; "Cancelar" fecha sem gravar;
  - modo criação (id novo `custom-<uuid>`), edição (mesmo id) e duplicação
    (pré-preenchido, nome com "(cópia)", id novo).
- `src/features/settings/AssistantsSettings.tsx`
  - liga "Editar", "Duplicar" e o CTA ao modal.
- `src/styles/globals.css`
  - layout do formulário dentro do modal (grade de 2 colunas para ícone/cor,
    textareas com `maxLength`, `.settings__field textarea` reaproveitado).

## Critérios de aceite

- [ ] "Editar" abre o modal com os valores do agente (nome, ícone, cor,
      prompts).
- [ ] "Criar Novo Assistente de IA" abre o **mesmo** modal, vazio.
- [ ] O modal recusa nome vazio e recusa agente sem nenhum prompt.
- [ ] Salvar com o prompt de resumo preenchido persiste o campo
      (`summary_prompt` no rascunho).
- [ ] Salvar **não** grava no disco: quem persiste é o "Salvar Alterações"
      global (BR-047).
- [ ] `Esc` e o botão ✕ fecham sem gravar.
- [ ] O limite de 16 384 caracteres vale nos textareas.

## Testes

- `src/features/settings/AgentEditorDialog.test.tsx` (novo): abrir com valores;
  criar vazio; recusa sem nome; recusa sem prompt; salva com `summary_prompt`;
  cancelar não grava.
- `pnpm test`, `tsc --noEmit`.
