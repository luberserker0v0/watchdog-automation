import { usePreferencesStore, type CaptureArea } from "../../shell/stores/usePreferencesStore";
import { useStreamStore } from "../stores/useStreamStore";
import { useTranslation } from "react-i18next";

export default function CaptureAreaSelector() {
  const { t } = useTranslation();
  const value = usePreferencesStore((state) => state.windowCaptureArea);
  const setValue = usePreferencesStore((state) => state.setWindowCaptureArea);
  const isStreaming = useStreamStore((state) => state.isStreaming);
  const pipelineControlled = useStreamStore((state) => state.captureOwner === "pipeline");

  return (
    <label className="capture-area-selector">
      <span>{t("capture.area")}</span>
      <select
        aria-label={t("capture.area")}
        value={value}
        disabled={isStreaming || pipelineControlled}
        title={pipelineControlled ? "Pipeline controls capture" : isStreaming ? "Stop streaming before changing capture area" : undefined}
        onChange={(event) => setValue(event.target.value as CaptureArea)}
      >
        <option value="frame">{t("capture.frame")}</option>
        <option value="client">{t("capture.client")}</option>
      </select>
    </label>
  );
}
