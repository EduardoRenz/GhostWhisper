# 09 — Bloco de ativação em “Fontes de IA & Modelos”

- **Status:** CONCLUDED (2026-10-26)
- **Depende de:** 04, 05, 06, 08
- **Habilita:** 10, 11
- **Issues:** — · **BR:** BR-097

## Objetivo

Colocar o **Modo de ativação de sugestões** no topo da aba **“Fontes de IA &
Modelos”**, com dois cards selecionáveis (classificador vs. tempo), o modelo e a
sensibilidade no card do classificador, e o restante em um dialog “Avançado”.
Sem aba “Sugestões” no MVP.

## Entregáveis

- `src/features/settings/SuggestionsActivationBlock.tsx` (novo):
  - header com faísca, título “Modo de ativação de sugestões”, badge
    “Estratégia de trigger”, label “Disparo inteligente em tempo real” e toggle
    mestre (`suggestion_auto_refresh`);
  - dois cards `role="radio"`/`aria-pressed`; o selecionado ganha badge
    **“Ativa”** e borda de accent:
    - **Modelo de classificação** (Laya · Classificador contextual):
      descrição, **Modelo selecionado** (`Laya (Padrão)` | `Regras locais`),
      **status do modelo** (instalado/versão/tamanho, não
      instalado, baixando, corrompido, feature desligada), **Sensibilidade**
      (`Baixa` | `Balanceada` | `Alta`), link “Avançado ⚙”;
    - **Sugestão por tempo** (Intervalo periódico contínuo): descrição,
      **Cadência de ciclo** (`8s` | `15s` | `30s`), nota “Disparo cego · sem
      filtro semântico”. Se o valor salvo for outro (ex.: `20s`), renderizar um
      chip extra com o valor atual.
  - selecionar card grava `suggestion_mode`; off desabilita os cards sem perder
    a escolha.
- `SuggestionsActivationBlock` inclui o avançado **inline/colapsável** no MVP
  (cooldown, timeout, fallback, blocos, categorias, sonda, mostrar última
  classificação e teste). O drawer/modal pode virar evolução se crescer.
- O **model manager** (download/versões/checksum) fica na task 06; aqui o card
  mostra backend/modelo, status e caminho do ONNX de dev.
- `src/features/settings/SettingsPage.tsx`:
  - renderizar `SuggestionsActivationBlock` no topo de `tab === "sources"`;
  - renomear a aba para “Fontes de IA & Modelos” (l10n);
  - `?tab=sources` continua funcionando.
- `src/features/settings/AiSourcesSettings.tsx`:
  - receber o bloco antes da lista de fontes (ou manter no SettingsPage).
- `src/features/settings/CadenceSettings.tsx`:
  - remover o card “Sugestões automáticas” e o intervalo de sugestões;
  - manter “Resumo automático” e “Perguntas prontas”.
- `src/core/domain/settings.ts` + `src-tauri/src/domain/settings.rs`:
  - `suggestion_classifier_sensitivity` (`low|balanced|high`);
  - `suggestion_classifier_model_id`, `suggestion_classifier_model_version`,
    `suggestion_classifier_model_path`;
  - `suggestion_classifier_auto_download` (default `false`).
- l10n: chaves da §8.5/§9 do PRD em pt/en.

## Critérios de aceite

- [x] O bloco aparece no topo de “Fontes de IA & Modelos”, acima das fontes.
- [x] A aba foi renomeada para “Fontes de IA & Modelos”; `?tab=sources` abre
      nela.
- [x] Os dois cards refletem e gravam `suggestion_mode`; o toggle mestre
      liga/desliga sem perder o modo.
- [x] Card classificador mostra modelo, status e sensibilidade; card tempo
      mostra 8/15/30 e a nota “disparo cego”; valor fora dos presets vira chip
      extra, sem perder a configuração.
- [x] Sensibilidade mapeia 0,85/0,70/0,55 e o preset de cues; “Avançado” pode
      restringir categorias.
- [x] “Avançado” abre inline com todos os campos; salvar aplica os clamps.
- [x] O card mostra status/caminho do modelo; download/checksum/ativação ficam
      na task 06 (model manager), sem bloquear o modo heurístico.
- [x] “Testar” usa o rascunho e não gera sugestão.
- [x] O card de sugestões saiu de “Geral & HUD”; o de resumo ficou.
- [x] Nenhum texto hardcoded; pt/en com paridade.
- [x] Sem a feature `laya`, `Regras locais` continua funcionando; o backend
      Laya fica como seleção futura até a task 06.

## Testes

- `activation_block_renders_two_cards`
- `activation_block_selects_classifier_and_persists_mode`
- `activation_block_toggle_preserves_mode`
- `classifier_card_shows_model_status_and_sensitivity`
- `time_card_shows_8_15_30_and_blind_note`
- `sensitivity_maps_to_threshold_and_cue_preset`
- `advanced_dialog_opens_and_saves`
- `model_manager_downloads_verifies_and_activates`
- `model_manager_checks_catalog_updates`
- `model_manager_never_downloads_without_confirmation`
- `classifier_unavailable_without_feature_falls_back_to_heuristic`
- `suggestion_card_removed_from_cadence_settings`
- `sources_tab_renamed_and_url_still_works`
- `settings_copy_has_pt_en_parity`

## Como testar

```bash
pnpm test -- src/features/settings/SuggestionsActivationBlock
pnpm test -- src/features/settings/SuggestionsAdvancedDialog
pnpm test -- src/features/settings/ClassifierModelManagerDialog
pnpm test -- src/features/settings/CadenceSettings
pnpm test -- src/features/settings/SettingsPage
pnpm tsc --noEmit
pnpm build
```

## Verificação (2026-10-26)

- `SuggestionsActivationBlock` 6 testes; `CadenceSettings` 6 testes;
  `SettingsPage` 19 testes.
- `pnpm test` 504 passed; `tsc` limpo.
- O download/checksum/ativação do modelo não entrou aqui: é a task 06.

## Notas

- O mockup usa “Fontes de IA & Modelos”; a aba atual é “Fontes de IA”. A
  renomeação é só l10n (a chave `settings_tab_sources` continua).
- Não criar aba nova; se o “Avançado” crescer, promover a aba “Sugestões” é uma
  evolução, não um requisito do MVP.
- O model manager é a única forma de baixar o modelo para o usuário final; o
  caminho `model_path`/`LAYA_ONNX_DIR` fica para dev.
- O card de cadência de sugestões sai de “Geral & HUD” para não ter dois lugares
  controlando a mesma coisa.
