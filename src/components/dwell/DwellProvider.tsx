import { createContext, useContext, useMemo, type ReactNode } from "react";
import { DEFAULT_DWELL_MS } from "../../core/domain/settings";

/** Contrato consumido pelos botões opt-in do drill-hover. */
export interface DwellContextValue {
  /** Drill-hover ligado nas settings e com sessão ativa. */
  enabled: boolean;
  /** Tempo de permanência para executar a ação (ms). */
  durationMs: number;
}

const fallback: DwellContextValue = {
  enabled: false,
  durationMs: DEFAULT_DWELL_MS,
};

const DwellContext = createContext<DwellContextValue | null>(null);

/** Contexto do drill-hover; fora do provider vira no-op (clique funciona). */
export function useDwell(): DwellContextValue {
  return useContext(DwellContext) ?? fallback;
}

interface DwellProviderProps {
  enabled: boolean;
  durationMs: number;
  /** Só arma com sessão/overlay ativo. */
  active: boolean;
  children: ReactNode;
}

/**
 * Configuração do drill-hover no overlay.
 *
 * O mecanismo tem **um único estágio**: o `DwellButton` preenche durante
 * `durationMs` e executa a ação ao completar. Não há confirmação "Sim".
 */
export function DwellProvider({
  enabled,
  durationMs,
  active,
  children,
}: DwellProviderProps) {
  const value = useMemo<DwellContextValue>(
    () => ({ enabled: enabled && active, durationMs }),
    [enabled, durationMs, active],
  );
  return (
    <DwellContext.Provider value={value}>{children}</DwellContext.Provider>
  );
}
