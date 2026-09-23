import { useWindowStore } from "../stores/useWindowStore";
import { useStreamStore } from "../stores/useStreamStore";
import { useTranslation } from "react-i18next";

export default function WindowStateFilter() {
  const { t } = useTranslation();
  const windowStateFilter = useWindowStore((s) => s.windowStateFilter);
  const setWindowStateFilter = useWindowStore((s) => s.setWindowStateFilter);
  const pipelineControlled = useStreamStore((state) => state.captureOwner === "pipeline");

  return (
    <div className="window-state-filter">
      <label>{t("capture.state")}</label>
      <div className="window-state-buttons">
        <button
          className={windowStateFilter === "Foreground" ? "active" : ""}
          disabled={pipelineControlled}
          onClick={() => setWindowStateFilter("Foreground")}
        >
          {t("capture.foreground")}
        </button>
      </div>
    </div>
  );
}
