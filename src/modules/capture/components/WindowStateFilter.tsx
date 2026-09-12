import { useWindowStore } from "../stores/useWindowStore";

export default function WindowStateFilter() {
  const windowStateFilter = useWindowStore((s) => s.windowStateFilter);
  const setWindowStateFilter = useWindowStore((s) => s.setWindowStateFilter);

  return (
    <div className="window-state-filter">
      <label>狀態</label>
      <div className="window-state-buttons">
        <button
          className={windowStateFilter === "Foreground" ? "active" : ""}
          onClick={() => setWindowStateFilter("Foreground")}
        >
          前景
        </button>
      </div>
    </div>
  );
}
