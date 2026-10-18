import { useCallback, useEffect, useState } from "react";
import { assistantModesList } from "../../adapters/ipc/suggestions";
import type { AssistantMode } from "../domain/assistant";

/** Lista os agentes embutidos + personalizados (recarrega quando `refreshKey` muda). */
export function useAssistantModes(refreshKey?: unknown): {
  modes: AssistantMode[];
  reload: () => Promise<void>;
} {
  const [modes, setModes] = useState<AssistantMode[]>([]);

  const reload = useCallback(async () => {
    const list = await assistantModesList();
    if (list) {
      setModes(list);
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload, refreshKey]);

  return { modes, reload };
}
