import { useWindowStore } from "../stores/useWindowStore";
import { useStreamStore } from "../stores/useStreamStore";

export default function WindowStateFilter() {
  const windowStateFilter = useWindowStore((s) => s.windowStateFilter);
  const setWindowStateFilter = useWindowStore((s) => s.setWindowStateFilter);
  const pipelineControlled = useStreamStore((state) => state.captureOwner === "pipeline");

  return (
    <div className="window-state-filter">
      <label>狀態</label>
      <div className="window-state-buttons">
        <button
          className={windowStateFilter === "Foreground" ? "active" : ""}
          disabled={pipelineControlled}
          onClick={() => setWindowStateFilter("Foreground")}
        >
          前景
        </button>
      </div>
    </div>
  );
}
