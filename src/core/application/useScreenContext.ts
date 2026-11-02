import { useCallback, useEffect, useRef, useState } from "react";
import { subscribe } from "../../adapters/ipc/events";
import { setScreenContextInclude } from "../../adapters/ipc/settings";
import { screenContextSnapshot } from "../../adapters/ipc/screenContext";
import type { Settings } from "../domain/settings";
import {
  screenContextState,
  type ScreenContextEvent,
  type ScreenContextState,
} from "../domain/screenContext";

/** Evento sintético de "sem contexto/falha" para erro de invoke (ISS-092). */
const UNAVAILABLE: ScreenContextEvent = {
  available: false,
  failed: true,
  app: "",
  window_title: "",
  preview: "",
  captured_at_ms: 0,
};

/**
 * Contexto de tela da AskBar e da aba Sessão (BR-106).
 *
 * Um único hook para as duas superfícies: consulta ao montar, **reconsulta
 * sob demanda** (`refresh` — a AskBar chama ao abrir/focar, para nunca
 * mostrar estado velho), assina o evento `screen_context` (foco) e expõe o
 * **checkbox** (`checked`/`toggle`), gravado em settings e sincronizado
 * entre janelas via `settings_changed`.
 *
 * **Desmarcado não lê**: nenhuma consulta é feita (nem no mount, nem no
 * foco, nem ao receber evento) — o usuário desligou o contexto para não
 * gastar processamento. O estado vira `paused`, o evento é limpo e o core
 * também descarta o cache. Ao remarcar, a leitura recomeça na hora.
 */
export function useScreenContext(settings: Settings): {
  /** Estado derivado da pill (hidden quando a flag mestre está desligada). */
  state: ScreenContextState;
  /** Último evento do core (estado ativo). */
  event: ScreenContextEvent | null;
  /** Checkbox marcado (contexto é lido e vai na pergunta). */
  checked: boolean;
  /** Marca/desmarca (grava settings e sincroniza as janelas). */
  toggle: () => void;
  /** Reconsulta agora (ex.: ao abrir a barra) — nunca fica com estado velho. */
  refresh: () => void;
} {
  const [event, setEvent] = useState<ScreenContextEvent | null>(null);
  const enabled = settings.screen_context_enabled;
  // Otimista: o clique responde na hora e não depende do round-trip do
  // `settings_changed` (sem valor velho em cliques seguidos).
  const [checked, setChecked] = useState(settings.screen_context_include);

  const aliveRef = useRef(true);
  const enabledRef = useRef(enabled);
  const checkedRef = useRef(checked);
  const retryRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  /** Gravações de `screen_context_include` em voo (sincroniza o toggle). */
  const pendingWritesRef = useRef(0);
  /** Fila: cliques rápidos gravam em ordem (último clique manda). */
  const writeChainRef = useRef<Promise<unknown>>(Promise.resolve());

  useEffect(() => {
    enabledRef.current = enabled;
  }, [enabled]);

  const query = useCallback(() => {
    // Desmarcado: zero consultas — o pedido do usuário é não gastar
    // processamento (BR-106).
    if (!aliveRef.current || !enabledRef.current || !checkedRef.current) {
      return;
    }
    const run = () => {
      void screenContextSnapshot()
        .then((snapshot) => {
          // Desmarcado no meio da leitura: descarta a resposta e não agenda
          // retry — nada de processamento enquanto o contexto está desligado.
          if (!aliveRef.current || !checkedRef.current) return;
          const incoming = snapshot ?? UNAVAILABLE;
          setEvent(incoming);
          // Sem contexto (não falha): o app pode ainda estar expondo a
          // árvore — tenta de novo uma única vez (ISS-093).
          if (!incoming.available && !incoming.failed && retryRef.current === null) {
            retryRef.current = setTimeout(() => {
              retryRef.current = null;
              if (!aliveRef.current || !checkedRef.current || !enabledRef.current) {
                return;
              }
              run();
            }, 1_500);
          }
        })
        .catch(() => {
          if (aliveRef.current && checkedRef.current) setEvent(UNAVAILABLE);
        });
    };
    run();
  }, []);

  // Preferência vinda de **outra janela** (ou do SettingsPage): acompanha e,
  // se religou, relê — a gravação já aconteceu, sem corrida.
  const lastIncludeRef = useRef(settings.screen_context_include);
  useEffect(() => {
    const next = settings.screen_context_include;
    if (pendingWritesRef.current > 0) {
      // Enquanto o próprio toggle grava, o estado local (otimista) manda —
      // um reload intermediário não pode reverter o clique do usuário.
      return;
    }
    const changed = lastIncludeRef.current !== next;
    lastIncludeRef.current = next;
    checkedRef.current = next;
    setChecked(next);
    if (changed && next && enabledRef.current && aliveRef.current) {
      query();
    }
  }, [settings.screen_context_include, query]);

  useEffect(() => {
    aliveRef.current = true;
    if (enabled && checkedRef.current) {
      query();
    } else {
      // Flag desligada ou checkbox desmarcado: sem estado velho quando
      // religar.
      setEvent(null);
    }

    let stop: (() => void) | undefined;
    void subscribe<ScreenContextEvent>("screen_context", (incoming) => {
      // Eventos de foco são ignorados enquanto desmarcado — nada é lido.
      if (aliveRef.current && incoming && checkedRef.current) {
        setEvent(incoming);
      }
    })
      .then((unlisten) => {
        if (!aliveRef.current) {
          unlisten();
        } else {
          stop = unlisten;
        }
      })
      .catch(() => {
        // Sem runtime Tauri (testes): a pill fica no estado derivado.
      });

    return () => {
      aliveRef.current = false;
      if (retryRef.current !== null) {
        clearTimeout(retryRef.current);
        retryRef.current = null;
      }
      stop?.();
    };
  }, [enabled, query]);

  const toggle = useCallback(() => {
    const next = !checkedRef.current;
    checkedRef.current = next;
    // O `settings_changed` que vem deste toggle não é "outra janela" —
    // evita reler duas vezes.
    lastIncludeRef.current = next;
    setChecked(next);
    if (!next) {
      // Pausa imediata: limpa o contexto e o retry pendente sem esperar o
      // backend (o `clear` do core descarta o cache).
      setEvent(null);
      if (retryRef.current !== null) {
        clearTimeout(retryRef.current);
        retryRef.current = null;
      }
    }
    pendingWritesRef.current += 1;
    // Fila: cada clique grava **em ordem**; o último valor é o que fica.
    writeChainRef.current = writeChainRef.current
      .then(() => setScreenContextInclude(next))
      // Só lê **depois** de gravar — evita a corrida com a leitura vendo o
      // valor antigo.
      .then(() => {
        if (next) query();
      })
      .catch(() => {
        // Falha de IPC: volta ao valor anterior (nunca perde o checkbox).
        checkedRef.current = !next;
        lastIncludeRef.current = !next;
        setChecked(!next);
      })
      .finally(() => {
        pendingWritesRef.current -= 1;
      });
  }, [query]);

  return {
    state: screenContextState(enabled, event, checked),
    event,
    checked,
    toggle,
    refresh: query,
  };
}
