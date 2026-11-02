# 04 — Adaptador AT-SPI (foco universal, cache, limites)

- **Status:** CONCLUDED (2026-11-01)
- **Depende de:** 03
- **Habilita:** 06
- **Issues:** ISS-090 · **BR:** BR-105

> **Desvio registrado (2026-11-01):** o architecture §3 previa X11 via
> `_NET_ACTIVE_WINDOW`+pid e Wayland via foco AT-SPI. A implementação usou o
> **foco AT-SPI como caminho único** para os dois ambientes: o evento
> `object:state-changed:focus` é gerado pelo toolkit do app (não pelo
> compositor), funciona igual em X11 e Wayland, e o `ApplicationProxy` do
> atspi 0.30 **não expõe pid** — o matching por pid (X11) não era viável
> sem gambiarra. Sem foco observado (boot recém-aberto), o snapshot é `None`
> ("Sem contexto") até o primeiro foco. Registro em
> [`../memory.md`](../memory.md). Evolução possível: fallback X11 com
> `_NET_ACTIVE_WINDOW` + WM_CLASS para o caso "nunca houve foco".

## Objetivo

Implementar `AtspiScreenContextSource`: conexão D-Bus de acessibilidade sob
demanda, resolução da janela ativa (X11) ou última focada (Wayland), caminhada
da árvore com limites e cache. Só é instanciado com a flag ligada.

## Entregáveis

- `src-tauri/src/adapters/screen/atspi_screen_context_source.rs` (novo):
  - crate `atspi` (zbus) como dependência (verificar versão/MSRV na task);
  - `snapshot()` lazy com timeout de 500 ms; caminhada com teto de 300 ms /
    5 000 nós;
  - X11: `_NET_ACTIVE_WINDOW` → `_NET_WM_PID` (x11rb) → nó AT-SPI por
    `ProcessId` (cache de pid ~2 s);
  - Wayland/sem X11: listener `object:state-changed:focused` em thread
    própria, estado `Arc<Mutex<Option<…>>>`;
  - `DeniedAccess`/nó sem árvore → `Ok(None)`; D-Bus inacessível → `Err`;
  - logs só com app/roles/nós/chars/latência (nunca conteúdo).
- `src-tauri/src/adapters/screen/mod.rs`: registrar.
- `lib.rs` (composition root): instanciar **somente** com
  `settings.screen_context_enabled`; caso contrário `None` no `AppState`.
- Teste de integração `#[ignore]` (requer sessão D-Bus): lê um app real.

## Critérios de aceite

- [ ] Flag desligada → nenhum `AtspiScreenContextSource` no `AppState`
      (teste de composição).
- [ ] X11: snapshot da janela ativa casa o pid correto (teste manual com
      Chrome/VS Code).
- [ ] Sem X11 (Wayland): segue o foco via AT-SPI; sem foco → `None`.
- [ ] Árvore gigante corta no teto (nós/chars) sem travar a AskBar.
- [ ] `DeniedAccess` → `None` sem erro.
- [ ] Nenhum log contém título/conteúdo (grep no teste).

## Testes

- `composition_omits_adapter_when_disabled`
- `snapshot_respects_node_and_time_limits` (com árvore sintética)
- `denied_access_yields_none`
- `logs_contain_no_content` (assert no harness de log)
