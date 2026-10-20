# 08 — Aba "Fontes de IA": cards, ordem e chaves

- **Status:** CONCLUDED
- **Depende de:** 06, 07
- **Habilita:** 09

## Objetivo

Construir a aba **Fontes de IA**: a lista ordenada de fontes (a primeira é a
primária), com reordenação, promoção, chaves por fonte e o sinal de saúde.

## Entregáveis

- `src/features/settings/AiSourcesSettings.tsx`:
  - cabeçalho da seção ("Provedores de IA Ativos" + "N fontes cadastradas ·
    Keyring Seguro Ativo");
  - lista de `ProviderCard` na ordem do rascunho;
  - formulário "Adicionar Nova Fonte de IA" (`AddProviderForm`).
- `src/features/settings/ProviderCard.tsx`:
  - alça de reordenação (`DragHandle`), nome, provider, modelo;
  - badges: "Primário", "Conectada · Latência N ms", "Fallback Ativo",
    "Falhou" (+ motivo), "Desabilitada";
  - ações: "Tornar Primário", "Validar chave", "Trocar chave",
    "Remover chave", "Editar" (modelo/`base_url`), "Remover fonte";
  - a chave mascarada (`••••••••·abcd`) quando existe.
- `src/features/settings/AddProviderForm.tsx`:
  - `<select>` de provedor (com `DeepSeek`), `<select>` de modelo (catálogo +
    "Outro modelo…"), `base_url` quando `custom`;
  - "Adicionar" cria a entrada **no rascunho** (ainda não salva).
- `src/core/domain/settings.ts`: `PROVIDER_MODELS` (catálogo por provedor).
- `src/adapters/ipc/aiSources.ts`: cliente IPC.
- Assinatura do evento `llm_source_used` para atualizar os badges ao vivo.

## Critérios de aceite

- [x] A lista mostra as fontes na ordem do rascunho; a primeira tem o badge
      "Primário".
- [x] "Tornar Primário" move a fonte para o topo do rascunho.
- [x] Reordenar (drag) altera a ordem enviada no `saveSettings`.
- [x] "Validar chave" mostra a resposta do provider ou o erro.
- [x] "Trocar chave" grava no keyring (IPC) **sem** salvar as settings.
- [x] Adicionar fonte cria a entrada no rascunho.
- [x] `pnpm test` e `tsc --noEmit` limpos.
