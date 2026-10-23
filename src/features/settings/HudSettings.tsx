import {
  DWELL_STEP_MS,
  HUD_DENSITIES,
  MAX_AI_FONT_SCALE,
  MAX_DWELL_MS,
  MAX_FONT_SCALE,
  MAX_OVERLAY_OPACITY,
  MIN_AI_FONT_SCALE,
  MIN_DWELL_MS,
  MIN_FONT_SCALE,
  MIN_OVERLAY_OPACITY,
  NOTCH_ANCHORS,
  OPACITY_LEGEND,
  type HudDensity,
  type NotchAnchor,
  type Settings,
} from "../../core/domain/settings";
import { useTranslation } from "../../core/l10n";
import {
  hudDensityKey,
  notchAnchorKey,
  opacityLegendKey,
} from "../../core/l10n/labels";

interface HudSettingsProps {
  settings: Settings;
  update: <K extends keyof Settings>(field: K, value: Settings[K]) => void;
}

/** Opacidade das superfícies de todas as janelas (BR-062). */
const OPACITY_STEP = 5;

/** Passo dos sliders de escala de fonte, em pontos percentuais (BR-087). */
const FONT_SCALE_STEP = 5;
const AI_FONT_SCALE_STEP = 5;

/** Converte um multiplicador (0,75–1,6) para pontos percentuais inteiros. */
const toPercent = (value: number): number => Math.round(value * 100);

/**
 * Aba "Geral & HUD" (BR-062/BR-087).
 *
 * Duas seções: **Aparência & Fonte** (escala global da UI + bônus do texto de
 * IA) e o comportamento do HUD — opacidade global das janelas, ancoragem,
 * dwell-hover e densidade do notch — na forma do mockup (cards em grade,
 * segmentados, toggle e slider com badge). O modo stealth fica no card de
 * privacidade.
 */
