# 10 — Histórico, export, documentação e pacotes

- **Status:** IN PROGRESS
- **Depende de:** 08, 09
- **Habilita:** —
- **Issues:** ISS-049 · **BR:** BR-068, BR-071
- **Refs:** [`../prd.md`](../prd.md) US-05; [`../issues.md`](../issues.md)

## Objetivo

Fechar o ciclo: achar e abrir a gravação pelo Histórico, exportar o caminho no
Markdown, documentar e empacotar o binário de captura.

## Entregáveis

- `src/features/history/HistoryPanel.tsx`:
  - selo `Vídeo`/`Áudio` nas sessões com gravação;
  - ação **Abrir gravação** no detalhe;
  - aviso "o vídeo também será apagado" na exclusão (CA-05.4).
- `src-tauri/src/application/markdown_export.rs`: linha
  `Gravação: <caminho> (<tamanho>)` no cabeçalho, quando existir (CA-05.3).
- `src-tauri/src/commands`/`session_commands.rs`: `recording_path` no DTO do
  histórico.
- `src-tauri/src/lib.rs`: abrir o player no início da gravação
  (`started_at_ms` da gravação, não da sessão — CA-05.2).
- `prds/business-rules.md`: BR-065…BR-071 (feito).
- `prds/recording/memory.md`: entrada de criação e de revisão.
- Empacotamento (bloqueado por **ISS-049**):
  - `.deb` `depends` com `ffmpeg` **ou** sidecar em `bundle.resources`;
  - `tauri.conf.json` com `externalBin`/`resources` conforme a decisão de
    licença;
  - teste do AppImage numa máquina sem `ffmpeg` no `PATH`.

## Estado (2026-10-21)

- **Feito:** selo `Vídeo`/`Áudio` no Histórico, ação **Abrir gravação**, linha
  `Gravação:` no export, exclusão apagando todas as execuções.
- **Pendente:** abrir o player no início da gravação (hoje revela o arquivo) e o
  empacotamento (`ffmpeg`/`parec`, ISS-049).

## Critérios de aceite

- [ ] O Histórico mostra o selo e abre a gravação no gerenciador.
- [ ] O export Markdown inclui a linha da gravação e continua válido sem ela.
- [ ] Excluir sessão com gravação apaga o arquivo **e** avisa antes.
- [ ] O app funciona sem `ffmpeg`: o botão fica desabilitado com motivo e nada
      quebra.
- [ ] `cargo test`, `cargo clippy`, `pnpm test`, `pnpm build` verdes.
- [ ] `memory.md` com as decisões e os IDs (BR-065…BR-071, ISS-049…ISS-053).
- [ ] Pacote instalado numa máquina limpa grava 10 s e reproduz o MP4.

## Verificação

- `cargo test` · `cargo clippy` · `pnpm test` · `pnpm tauri:build:cpu`
- Teste manual: instalar o `.deb` em ambiente sem `ffmpeg`

## Notas

- A decisão de empacotamento (sidecar GPL vs dependência de sistema vs build
  LGPL com VAAPI) é a **Q-01/ISS-049** e precede o release.
- Referenciar os IDs em vez de repetir as regras (BR-016).
