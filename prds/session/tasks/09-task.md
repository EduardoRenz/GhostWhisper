# 09 — Documentação, verificação e pacotes

- **Status:** PENDING
- **Depende de:** 04, 05, 07, 08, 10
- **Habilita:** —
- **Issues:** ISS-041, ISS-042 · **BR:** BR-014, BR-016, BR-017, BR-052…BR-058
- **Refs:** [`../prd.md`](../prd.md) §5, §8;
  [`../architecture/session-journal.md`](../architecture/session-journal.md)

## Objetivo

Fechar a PRD: registrar as **regras de negócio**, atualizar a **memória**,
verificar tudo (testes, lint, build) e gerar os **pacotes**.

## Entregáveis

- `prds/business-rules.md`: BR-052…BR-058 (já escritas na criação da PRD —
  conferir o texto final contra o que foi implementado e ajustar se
  preciso).
- `prds/session/memory.md`: entrada da data de hoje com `IDs:` e `Refs:`,
  registrado **por PRD** (BR-017, R15).
- `prds/session/issues.md`: ISS-041/ISS-042 atualizados com o **status**
  real (`FIXED`/`WONTFIX`) e o arquivo que corrigiu.
- `prds/session/tasks/_tasks.md`: status de cada task.
- `README.md`: menção da PRD nova na seção de specs (se houver lista).
- `AGENTS.md`/`.agents/rules.md`: **só** se alguma regra geral mudar (a
  lista de PRDs não é duplicada lá — R14).

## Verificação

```bash
pnpm test            # frontend
cargo test --lib     # núcleo Rust
cargo clippy --all-targets
pnpm tsc --noEmit    # se houver script equivalente
pnpm tauri build     # .deb + AppImage
```

## Critérios de aceite

- [ ] `pnpm test` e `cargo test --lib` verdes, com a cobertura do núcleo
      acima de 80% (RNF-03).
- [ ] `cargo clippy` sem avisos novos.
- [ ] As BRs descrevem o que **foi implementado** (não o que foi planejado).
- [ ] `memory.md` tem a entrada da data, com `IDs:` e `Refs:`.
- [ ] Os issues reflecting o estado final.
- [ ] AppImage e `.deb` reconstruídos e o app aberto com o build novo.

## Notas

- A cobertura do núcleo é a meta de R9 (rules): as tasks 01, 03 e 08 são as
  que mais pesam no `domain`/`application`.
- Se o botão do notch ou a aba ficarem diferentes do desenhado, **corrigir o
  doc** (não o contrário): o doc é a fonte única (BR-016).
