import { usePreferencesStore } from "../../shell/stores/usePreferencesStore";
import { useScreenStore } from "../stores/useScreenStore";
import { useWindowStore } from "../stores/useWindowStore";
import { useStreamStore } from "../stores/useStreamStore";

function ScreenTargetSelect() {
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
        <option value="">{isLoading ? "載入中..." : "選擇目標"}</option>
        {items.map((item) => (
          <option key={item.index} value={item.index}>
            Screen {item.index} - {item.width}x{item.height}{item.is_primary ? " (主螢幕)" : ""}
          </option>
        ))}
      </select>
      <button
        type="button"
        className="target-refresh-btn"
        aria-label="重新載入螢幕"
        title="重新載入螢幕"
        disabled={isLoading || pipelineControlled}
        onClick={() => void fetchScreens()}
      >
        ↻
      </button>
    </>
  );
}

function WindowTargetSelect() {
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
        <option value="">{isLoading ? "載入中..." : "選擇目標"}</option>
        {items.map((item) => (
          <option key={item.hwnd} value={item.index}>
            {item.title} - {item.width}x{item.height}
          </option>
        ))}
      </select>
      <button
        type="button"
        className="target-refresh-btn"
        aria-label="重新載入視窗"
        title="重新載入視窗（套用目前篩選）"
        disabled={isLoading || pipelineControlled}
        onClick={() => void fetchWindows()}
      >
        ↻
      </button>
    </>
  );
}

export default function TargetSelect() {
  const mode = usePreferencesStore((state) => state.mode);
  return (
    <div className="target-select">
      <div className="target-filter">
        <label>目標</label>
        {mode === "screen" ? <ScreenTargetSelect /> : <WindowTargetSelect />}
      </div>
    </div>
  );
}
