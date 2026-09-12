import { usePreferencesStore } from "../../core/stores/usePreferencesStore";

export default function ModeSelector() {
  const mode = usePreferencesStore((s) => s.mode);
  const setMode = usePreferencesStore((s) => s.setMode);

  return (
    <div className="mode-selector">
      <button
        className={mode === "screen" ? "active" : ""}
        onClick={() => setMode("screen")}
      >
        螢幕截圖
      </button>
      <button
        className={mode === "window" ? "active" : ""}
        onClick={() => setMode("window")}
      >
        視窗截圖
      </button>
    </div>
  );
}
