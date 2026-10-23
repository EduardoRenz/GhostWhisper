import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  clearSourceKey,
  hasSourceKey,
  setSourceKey,
  sourceKeyPreview,
  sourcesHealth,
  validateSource,
} from "../../adapters/ipc/aiSources";
import { subscribe } from "../../adapters/ipc/events";
import { onWindowFocus } from "../../adapters/ipc/client";
import { secretsPersistent } from "../../adapters/ipc/settings";
import type { AiSource, AiSourceHealth, ProviderKind } from "../../core/domain/settings";
import { useTranslation } from "../../core/l10n";
import { AddProviderForm } from "./AddProviderForm";
import { ProviderCard } from "./ProviderCard";

/** Fonte de IA que respondeu (evento `llm_source_used`). */
interface LlmSourceUsage {
  source_id: string;
  label: string;
  is_fallback: boolean;
  latency_ms: number;
}

interface AiSourcesSettingsProps {
  /** Fontes do rascunho (a ordem é a prioridade). */
  sources: AiSource[];
  /** Provedores suportados (vindos do core). */
  providers: ProviderKind[];
  /** Troca a lista do rascunho. */
  onChange: (sources: AiSource[]) => void;
  /** Mostra uma mensagem na tela (status global). */
  onStatus: (status: { kind: "info" | "ok" | "error"; text: string }) => void;
}

/**
 * Aba "Fontes de IA" (BR-044, BR-045).
 *
 * Lista as fontes na ordem de prioridade (a primeira é a primária), com
 * reordenação, promoção, chaves por fonte e o sinal de saúde (conectada,
 * em fallback, falhou).
 */
