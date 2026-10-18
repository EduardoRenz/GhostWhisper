# 05 — Settings: agentes personalizados e cadências

- **Status:** CONCLUDED
- **Depende de:** 03, 04
- **Habilita:** 06

## Objetivo

Permitir criar e gerenciar agentes personalizados e ajustar o comportamento
automático das duas janelas.

## Entregáveis

- Aba **Assistente** em `SettingsPage`:
  - modo padrão (embutidos + personalizados);
  - resumo automático on/off + intervalo (15/30/60/120 s);
  - sugestões automáticas on/off + intervalo (10/20/30/60 s);
  - lista de agentes personalizados com criar/editar/duplicar/excluir.
- Editor: nome, categoria, prompt de sistema, prompt de sugestão, contexto fixo;
  contador/limite de 16.384 chars; validação e mensagens de erro.
- Duplicar embutido preenche os campos com os prompts do agente.
- `settings_set` reaplica validação; se o modo selecionado sumir, volta para
  Reunião.
- Testes de UI e de validação no domínio.

## Critérios de aceite

- [x] Criar um agente “Vendas” e selecioná-lo na janela de sugestões.
- [x] Editar/excluir reflete na lista e no seletor.
- [x] Agente inválido não salva e mostra erro.
- [x] Cadências e toggles persistem e valem no próximo open.
- [x] `pnpm test`/`tsc`/`build` e `cargo test` verdes.
