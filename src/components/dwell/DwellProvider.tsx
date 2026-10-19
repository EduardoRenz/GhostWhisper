import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { dwellHide, dwellShow, dwellTriggerHover } from "../../adapters/ipc/dwell";
import { subscribe } from "../../adapters/ipc/events";
import { DEFAULT_DWELL_MS } from "../../core/domain/dwell";

/** Contrato consumido pelos botões opt-in. */
export interface DwellContextValue {
  /** Drill-hover ligado nas settings. */
  enabled: boolean;
  /** Duração de cada estágio (ms). */
  durationMs: number;
  /** Gatilho cuja confirmação está aberta (ou `null`). */
  openTrigger: string | null;
  /** Abre a confirmação para a ação do gatilho. */
  request: (
    triggerId: string,
    anchor: DOMRect,
    action: () => void,
  ) => void;
  /** Cancela/fecha sem executar. */
  cancel: () => void;
  /** Clique: executa a ação na hora (fecha uma confirmação aberta). */
  click: (triggerId: string, action: () => void) => void;
  /** O cursor entrou/saiu do gatilho (para o grace). */
  setTriggerHover: (triggerId: string, hovered: boolean) => void;
}

const fallback: DwellContextValue = {
  enabled: false,
  durationMs: DEFAULT_DWELL_MS,
  openTrigger: null,
  request: () => {},
  cancel: () => {},
  click: (_triggerId, action) => action(),
  setTriggerHover: () => {},
};

const DwellContext = createContext<DwellContextValue | null>(null);

/** Contexto do drill-hover; fora do provider vira no-op (clique funciona). */
export function useDwell(): DwellContextValue {
  return useContext(DwellContext) ?? fallback;
}

interface DwellProviderProps {
  enabled: boolean;
  durationMs: number;
  /** Só mantém a confirmação com sessão/overlay ativo. */
  active: boolean;
  children: ReactNode;
}

/**
 * Controlador do drill-hover no overlay.
 *
 * Guarda a **ação pendente** (para a janela `dwell` só responder "Sim"), o gatilho
 * aberto e o estado de hover das duas janelas. O grace (`dwell_ms`) evita fechar
 * a confirmação quando o cursor atravessa de uma janela para a outra.
 */
export function DwellProvider({
  enabled,
  durationMs,
  active,
  children,
}: DwellProviderProps) {
  const [openTrigger, setOpenTrigger] = useState<string | null>(null);
  const openRef = useRef<string | null>(null);
  const pendingAction = useRef<(() => void) | null>(null);

  const updateOpen = useCallback((triggerId: string | null) => {
    openRef.current = triggerId;
    setOpenTrigger(triggerId);
  }, []);

  const cancel = useCallback(() => {
    pendingAction.current = null;
    updateOpen(null);
    void dwellHide().catch(() => {});
  }, [updateOpen]);

  const request = useCallback(
    (triggerId: string, anchor: DOMRect, action: () => void) => {
      if (!enabled || openRef.current) {
        return;
      }
      pendingAction.current = action;
      updateOpen(triggerId);
      void dwellShow(
        {
          x: Math.round(anchor.left),
          y: Math.round(anchor.top),
          width: Math.round(anchor.width),
          height: Math.round(anchor.height),
        },
        durationMs,
      ).catch(() => {
        pendingAction.current = null;
        updateOpen(null);
      });
    },
    [enabled, durationMs, updateOpen],
  );

  const click = useCallback(
    (_triggerId: string, action: () => void) => {
      if (openRef.current) {
        pendingAction.current = null;
        updateOpen(null);
        void dwellHide().catch(() => {});
      }
      action();
    },
    [updateOpen],
  );

  const setTriggerHover = useCallback(
    (triggerId: string, hovered: boolean) => {
      if (openRef.current === triggerId) {
        void dwellTriggerHover(hovered).catch(() => {});
      }
    },
    [],
  );

  // Resultado da janela `dwell` (confirmou/cancelou).
  useEffect(() => {
    let stopClosed: (() => void) | undefined;
    void subscribe<{ confirmed: boolean }>("dwell_closed", (payload) => {
      const action = pendingAction.current;
      pendingAction.current = null;
      updateOpen(null);
      if (payload.confirmed && action) {
        action();
      }
    }).then((stop) => {
      stopClosed = stop;
    });
    return () => stopClosed?.();
  }, [updateOpen]);

  // Sem sessão/overlay ativo, não deixa confirmação pendurada.
  useEffect(() => {
    if (!active && openRef.current) {
      cancel();
    }
  }, [active, cancel]);

  // Desligar o drill-hover nos Ajustes fecha uma confirmação já aberta.
  useEffect(() => {
    if (!enabled && openRef.current) {
      cancel();
    }
  }, [enabled, cancel]);

  // `Escape` cancela (quando o foco fica no overlay).
  useEffect(() => {
    if (!openTrigger) {
      return;
    }
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        cancel();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [openTrigger, cancel]);

  // Desmontou: fecha a janela.
  useEffect(
    () => () => {
      if (openRef.current) {
        void dwellHide().catch(() => {});
      }
    },
    [],
  );

  const value = useMemo<DwellContextValue>(
    () => ({
      enabled,
      durationMs,
      openTrigger,
      request,
      cancel,
      click,
      setTriggerHover,
    }),
    [enabled, durationMs, openTrigger, request, cancel, click, setTriggerHover],
  );

  return (
    <DwellContext.Provider value={value}>{children}</DwellContext.Provider>
  );
}
