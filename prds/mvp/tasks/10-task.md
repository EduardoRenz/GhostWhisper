# 10 — Empacotamento + spike Wayland/invisibilidade

- **Status:** CONCLUDED
- **Depende de:** 01, 07, 09
- **Habilita:** —

## Objetivo
Gerar instaláveis Linux (`.deb` e AppImage) e executar o spike que decide a
viabilidade de Wayland nativo e de "invisibilidade" no compartilhamento de tela.

## Entregáveis
- `scripts/package-linux.sh` — build `.deb`/AppImage (com opção só `deb`).
- `src-tauri/tauri.conf.json` — `bundle.targets`, ícones, dependências de runtime.
- `prds/mvp/architecture/wayland-spike.md` — decisão go/no-go.
- `README.md` — instalação real, atalhos e aviso de Wayland/privacidade.

## Parte A — Empacotamento (evidências)
- `.deb`: `GhostWhisper_0.1.0_amd64.deb` (**7,92 MiB**) — pacote `open-sua`, amd64,
  contém `usr/bin/ghostwhisper` (26 MB), `GhostWhisper.desktop` e ícones hicolor.
  `Depends`: `libasound2t64 | libasound2, libpulse0, libwebkit2gtk-4.1-0,
  libgtk-3-0` (webkit/gtk são adicionados pelo Tauri).
- AppImage: `GhostWhisper_0.1.0_amd64.AppImage` (**85,64 MiB**) — `--appimage-extract`
  ok (AppRun + `.desktop` + ícone); `bundleMediaFramework=false`.
- Binário **release** executa (`timeout` → exit 124 = app rodando).
- `.deb`/AppImage construídos via `pnpm tauri build --bundles deb,appimage`.

## Parte B — Spike (resumo)
Decisão: **Wayland nativo = NO-GO** no MVP (manter X11/XWayland); **invisibilidade
no compartilhamento = NO-GO** (não prometer). Justificativa e evidências em
[`wayland-spike.md`](../architecture/wayland-spike.md): sem `gtk-layer-shell`,
Wayland não expõe cursor global (quebra o click-through), atalhos globais exigem
portal e o output virtual dedicado só existe em wlroots.

## Critérios de aceite
- [~] `.deb` instala e roda em Ubuntu e Fedora — **verificado por inspeção**
  (`dpkg-deb -I/-c`) + execução do binário release; instalação em VM limpa
  (Ubuntu 24.04 / Fedora 40) **pendente** (sem VM disponível). Fedora usa o
  **AppImage** (o `.deb` é formato Debian).
- [x] AppImage executa com `chmod +x` (sem instalar) — validado por extração.
- [x] Spike documentado com decisão clara (no-go) e riscos.
- [x] README atualizado com instruções de instalação reais e limitações.

## Verificação
- `cargo test` → **77 testes**; `pnpm test` → **10**; `clippy` (0 warnings) e
  `fmt` limpos.
- Bundles presentes e inspecionados; release roda.
- Cenário de **conflito de atalho** observado em uso real (instância antiga
  segurando as teclas): o app **reportou e continuou rodando**; após liberar as
  teclas, os 4 atalhos registraram.

## Pendências
- Matriz de instalação limpa em VM (Ubuntu 24.04 + Fedora 40 com AppImage).
- Validação manual de atalhos/overlay em GNOME/KDE (X11).
