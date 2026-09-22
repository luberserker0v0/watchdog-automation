import { useCallback, useEffect, useRef, useState } from "react";
import { useLogStore } from "../../../capture/stores/useLogStore";

export default function ExecutionLog() {
  const [collapsed, setCollapsed] = useState(false);
  const heightRef = useRef(200);
  const containerRef = useRef<HTMLDivElement>(null);
  const scrollRef = useRef<HTMLDivElement>(null);
  const dragCleanupRef = useRef<(() => void) | null>(null);
  const clearLogs = useLogStore((s) => s.clearLogs);
  const entryCount = useLogStore((s) => s.entries.length);

  useEffect(() => {
    if (scrollRef.current) scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
  }, [entryCount]);

  useEffect(() => () => dragCleanupRef.current?.(), []);

  const handleDragStart = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    const startY = e.clientY;
    const container = containerRef.current;
    if (!container) return;
    const startHeight = container.getBoundingClientRect().height;

    const onMouseMove = (ev: MouseEvent) => {
      const delta = startY - ev.clientY;
      heightRef.current = Math.max(80, Math.min(600, startHeight + delta));
      container.style.height = `${heightRef.current}px`;
    };

    const onMouseUp = () => {
      document.removeEventListener("mousemove", onMouseMove);
      document.removeEventListener("mouseup", onMouseUp);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
      dragCleanupRef.current = null;
    };

    dragCleanupRef.current?.();
    dragCleanupRef.current = onMouseUp;
    document.addEventListener("mousemove", onMouseMove);
    document.addEventListener("mouseup", onMouseUp);
    document.body.style.cursor = "ns-resize";
    document.body.style.userSelect = "none";
  }, []);

  return (
    <>
      <div className="drag-handle" onMouseDown={handleDragStart}>
        <div className="drag-handle-grip" />
      </div>

      <div
        ref={containerRef}
        className="execution-log"
        style={{ height: collapsed ? 32 : heightRef.current }}
      >
        <div className="execution-log-header">
          <span className="execution-log-title">
            Log {entryCount > 0 && `(${entryCount})`}
          </span>
          <div className="execution-log-actions">
            <button
              className="execution-log-action"
              onClick={() => clearLogs()}
            >
              Clear
            </button>
            <button
              className="execution-log-action"
              onClick={() => setCollapsed(!collapsed)}
            >
              {collapsed ? "Expand" : "Hide"}
            </button>
          </div>
        </div>
        {!collapsed && (
          <>
            {entryCount === 0 ? (
              <div className="execution-log-empty">No execution log yet</div>
            ) : (
              <div className="execution-log-entries" ref={scrollRef}>
                <LogEntries />
              </div>
            )}
          </>
        )}
      </div>
    </>
  );
}

function LogEntries() {
  const entries = useLogStore((s) => s.entries);
  return <>{entries.map((entry) => (
    <div key={entry.id} className={`execution-log-entry ${entry.source} ${entry.result || ""}`}>
      <span className="execution-log-icon">{entry.result === "error" ? "✗" : entry.result === "success" ? "✓" : "ℹ"}</span>
      <span className="execution-log-pipeline">{entry.label}</span>
      {entry.tag && (
        <span className={`execution-log-tag ${entry.tag}`}>
          {entry.tag === "hint" ? "HINT ONLY" : "REDIRECT"}
        </span>
      )}
      <span className="execution-log-node">{entry.source}</span>
      <span className="execution-log-detail">{entry.detail}</span>
    </div>
  ))}</>;
}
