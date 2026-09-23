import { usePreferencesStore } from "../../shell/stores/usePreferencesStore";
import { useStreamStore } from "../stores/useStreamStore";
import { useTranslation } from "react-i18next";

export default function ModeSelector() {
  const { t } = useTranslation();
  const mode = usePreferencesStore((s) => s.mode);
  const setMode = usePreferencesStore((s) => s.setMode);
  const pipelineControlled = useStreamStore((state) => state.captureOwner === "pipeline");

  return (
    <div className="mode-selector">
      <button
        className={mode === "screen" ? "active" : ""}
        disabled={pipelineControlled}
        onClick={() => setMode("screen")}
      >
        {t("capture.screen")}
      </button>
      <button
        className={mode === "window" ? "active" : ""}
        disabled={pipelineControlled}
        onClick={() => setMode("window")}
      >
        {t("capture.window")}
      </button>
    </div>
  );
}