export function AiSourcesSettings({
  sources,
  providers,
  onChange,
  onStatus,
}: AiSourcesSettingsProps) {
  const { t } = useTranslation();
  const [health, setHealth] = useState<AiSourceHealth[]>([]);
  const [keyState, setKeyState] = useState<Record<string, { has: boolean; preview: string }>>(
    {},
  );
  const [persistent, setPersistent] = useState(true);
  const [busy, setBusy] = useState(false);
  // O id arrastado fica num ref: o `drop` acontece no mesmo "tick" do
  // `dragstart` e o estado ainda estaria velho no closure.
  const draggingId = useRef<string | null>(null);
  const [dropTargetId, setDropTargetId] = useState<string | null>(null);

  const refreshHealth = useCallback(async () => {
    const entries = await sourcesHealth();
    if (entries) setHealth(entries);
  }, []);

  const refreshKeys = useCallback(async (list: AiSource[]) => {
    const entries = await Promise.all(
      list.map(async (source) => {
        const has = (await hasSourceKey(source.id)) ?? false;
        const preview = has ? ((await sourceKeyPreview(source.id)) ?? "") : "";
        return [source.id, { has, preview }] as const;
      }),
    );
    setKeyState(Object.fromEntries(entries));
  }, []);

  useEffect(() => {
    void (async () => {
      setPersistent((await secretsPersistent()) ?? true);
      await refreshHealth();
    })();
  }, [refreshHealth]);

  // A janela de Configurações **nasce oculta**: o React monta no boot e a
  // saúde da fonte ficaria congelada no estado inicial ("Nunca usada") mesmo
  // depois de a fonte responder. Ao ganhar foco, busca de novo (BR-045).
  useEffect(() => {
    let stop: (() => void) | undefined;
    void onWindowFocus((focused) => {
      if (focused) void refreshHealth();
    }).then((unlisten) => {
      stop = unlisten;
    });
    return () => stop?.();
  }, [refreshHealth]);

  useEffect(() => {
    void refreshKeys(sources);
  }, [sources, refreshKeys]);

  // A fonte que respondeu (com fallback) muda ao vivo (BR-045).
  useEffect(() => {
    let stop: (() => void) | undefined;
    void subscribe<LlmSourceUsage>("llm_source_used", (usage) => {
      setHealth((previous) => {
        const others = previous.filter((entry) => entry.source_id !== usage.source_id);
        return [
          ...others,
          {
            source_id: usage.source_id,
            state: "ready",
            consecutive_failures: 0,
            latency_ms: usage.latency_ms,
            last_error: null,
            is_fallback: usage.is_fallback,
            open_until_ms: null,
          },
        ];
      });
    }).then((unlisten) => {
      stop = unlisten;
    });
    return () => stop?.();
  }, []);

  const healthById = useMemo(() => {
    const map = new Map<string, AiSourceHealth>();
    for (const entry of health) map.set(entry.source_id, entry);
    return map;
  }, [health]);

  const replace = (next: AiSource[]) => onChange(next);

  const makePrimary = (id: string) => {
    const source = sources.find((entry) => entry.id === id);
    if (!source) return;
    replace([source, ...sources.filter((entry) => entry.id !== id)]);
  };

  const patch = (id: string, changes: Partial<AiSource>) => {
    replace(
      sources.map((source) =>
        source.id === id ? { ...source, ...changes } : source,
      ),
    );
  };

  const remove = (id: string) => {
    replace(sources.filter((source) => source.id !== id));
  };

  const move = (fromId: string, toId: string) => {
    if (fromId === toId) return;
    const from = sources.findIndex((source) => source.id === fromId);
    const to = sources.findIndex((source) => source.id === toId);
    if (from < 0 || to < 0) return;
    const next = [...sources];
    const [moved] = next.splice(from, 1);
    if (!moved) return;
    next.splice(to, 0, moved);
    replace(next);
  };

  const withBusy = async (action: () => Promise<void>) => {
    setBusy(true);
    try {
      await action();
    } finally {
      setBusy(false);
    }
  };

  const onValidate = (source: AiSource) =>
    withBusy(async () => {
      try {
        const reply = await validateSource(source);
        await refreshHealth();
        onStatus({
          kind: "ok",
          text: t("sources_connection_validated_label_reply", {
            label: source.label,
            reply: reply ?? t("sources_no_response"),
          }),
        });
      } catch (error) {
        await refreshHealth();
        onStatus({ kind: "error", text: String(error) });
      }
    });

  const onChangeKey = (source: AiSource, key: string) =>
    withBusy(async () => {
      try {
        await setSourceKey(source.id, key);
        await refreshKeys(sources);
        onStatus({ kind: "ok", text: t("sources_key_label_saved_keyring", { label: source.label }) });
      } catch (error) {
        onStatus({ kind: "error", text: String(error) });
      }
    });

  const onClearKey = (source: AiSource) =>
    withBusy(async () => {
      try {
        await clearSourceKey(source.id);
        await refreshKeys(sources);
        onStatus({ kind: "info", text: t("sources_key_label_removed", { label: source.label }) });
      } catch (error) {
        onStatus({ kind: "error", text: String(error) });
      }
    });

  return (
    <>
      <section className="settings__card">
        <header className="settings__card-header">
          <h2 className="settings__card-title">
            {t("sources_active_ai_providers")}
          </h2>
          <span className="settings__card-note">
            {sources.length === 1
              ? t("sources_1_source_configured")
              : t("sources_count_sources_configured", { count: sources.length })}{" "}
            · {persistent ? t("sources_secure_keyring_active") : t("sources_keys_only_session")}
          </span>
        </header>
        <p className="settings__hint">
          {t("sources_first_source_list_answers_fails_next",
          )}
        </p>

        <div className="settings__sources">
          {sources.map((source, index) => (
            <ProviderCard
              key={source.id}
              source={source}
              isPrimary={index === 0}
              health={healthById.get(source.id)}
              hasKey={keyState[source.id]?.has ?? false}
              keyPreview={keyState[source.id]?.preview ?? ""}
              canRemove={sources.length > 1}
              busy={busy}
              onMakePrimary={() => makePrimary(source.id)}
              onValidate={() => onValidate(source)}
              onChangeKey={(key) => onChangeKey(source, key)}
              onClearKey={() => onClearKey(source)}
              onEdit={(changes) => patch(source.id, changes)}
              onRemove={() => remove(source.id)}
              onDragStart={() => {
                draggingId.current = source.id;
              }}
              onDragEnd={() => {
                draggingId.current = null;
                setDropTargetId(null);
              }}
              onDragOver={() => setDropTargetId(source.id)}
              onDrop={() => {
                const from = draggingId.current;
                if (from) move(from, source.id);
                draggingId.current = null;
                setDropTargetId(null);
              }}
              isDropTarget={dropTargetId === source.id}
            />
          ))}
        </div>
      </section>

      <AddProviderForm providers={providers} onAdd={(source) => replace([...sources, source])} />
    </>
  );
}