export function HudSettings({ settings, update }: HudSettingsProps) {
  const { t } = useTranslation();
  const opacityPercent = Math.round(settings.overlay_opacity * 100);
  const fontPercent = toPercent(settings.font_scale);
  const aiPercent = toPercent(settings.ai_font_scale);
  const anchored = t(notchAnchorKey(settings.notch_anchor));

  return (
    <>
      {/* Escala de fonte (BR-087): global + bônus do texto de IA. */}
      <section className="settings__hud">
        <header className="settings__section-header">
          <h2 className="settings__section-title">{t("hud_appearance_font")}</h2>
          <span className="settings__section-note">
            {t("hud_appearance_font_note")}
          </span>
        </header>

        <div className="settings__grid">
          <article className="settings__card settings__card--hud">
            <header className="settings__card-header">
              <h3 className="settings__card-title">{t("hud_font_scale")}</h3>
              <span className="settings__badge">{fontPercent}%</span>
            </header>
            <p className="settings__card-description">
              {t("hud_font_scale_description")}
            </p>
            <input
              type="range"
              aria-label={t("hud_font_scale_slider_label")}
              min={toPercent(MIN_FONT_SCALE)}
              max={toPercent(MAX_FONT_SCALE)}
              step={FONT_SCALE_STEP}
              value={fontPercent}
              onChange={(event) =>
                update("font_scale", Number(event.target.value) / 100)
              }
            />
            <div className="settings__range-labels">
              <span>{t("hud_font_scale_small_75")}</span>
              <span>{t("hud_font_scale_default_100")}</span>
              <span>{t("hud_font_scale_large_160")}</span>
            </div>
          </article>

          <article className="settings__card settings__card--hud">
            <header className="settings__card-header">
              <h3 className="settings__card-title">{t("hud_ai_font_scale")}</h3>
              <span className="settings__badge">{aiPercent}%</span>
            </header>
            <p className="settings__card-description">
              {t("hud_ai_font_scale_description")}
            </p>
            <input
              type="range"
              aria-label={t("hud_ai_font_scale_slider_label")}
              min={toPercent(MIN_AI_FONT_SCALE)}
              max={toPercent(MAX_AI_FONT_SCALE)}
              step={AI_FONT_SCALE_STEP}
              value={aiPercent}
              onChange={(event) =>
                update("ai_font_scale", Number(event.target.value) / 100)
              }
            />
            <div className="settings__range-labels">
              <span>{t("hud_ai_font_scale_original_100")}</span>
              <span>{t("hud_ai_font_scale_bigger_150")}</span>
            </div>
          </article>
        </div>
      </section>

      <section className="settings__hud">
        <header className="settings__section-header">
          <h2 className="settings__section-title">
            {t("hud_hud_window_behavior")}
          </h2>
          <span className="settings__section-note">
            {t("hud_hud_anchor_anchor", { anchor: anchored })}
          </span>
        </header>

        <div className="settings__grid">
          <article className="settings__card settings__card--hud">
            <header className="settings__card-header">
              <h3 className="settings__card-title">
                {t("hud_window_opacity")}
              </h3>
              <span className="settings__badge">{opacityPercent}%</span>
            </header>
            <p className="settings__card-description">
              {t("hud_transparency_all_window_surfaces_applied_real",
              )}
            </p>
            <input
              type="range"
              aria-label={t("hud_opacity_slider_label")}
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
                <span key={entry.label}>{t(opacityLegendKey(entry.label))}</span>
              ))}
            </div>
          </article>

          <article className="settings__card settings__card--hud">
            <header className="settings__card-header">
              <h3 className="settings__card-title">
                {t("hud_notch_anchor_position")}
              </h3>
            </header>
            <p className="settings__card-description">
              {t("hud_anchor_point_orientation_hud_primary_workspace",
              )}
            </p>
            <div
              className="settings__segmented"
              role="group"
              aria-label={t("hud_notch_anchor_group_label")}
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
                  {t(notchAnchorKey(anchor))}
                </button>
              ))}
            </div>
          </article>

          <article className="settings__card settings__card--hud">
            <header className="settings__card-header">
              <h3 className="settings__card-title">
                {t("hud_expand_notch_dwell_hover")}
              </h3>
              <label className="settings__toggle">
                <input
                  type="checkbox"
                  aria-label={t("hud_expand_notch_dwell")}
                  checked={settings.dwell_enabled}
                  onChange={(event) =>
                    update("dwell_enabled", event.target.checked)
                  }
                />
                <span className="settings__toggle-track" aria-hidden="true" />
              </label>
            </header>
            <p className="settings__card-description">
              {t("hud_opens_quick_transcription_view_hovering_top",
              )}
            </p>

            <div className="settings__hud-field">
              <span className="settings__hud-field-label">
                {t("hud_dwell_time")}
                <span className="settings__badge">{settings.dwell_ms}ms</span>
              </span>
              <input
                type="range"
                aria-label={t("hud_dwell_time_milliseconds")}
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
              <span>{t("hud_100ms_fast")}</span>
              <span>{t("hud_1_5s_long")}</span>
            </div>
          </article>

          <article className="settings__card settings__card--hud">
            <header className="settings__card-header">
              <h3 className="settings__card-title">
                {t("hud_notch_size_density")}
              </h3>
            </header>
            <p className="settings__card-description">
              {t("hud_defines_internal_spacing_thickness_floating_bar",
              )}
            </p>
            <div
              className="settings__segmented"
              role="group"
              aria-label={t("hud_notch_density")}
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
                  {t(hudDensityKey(density))}
                </button>
              ))}
            </div>
          </article>
        </div>

        <article className="settings__card settings__card--hud">
          <header className="settings__card-header">
            <h3 className="settings__card-title">{t("hud_privacy")}</h3>
            <label className="settings__toggle">
              <input
                type="checkbox"
                aria-label={t("hud_stealth_mode")}
                checked={settings.stealth}
                onChange={(event) => update("stealth", event.target.checked)}
              />
              <span className="settings__toggle-track" aria-hidden="true" />
            </label>
          </header>
          <p className="settings__card-description">
            {t("hud_stealth_mode_hidden_taskbar_no_window")}
          </p>
          <p className="settings__hint">
            {t("hud_not_hide_screen_sharing_linux_no",
            )}
          </p>
        </article>
      </section>
    </>
  );
}
