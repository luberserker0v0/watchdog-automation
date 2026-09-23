import { useEffect } from "react";
import { useDesktopStore } from "../stores/useDesktopStore";
import { useWindowStore } from "../stores/useWindowStore";
import { useStreamStore } from "../stores/useStreamStore";
import { useTranslation } from "react-i18next";

export default function DesktopFilter() {
  const { t } = useTranslation();
  const desktops = useDesktopStore((s) => s.desktops);
  const selectedDesktopId = useDesktopStore((s) => s.selectedDesktopId);
  const isLoading = useDesktopStore((s) => s.isLoading);
  const fetchDesktops = useDesktopStore((s) => s.fetchDesktops);
  const setSelectedDesktop = useDesktopStore((s) => s.setSelectedDesktop);
  const fetchWindows = useWindowStore((s) => s.fetchWindows);
  const pipelineControlled = useStreamStore((state) => state.captureOwner === "pipeline");

  const vdSupported = !(desktops.length === 1 && desktops[0].id === "default");
  const currentDesktop = desktops.find((d) => d.is_current);

  useEffect(() => {
    fetchDesktops();
  }, [fetchDesktops]);

  if (!vdSupported) {
    return (
      <div className="desktop-filter">
        <label>{t("capture.desktop")}</label>
        <select value="" disabled>
          <option value="">{t("common.current")}</option>
        </select>
        {isLoading && <span className="desktop-loading">{t("capture.loading")}</span>}
      </div>
    );
  }

  const currentValue = selectedDesktopId === currentDesktop?.id ? "" : selectedDesktopId ?? "";

  return (
    <div className="desktop-filter">
      <label>{t("capture.desktop")}</label>
      <select
        value={currentValue}
        disabled={pipelineControlled}
        onChange={(e) => {
          setSelectedDesktop(e.target.value || null);
          void fetchWindows();
        }}
      >
        {currentDesktop && (
          <option value="">{currentDesktop.name || "Desktop 1"} ({t("common.current")})</option>
        )}
        {desktops
          .filter((d) => !d.is_current)
          .map((d) => (
            <option key={d.id} value={d.id}>
              {d.name || `Desktop ${d.index + 1}`}
            </option>
          ))}
      </select>
      {isLoading && <span className="desktop-loading">{t("capture.loading")}</span>}
    </div>
  );
}
