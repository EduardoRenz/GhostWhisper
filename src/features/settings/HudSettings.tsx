import {
  DWELL_STEP_MS,
  HUD_DENSITIES,
  HUD_DENSITY_LABELS,
  MAX_DWELL_MS,
  MAX_OVERLAY_OPACITY,
  MIN_DWELL_MS,
  MIN_OVERLAY_OPACITY,
  NOTCH_ANCHORS,
  NOTCH_ANCHOR_LABELS,
  OPACITY_LEGEND,
  type HudDensity,
  type NotchAnchor,
  type Settings,
} from "../../core/domain/settings";

interface HudSettingsProps {
  settings: Settings;
  update: <K extends keyof Settings>(field: K, value: Settings[K]) => void;
}

/** Opacidade das superfícies de todas as janelas (BR-062). */
const OPACITY_STEP = 5;

/**
 * Aba "Geral & HUD" (BR-062).
 *
 * Reúne o comportamento do HUD: opacidade global das janelas, ancoragem,
 * dwell-hover e densidade do notch — na forma do mockup (cards em grade,
 * segmentados, toggle e slider com badge). O modo stealth fica no card de
 * privacidade.
 */
export function HudSettings({ settings, update }: HudSettingsProps) {
  const opacityPercent = Math.round(settings.overlay_opacity * 100);
  const anchored = NOTCH_ANCHOR_LABELS[settings.notch_anchor];

  return (
    <section className="settings__hud">
      <header className="settings__section-header">
        <h2 className="settings__section-title">Comportamento do HUD &amp; Janela</h2>
        <span className="settings__section-note">HUD Anchor: {anchored}</span>
      </header>

      <div className="settings__grid">
        <article className="settings__card settings__card--hud">
          <header className="settings__card-header">
            <h3 className="settings__card-title">Opacidade das Janelas</h3>
            <span className="settings__badge">{opacityPercent}%</span>
          </header>
          <p className="settings__card-description">
            Transparência das superfícies de todas as janelas. Aplicada em tempo
            real e salva automaticamente.
          </p>
          <input
            type="range"
            aria-label="Opacidade das janelas"
            min={MIN_OVERLAY_OPACITY * 100}
            max={MAX_OVERLAY_OPACITY * 100}
            step={OPACITY_STEP}
            value={opacityPercent}
            onChange={(event) =>
              update("overlay_opacity", Number(event.target.value) / 100)
            }
          />
          <div className="settings__range-labels">
            {OPACITY_LEGEND.map((entry) => (
              <span key={entry.label}>{entry.label}</span>
            ))}
          </div>
        </article>

        <article className="settings__card settings__card--hud">
          <header className="settings__card-header">
            <h3 className="settings__card-title">Posição de Ancoragem do Notch</h3>
          </header>
          <p className="settings__card-description">
            Ponto de fixação e orientação do HUD na área de trabalho primária.
          </p>
          <div
            className="settings__segmented"
            role="group"
            aria-label="Posição de ancoragem do notch"
          >
            {NOTCH_ANCHORS.map((anchor: NotchAnchor) => (
              <button
                key={anchor}
                type="button"
                aria-pressed={settings.notch_anchor === anchor}
                className={
                  settings.notch_anchor === anchor
                    ? "settings__segment settings__segment--active"
                    : "settings__segment"
                }
                onClick={() => update("notch_anchor", anchor)}
              >
                {NOTCH_ANCHOR_LABELS[anchor]}
              </button>
            ))}
          </div>
        </article>

        <article className="settings__card settings__card--hud">
          <header className="settings__card-header">
            <h3 className="settings__card-title">Expandir Notch por Dwell (Hover)</h3>
            <label className="settings__toggle">
              <input
                type="checkbox"
                aria-label="Expandir notch por dwell"
                checked={settings.dwell_enabled}
                onChange={(event) =>
                  update("dwell_enabled", event.target.checked)
                }
              />
              <span className="settings__toggle-track" aria-hidden="true" />
            </label>
          </header>
          <p className="settings__card-description">
            Abre visualização rápida de transcrição ao pairar o mouse no topo da
            tela.
          </p>

          <div className="settings__hud-field">
            <span className="settings__hud-field-label">
              Tempo de Dwell
              <span className="settings__badge">{settings.dwell_ms}ms</span>
            </span>
            <input
              type="range"
              aria-label="Tempo de dwell em milissegundos"
              min={MIN_DWELL_MS}
              max={MAX_DWELL_MS}
              step={DWELL_STEP_MS}
              value={settings.dwell_ms}
              disabled={!settings.dwell_enabled}
              onChange={(event) =>
                update("dwell_ms", Number(event.target.value))
              }
            />
          </div>
          <div className="settings__range-labels">
            <span>100ms (Rápido)</span>
            <span>1,5s (Longo)</span>
          </div>
        </article>

        <article className="settings__card settings__card--hud">
          <header className="settings__card-header">
            <h3 className="settings__card-title">Tamanho / Densidade do Notch</h3>
          </header>
          <p className="settings__card-description">
            Define o espaçamento interno e a espessura da barra flutuante.
          </p>
          <div
            className="settings__segmented"
            role="group"
            aria-label="Densidade do notch"
          >
            {HUD_DENSITIES.map((density: HudDensity) => (
              <button
                key={density}
                type="button"
                aria-pressed={settings.hud_density === density}
                className={
                  settings.hud_density === density
                    ? "settings__segment settings__segment--active"
                    : "settings__segment"
                }
                onClick={() => update("hud_density", density)}
              >
                {HUD_DENSITY_LABELS[density]}
              </button>
            ))}
          </div>
        </article>
      </div>

      <article className="settings__card settings__card--hud">
        <header className="settings__card-header">
          <h3 className="settings__card-title">Privacidade</h3>
          <label className="settings__toggle">
            <input
              type="checkbox"
              aria-label="Modo stealth"
              checked={settings.stealth}
              onChange={(event) => update("stealth", event.target.checked)}
            />
            <span className="settings__toggle-track" aria-hidden="true" />
          </label>
        </header>
        <p className="settings__card-description">
          Modo stealth: fora da barra de tarefas e sem título na janela.
        </p>
        <p className="settings__hint">
          <strong>Ele não o esconde do compartilhamento de tela</strong>: no Linux
          não existe API para excluir uma janela de captura (BR-006). Para
          reuniões, compartilhe <em>uma janela</em> em vez da tela inteira ou
          deixe o overlay em outro monitor.
        </p>
      </article>
    </section>
  );
}
