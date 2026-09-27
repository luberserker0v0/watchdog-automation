import { useTranslation } from "react-i18next";
import { usePreferencesStore } from "../../shell/stores/usePreferencesStore";
import { checkForUpdates, installAvailableUpdate, runtimeBusyReason } from "../services/updateService";
import { useUpdateStore } from "../stores/useUpdateStore";

function byteSize(value: number): string {
  if (value < 1024) return `${value} B`;
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KiB`;
  return `${(value / (1024 * 1024)).toFixed(1)} MiB`;
}

export default function UpdateSettingsSection() {
  const { t, i18n } = useTranslation();
  const automatic = usePreferencesStore((state) => state.automaticUpdateChecks);
  const setAutomatic = usePreferencesStore((state) => state.setAutomaticUpdateChecks);
  const state = useUpdateStore();
  const busy = runtimeBusyReason();
  const checking = state.phase === "checking";
  const installing = state.phase === "downloading" || state.phase === "installing";
  const progress = state.totalBytes && state.totalBytes > 0
    ? Math.min(100, Math.round((state.downloadedBytes / state.totalBytes) * 100))
    : null;

  return (
    <div className="settings-section update-settings-section">
      <h3 className="settings-section-title">{t("updater.title")}</h3>
      <div className="settings-section-body">
        <div className="settings-item">
          <label className="settings-item-label" htmlFor="automatic-update-checks">{t("updater.automaticChecks")}</label>
          <label className="settings-toggle">
            <input id="automatic-update-checks" type="checkbox" checked={automatic}
              onChange={(event) => setAutomatic(event.target.checked)} />
            <span className="settings-toggle-slider" />
          </label>
          <span className="settings-item-hint">{t("updater.automaticHint")}</span>
        </div>
        <div className="settings-item">
          <span className="settings-item-label">{t("updater.status")}</span>
          <span className="settings-item-text">{t(`updater.phase.${state.phase}`)}</span>
          {state.lastCheckTime && <span className="settings-item-hint">
            {t("updater.lastChecked", { time: new Intl.DateTimeFormat(i18n.resolvedLanguage, { dateStyle: "medium", timeStyle: "short" }).format(state.lastCheckTime) })}
          </span>}
          {state.error && <span className="settings-item-error" role="alert">{state.error}</span>}
        </div>
        {state.currentVersion && <div className="settings-item">
          <span className="settings-item-label">{t("updater.currentVersion")}</span>
          <span className="settings-item-text">{state.currentVersion}</span>
        </div>}
        {state.update && <>
          <div className="settings-item">
            <span className="settings-item-label">{t("updater.offeredVersion")}</span>
            <span className="settings-item-text">{state.update.version}</span>
            {state.update.publicationDate && <span className="settings-item-hint">{state.update.publicationDate}</span>}
          </div>
          <div className="settings-item update-notes-item">
            <span className="settings-item-label">{t("updater.releaseNotes")}</span>
            <pre className="update-release-notes">{state.update.notes || t("updater.noNotes")}</pre>
          </div>
          {installing && <div className="settings-item">
            <span className="settings-item-label">{t("updater.progress")}</span>
            <progress value={progress ?? undefined} max={100} />
            <span className="settings-item-hint">
              {progress == null ? byteSize(state.downloadedBytes) : `${progress}% · ${byteSize(state.downloadedBytes)} / ${byteSize(state.totalBytes!)}`}
            </span>
          </div>}
        </>}
        <div className="settings-item update-actions">
          <span className="settings-item-label" />
          <button className="settings-action" type="button" disabled={checking || installing}
            onClick={() => void checkForUpdates()}>{checking ? t("updater.checking") : t("updater.check")}</button>
          {state.update && <button className="settings-action primary" type="button" disabled={installing || busy !== null}
            onClick={() => {
              if (window.confirm(t("updater.confirmInstall", { version: state.update!.version }))) {
                void installAvailableUpdate().catch(() => undefined);
              }
            }}>{t("updater.installRestart")}</button>}
          {busy && <span className="settings-item-hint">{t(`updater.busy.${busy}`)}</span>}
        </div>
      </div>
    </div>
  );
}
