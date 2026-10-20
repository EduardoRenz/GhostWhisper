# 14 — Redesign da aba "Fontes de IA"

- **Status:** PENDING
- **Depende de:** 13
- **Habilita:** —
- **Issues:** — · **BR:** BR-044, BR-045, BR-047, BR-049

## Objetivo

Aproximar a aba **"Fontes de IA"** do mockup do usuário (2026-10-20), reusando
o padrão visual já aplicado em "Geral & HUD". É uma entrega de **frontend e
CSS**: não altera o comportamento de fontes, chaves, saúde, fallback,
reordenação ou salvamento.

Detalhe de produto: [`../prd.md`](../prd.md) §11. Arquitetura de UI (DOM,
classes e estados): [`../architecture/settings-redesign.md`](../architecture/settings-redesign.md) §10.

## Entregáveis

- `src/features/settings/AiSourcesSettings.tsx`
  - cabeçalho de seção fora do card, com ícone, subtítulo curto e nota
    `N fontes cadastradas · Keyring Seguro Ativo`;
  - lista de `ProviderCard` sem o card externo;
  - `AddProviderForm` recebendo a chave opcional e gravando via `setSourceKey`.
- `src/features/settings/ProviderCard.tsx`
  - cabeçalho com tile do provedor, nome, pill do modelo, pills de
    primário/fallback e status de saúde;
  - descrição curta derivada do papel (primária, fallback, reserva);
  - rodapé com chave mascarada + origem no keyring e ações compactas
    ("Validar chave", "Trocar chave", "Remover chave");
  - "Tornar primário" no cabeçalho; "Editar" e "Excluir fonte" como ações
    secundárias;
  - preservar edição inline, troca de chave, drag & drop e todos os callbacks.
- `src/features/settings/AddProviderForm.tsx`
  - card com cabeçalho (ícone + título + nota do keyring);
  - grid de 2 colunas (provedor, modelo) e campo de chave opcional;
  - helper do keyring, rodapé "Cancelar"/"Adicionar" e limpeza do formulário.
- `src/components/icons.tsx`
  - `KeyIcon`, `LockIcon`, `SyncIcon`, `PencilIcon`, `TrashIcon`,
    `PlusCircleIcon`, `ServerIcon`.
- `src/styles/globals.css`
  - classes `.settings__ai`, `.settings__section-heading`,
    `.settings__section-icon`, `.settings__section-caption`;
  - `.settings__source-head`, `.settings__source-tile`,
    `.settings__source-title-row`, `.settings__source-desc`,
    `.settings__source-meta`, `.settings__source-status` (+ variantes),
    `.settings__source-footer`, `.settings__key`, `.settings__key-value`,
    `.settings__key-store`, `.settings__source-actions`;
  - modificadores `.settings__button--ghost`, `.settings__button--compact`;
  - `.settings__add-grid`, `.settings__add-key`, `.settings__helper`,
    `.settings__add-footer`.
- Testes:
  - `src/features/settings/ProviderCard.test.tsx` (novo);
  - atualização de `src/features/settings/AiSourcesSettings.test.tsx`;
  - `src/components/icons.test.tsx` com os ícones novos;
  - ajuste mínimo de `SettingsPage.test.tsx`, se necessário (sem novas abas).

## Critérios de aceite

- [ ] A aba tem o cabeçalho "Provedores de IA Ativos" com contagem e estado do
      keyring, e os cards ficam diretamente na lista.
- [ ] O card mostra tile do provedor, nome, pill do modelo, "Primário",
      "Fallback ativo" e o status de saúde com latência.
- [ ] A chave mascarada e a origem no keyring aparecem no rodapé do card, com
      as ações "Validar chave", "Trocar chave" e "Remover chave".
- [ ] "Tornar primário", reordenação, edição, exclusão e troca/remoção de chave
      continuam funcionando.
- [ ] O formulário de adição tem grid de 2 colunas, campo de chave opcional,
      helper do keyring e "Cancelar" limpando o formulário.
- [ ] A chave digitada no formulário é gravada no keyring ao adicionar a fonte,
      sem salvar as settings.
- [ ] A tela mantém as 5 abas atuais; nenhum card ou estado fora do escopo
      desta entrega foi adicionado.
- [ ] Nenhuma mudança em Rust, IPC, `Settings` ou no contrato de fallback.
- [ ] `pnpm test`, `tsc --noEmit` e `pnpm build` verdes.
- [ ] Conferência visual em 1020×720 e 760×560, sem rolagem horizontal.

## Como testar

1. Abrir Configurações → "Fontes de IA".
2. Comparar com o mockup: cabeçalho de seção, card primário, card de fallback,
   rodapé com chave/ações e formulário de adição.
3. Reordenar, promover, validar, trocar e remover chaves.
4. Adicionar uma fonte com e sem chave; conferir o keyring e o rodapé global.
5. Reduzir a janela para 760×560 e conferir o empilhamento das ações.

## Verificação

Pendente.
