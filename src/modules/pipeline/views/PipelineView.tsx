import { useCallback, useEffect, useRef, useState } from "react";
import { useShallow } from "zustand/react/shallow";
import { listPipelines, type PipelineInfo } from "../../../api/pipeline";
import MermaidGraph from "../components/MermaidGraph";
import PipelineCard from "../components/PipelineCard";
import PipelinePreviewModal from "../components/PipelinePreviewModal";
import {
  selectDisplayedPipelineId,
  usePipelineStore,
} from "../stores/usePipelineStore";
import "../styles/pipeline.css";
import "../styles/testbed.css";

function formatTime(ms: number) {
  const seconds = Math.floor(ms / 1000);
  const minutes = Math.floor(seconds / 60);
  return minutes > 0 ? `${minutes}m ${seconds % 60}s` : `${seconds}s`;
}

function PipelineExecutionHeader() {
  const summary = usePipelineStore(useShallow((state) => {
    const entries = state.pipelineIds.map((id) => state.pipelinesById[id]);
    const running = entries.find((entry) => entry.status === "running");
    return {
      isExecuting: state.isExecuting,
      completed: entries.filter((entry) => entry.status === "completed").length,
      total: entries.length,
      checked: entries.filter((entry) => entry.isChecked).length,
      elapsedMs: running?.elapsedMs ?? 0,
    };
  }));
  const start = usePipelineStore((state) => state.startExecution);
  const pause = usePipelineStore((state) => state.pauseExecution);
  const stop = usePipelineStore((state) => state.stopExecution);

  return (
    <div className="pipeline-header">
      <h2>Pipeline</h2>
      <div className="pipeline-execution-controls">
        <button className="pipeline-btn pipeline-btn-primary" onClick={start} disabled={summary.isExecuting || summary.checked === 0}>▶ Start</button>
        <button className="pipeline-btn pipeline-btn-secondary" onClick={pause} disabled={!summary.isExecuting}>⏸ Pause</button>
        <button className="pipeline-btn pipeline-btn-danger" onClick={stop} disabled={!summary.isExecuting}>■ Stop</button>
        <span className="pipeline-status-text">
          {summary.isExecuting
            ? `${summary.completed}/${summary.total}  ${formatTime(summary.elapsedMs)}`
            : `${summary.completed}/${summary.total} completed`}
        </span>
      </div>
    </div>
  );
}

function PipelinePicker() {
  const [available, setAvailable] = useState<PipelineInfo[]>([]);
  const paths = usePipelineStore(useShallow((state) =>
    state.pipelineIds.map((id) => state.pipelinesById[id].path),
  ));
  const addPipeline = usePipelineStore((state) => state.addPipeline);

  useEffect(() => {
    let active = true;
    listPipelines().then((items) => {
      if (active) setAvailable(items);
    }).catch(() => undefined);
    return () => { active = false; };
  }, []);

  return (
    <div className="pipeline-add-row">
      <select
        className="pipeline-add-select"
        value=""
        onChange={(event) => {
          const path = event.target.value;
          if (path) void addPipeline(path).catch((error) => console.error("Failed to add pipeline:", error));
        }}
      >
        <option value="">+ Add pipeline...</option>
        {available.filter((pipeline) => !paths.includes(pipeline.path)).map((pipeline) => (
          <option key={pipeline.path} value={pipeline.path}>
            {pipeline.source === "built-in" ? "🔧 " : "📁 "}{pipeline.name}
          </option>
        ))}
      </select>
    </div>
  );
}

function PipelineList({ onPreview }: { onPreview: (id: string) => void }) {
  const ids = usePipelineStore((state) => state.pipelineIds);
  return (
    <>
      {ids.map((id) => <PipelineCard key={id} id={id} onPreview={onPreview} />)}
      <PipelinePicker />
    </>
  );
}

function PipelineGraphPanel() {
  const displayedId = usePipelineStore(selectDisplayedPipelineId);
  const graph = usePipelineStore(useShallow((state) => {
    const entry = displayedId ? state.pipelinesById[displayedId] : undefined;
    return { mermaid: entry?.mermaid ?? "", currentNodeId: entry?.currentNodeId ?? null };
  }));
  if (!graph.mermaid) return <div className="pipeline-main" />;
  return (
    <div className="pipeline-main">
      <div className="pipeline-graph-panel">
        <MermaidGraph mermaid={graph.mermaid} currentNodeId={graph.currentNodeId} selectedNodeId={null} onNodeClick={() => undefined} />
      </div>
    </div>
  );
}

function Preview({ id, onClose }: { id: string; onClose: () => void }) {
  const preview = usePipelineStore(useShallow((state) => {
    const entry = state.pipelinesById[id];
    return { name: entry?.name ?? "", mermaid: entry?.mermaid ?? "" };
  }));
  if (!preview.mermaid) return null;
  return <PipelinePreviewModal name={preview.name} mermaid={preview.mermaid} onClose={onClose} />;
}

export default function PipelineView() {
  const [previewId, setPreviewId] = useState<string | null>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const dragCleanupRef = useRef<(() => void) | null>(null);
  const showPreview = useCallback((id: string) => setPreviewId(id), []);

  useEffect(() => () => dragCleanupRef.current?.(), []);

  const startResize = useCallback((event: React.MouseEvent) => {
    event.preventDefault();
    const list = listRef.current;
    if (!list) return;
    const startY = event.clientY;
    const startHeight = list.getBoundingClientRect().height;
    const move = (moveEvent: MouseEvent) => {
      list.style.height = `${Math.max(60, Math.min(300, startHeight + moveEvent.clientY - startY))}px`;
    };
    const finish = () => {
      document.removeEventListener("mousemove", move);
      document.removeEventListener("mouseup", finish);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
      dragCleanupRef.current = null;
    };
    dragCleanupRef.current?.();
    dragCleanupRef.current = finish;
    document.addEventListener("mousemove", move);
    document.addEventListener("mouseup", finish);
    document.body.style.cursor = "ns-resize";
    document.body.style.userSelect = "none";
  }, []);

  return (
    <div className="pipeline-view">
      <PipelineExecutionHeader />
      <div className="pipeline-list" ref={listRef} style={{ height: 120 }}>
        <PipelineList onPreview={showPreview} />
      </div>
      <div className="drag-handle" onMouseDown={startResize}><div className="drag-handle-grip" /></div>
      <PipelineGraphPanel />
      {previewId && <Preview id={previewId} onClose={() => setPreviewId(null)} />}
    </div>
  );
}
