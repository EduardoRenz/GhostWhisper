# 11 — Hardening, docs, pacote e teste manual

- **Status:** IN PROGRESS (2026-10-26) — checks automatizados concluídos; roteiro manual no app pendente
- **Depende de:** 10
- **Habilita:** —
- **Issues:** — · **BR:** BR-092…BR-100

## Objetivo

Fechar o PRD: validar o fluxo real no Linux, garantir privacidade/desempenho,
atualizar a documentação única e empacotar. Nenhuma feature nova.

## Entregáveis

- Teste manual completo (roteiro §11.3 da arquitetura) em sessão real no Linux:
  - pergunta, hesitação, perda do fio, convite, silêncio, nada a sugerir;
  - alternância auto/classificador no topo de “Fontes de IA & Modelos”;
  - baixar/verificar/ativar/apagar modelo; modo offline;
  - erro do Laya e recuperação;
  - abrir/fechar a janela e hidratação da última classificação;
  - pausar/retomar.
- Ajustes de copy/l10n encontrados no teste (pt/en).
- Revisão de privacidade:
  - grep por `println!`/`tracing` com texto de transcrição;
  - confirmar que manifesto/checksum não contêm dados de sessão;
  - confirmar que `evidence` é curta e não vai para log;
  - confirmar que nenhum dado sai para download/telemetria.
- Performance:
  - medir `build_blocks` (alvo < 5 ms para 40 segmentos);
  - medir p50/p95 do heurístico e do Laya;
  - confirmar que a UI não trava durante “Analisando…”.
- Documentação:
  - `prds/business-rules.md`: conferir/ajustar BR-092…BR-100 já registradas
    (texto único).
  - `prds/suggestion-mode/prd.md`: status → Implementado; ajustes de escopo.
  - `prds/suggestion-mode/memory.md`: entradas por task e resultado da POC.
  - `prds/suggestion-mode/issues.md`: ISS-059+ encontrados.
  - `prds/suggestion-mode/tasks/_tasks.md`: status final.
  - `prds/suggestion-mode/architecture/suggestion-gating.md`: decisões finais
    (backend padrão, se o adaptador recria no boot, etc.).
- Pacote:
  - `pnpm build` + `cargo build --release` (ou `pnpm tauri build`);
  - gerar AppImage/deb se o usuário pedir;
  - registrar horário/versão no `memory.md`.

## Critérios de aceite

- [ ] Roteiro manual executado e registrado (data, sessão, resultado).
      **Pendente:** precisa de uma sessão real no Linux.
- [x] Nenhum texto de transcrição/áudio/token em log. `grep` de `tracing`
      não achou texto de bloco/segmento; `evidence` é truncada em 160 chars;
      logs do classificador só têm cue/confiança/bloco/latência.
- [x] Nenhuma sugestão duplicada no mesmo bloco (dedupe por `block_id` no
      gate + teste).
- [x] Nenhuma regressão no modo automático (branch separado + testes).
- [x] A faixa de última classificação mostra decisão positiva, negativa,
      erro e modo automático (testes da task 08).
- [x] O bloco em “Fontes de IA & Modelos” salva e recarrega os campos
      (settings tipadas + testes).
- [x] O model manager baixa/verifica/ativa/apaga e nunca deixa parcial
      (testes do adaptador + artefato local de dev).
- [x] O modo offline funciona com o heurístico; com o modelo, depende do
      artefato ONNX (ISS-059).
- [ ] A POC Laya tem resultado documentado; bloqueada no artefato (ISS-059).
      O padrão segue `automático` e o `heuristic` está disponível.
- [x] `cargo test`, `cargo clippy --all-targets -D warnings`, `cargo fmt
      --check`, `pnpm test`, `tsc --noEmit` e `pnpm build` verdes;
      `cargo build/check --features laya` compila.
- [x] Documentação atualizada (PRD, memória, issues, tasks, arquitetura,
      README/AGENTS).

## Verificação automatizada (2026-10-26)

- Rust: **814 passed**, 11 ignored (sem feature); **817 passed**, 15 ignored
  (`--features laya`); `cargo clippy --all-targets -D warnings` e
  `cargo fmt --check` limpos.
- Frontend: **518 passed**; `tsc --noEmit` e `pnpm build` limpos.
- `build_blocks` com 40 segmentos: **145 µs** (alvo < 5 ms).
- Privacidade: sem log de texto de transcrição; `evidence` ≤ 160 chars;
  manifesto/checksum do modelo não têm dados de sessão; sem telemetria.
- Pacote: `pnpm build` ok. AppImage com Vulkan + Laya + modelo gerado
  (`pnpm tauri:build:laya`, 821 MiB) em
  `src-tauri/target/release/bundle/appimage/`; o `.deb` de Laya não foi gerado
  (só AppImage). O AppImage default segue `heuristic`; Laya é POC (ISS-062).
- Bug do teste manual: `Classificador indisponível: configuração inválida:
  nenhum bloco classificável` era o gate usando época Unix contra timestamps
  relativos da sessão (ISS-060). Corrigido; reexecutar o roteiro manual.

## Roteiro manual (pendente)

1. `pnpm tauri dev` (ou `--features laya` com o artefato).
2. **Fontes de IA & Modelos** → escolher `Modelo de classificação` →
   `Regras locais` → salvar.
3. Abrir **Sugestões**, iniciar sessão e conversar:
   - pergunta de THEY → `question` + sugestão;
   - ME hesitando por ≥ 4 s → `hesitation` mesmo com ME falando;
   - conversa fluente → `no_suggestion`, sem chamada ao LLM;
   - silêncio e convite (se o backend Laya estiver ativo).
4. Trocar para `Sugestão por tempo` e confirmar o cronômetro/contagem.
5. Pausar/retomar, fechar/abrir a janela e conferir a hidratação da faixa.
6. Model manager: baixar (artefato de dev), verificar progresso, ativar,
   remover a versão inativa; confirmar que a ativa é protegida.
7. Trocar de sessão e confirmar que a faixa/cache resetam.

Registrar data, sessão e resultado no `memory.md`.

## Testes finais

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
pnpm test
pnpm tsc --noEmit
pnpm build
```

## Checklist de privacidade

- [ ] `grep -R "block.text\|transcript\|evidence" src-tauri/src | grep -i log`
      não encontra log de conteúdo.
- [ ] `evidence` limitada a 160 chars no adaptador.
- [ ] Nenhum dado de sessão em `manifest.json`/métricas.
- [ ] Download só com confirmação; checksum verificado.

## Notas

- Se a POC Laya não passar no gate, **não** remover o adaptador; deixá-lo
  opt-in e documentar o motivo.
- O modo automático é o fallback de produto; o modo classificador deve ser
  claramente opt-in.
- A próxima evolução (feedback “útil/não útil”, persistência de classificação)
  é um novo PRD. O caminho de fine-tuning já tem contrato na task 12.
