import { usePreferencesStore } from "../../shell/stores/usePreferencesStore";
import { useScreenStore } from "../stores/useScreenStore";
import { useWindowStore } from "../stores/useWindowStore";
import { useStreamStore } from "../stores/useStreamStore";
import { useTranslation } from "react-i18next";

function ScreenTargetSelect() {
  const { t } = useTranslation();
  const items = useScreenStore((state) => state.screens);
  const selectedIndex = useScreenStore((state) => state.selectedIndex);
  const setSelectedIndex = useScreenStore((state) => state.setSelectedIndex);
  const fetchScreens = useScreenStore((state) => state.fetchScreens);
  const isLoading = useScreenStore((state) => state.isLoading);
  const pipelineControlled = useStreamStore((state) => state.captureOwner === "pipeline");
  return (
    <>
      <select value={selectedIndex ?? ""} disabled={isLoading || pipelineControlled} onChange={(event) => {
        if (event.target.value) setSelectedIndex(Number(event.target.value));
      }}>
        <option value="">{isLoading ? t("capture.loading") : t("capture.selectTarget")}</option>
        {items.map((item) => (
          <option key={item.index} value={item.index}>
            Screen {item.index} - {item.width}x{item.height}{item.is_primary ? ` (${t("capture.primary")})` : ""}
          </option>
        ))}
      </select>
      <button
        type="button"
        className="target-refresh-btn"
        aria-label={t("capture.reloadScreens")}
        title={t("capture.reloadScreens")}
        disabled={isLoading || pipelineControlled}
        onClick={() => void fetchScreens()}
      >
        ↻
      </button>
    </>
  );
}

function WindowTargetSelect() {
  const { t } = useTranslation();
  const items = useWindowStore((state) => state.windows);
  const selectedIndex = useWindowStore((state) => state.selectedIndex);
  const setSelectedIndex = useWindowStore((state) => state.setSelectedIndex);
  const fetchWindows = useWindowStore((state) => state.fetchWindows);
  const isLoading = useWindowStore((state) => state.isLoading);
  const pipelineControlled = useStreamStore((state) => state.captureOwner === "pipeline");
  return (
    <>
      <select value={selectedIndex ?? ""} disabled={isLoading || pipelineControlled} onChange={(event) => {
        if (event.target.value) setSelectedIndex(Number(event.target.value));
      }}>
        <option value="">{isLoading ? t("capture.loading") : t("capture.selectTarget")}</option>
        {items.map((item) => (
          <option key={item.hwnd} value={item.index}>
            {item.title} - {item.width}x{item.height}
          </option>
        ))}
      </select>
      <button
        type="button"
        className="target-refresh-btn"
        aria-label={t("capture.reloadWindows")}
        title={t("capture.reloadWindows")}
        disabled={isLoading || pipelineControlled}
        onClick={() => void fetchWindows()}
      >
        ↻
      </button>
    </>
  );
}

export default function TargetSelect() {
  const { t } = useTranslation();
  const mode = usePreferencesStore((state) => state.mode);
  return (
    <div className="target-select">
      <div className="target-filter">
        <label>{t("capture.target")}</label>
        {mode === "screen" ? <ScreenTargetSelect /> : <WindowTargetSelect />}
      </div>
    </div>
  );
}
