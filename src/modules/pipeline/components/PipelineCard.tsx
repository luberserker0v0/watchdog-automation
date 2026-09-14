import { memo, useCallback, useRef, useState } from "react";
import { useShallow } from "zustand/react/shallow";
import ParamsForm from "./ParamsForm";
import TargetWindowButton from "./target/TargetWindowButton";
import { usePipelineStore } from "../stores/usePipelineStore";

interface PipelineCardProps {
  id: string;
  onPreview: (id: string) => void;
}

function PipelineCard({ id, onPreview }: PipelineCardProps) {
  const entry = usePipelineStore(useShallow((state) => {
    const pipeline = state.pipelinesById[id];
    return pipeline ? {
      path: pipeline.path,
      name: pipeline.name,
      status: pipeline.status,
      isChecked: pipeline.isChecked,
      collectedParams: pipeline.collectedParams,
      userParams: pipeline.userParams,
      error: pipeline.error,
    } : null;
  }));
  const isSelected = usePipelineStore((state) => state.selectedId === id);
  const isLocked = usePipelineStore((state) => state.batchIds.includes(id));
  const resizeIssue = usePipelineStore((state) => state.resizeIssue?.pipelineId === id ? state.resizeIssue : null);
  const selectPipeline = usePipelineStore((state) => state.selectPipeline);
  const toggleCheck = usePipelineStore((state) => state.toggleCheck);
  const removePipeline = usePipelineStore((state) => state.removePipeline);
  const updateParams = usePipelineStore((state) => state.updateParams);
  const retryWindowSize = usePipelineStore((state) => state.retryWindowSize);
  const cancelWindowSize = usePipelineStore((state) => state.cancelWindowSize);
  const [expanded, setExpanded] = useState(false);
  const cardRef = useRef<HTMLDivElement>(null);
  const handleParamChange = useCallback(
    (key: string, value: string | number | boolean) => updateParams(id, key, value),
    [id, updateParams],
  );

  if (!entry) return null;
  const isRunning = entry.status === "running";

  return (
    <div ref={cardRef} className={`pipeline-card ${isSelected ? "selected" : ""} ${isRunning ? "running" : ""}`}>
      <div className="pipeline-card-row" onClick={() => selectPipeline(id)}>
        <input
          type="checkbox"
          className="pipeline-card-checkbox"
          checked={entry.isChecked}
          disabled={isLocked}
          onChange={(event) => {
            event.stopPropagation();
            toggleCheck(id);
          }}
          onClick={(event) => event.stopPropagation()}
        />
        <span className="pipeline-card-icon">📄</span>
        <span className="pipeline-card-name">{entry.name}</span>
        {["running", "paused", "stopping", "failed", "stopped", "needs_resize"].includes(entry.status) && (
          <span className={`pipeline-card-status ${entry.status}`} title={entry.error ?? undefined}>{entry.status}</span>
        )}
        <button
          className="pipeline-card-preview"
          onClick={(event) => {
            event.stopPropagation();
            onPreview(id);
          }}
          title="Preview graph"
        >
          👁
        </button>
        <button
          className="pipeline-card-toggle"
          onClick={(event) => {
            event.stopPropagation();
            setExpanded((value) => {
              const next = !value;
              if (next) {
                requestAnimationFrame(() => cardRef.current?.scrollIntoView?.({ block: "nearest" }));
              }
              return next;
            });
          }}
          aria-expanded={expanded}
          title={expanded ? "Hide pipeline settings" : "Show pipeline settings"}
        >
          {expanded ? "▴" : "▾"}
        </button>
        <button
          className="pipeline-card-remove"
          disabled={isLocked}
          onClick={(event) => {
            event.stopPropagation();
            removePipeline(id);
          }}
          title="Remove pipeline"
        >
          ✕
        </button>
      </div>

      {resizeIssue && (
        <div className="pipeline-window-size-warning" role="alert">
          <strong>Resize target application</strong>
          {resizeIssue.title && <span>Target: {resizeIssue.title}</span>}
          <span>
            Expected content size {resizeIssue.expected[0]} × {resizeIssue.expected[1]}; current size {resizeIssue.before[0]} × {resizeIssue.before[1]}.
          </span>
          {(resizeIssue.actual[0] !== resizeIssue.before[0] || resizeIssue.actual[1] !== resizeIssue.before[1]) && (
            <span>Automatic resizing reached {resizeIssue.actual[0]} × {resizeIssue.actual[1]} before the original placement was restored.</span>
          )}
          <span>{resizeIssue.diagnostic}</span>
          <div>
            <button onClick={() => void retryWindowSize()}>Retry</button>
            <button onClick={cancelWindowSize}>Cancel</button>
          </div>
        </div>
      )}

      {expanded && (
        <div className="pipeline-card-panel">
          {entry.collectedParams.length > 0 && (
            <ParamsForm
              params={entry.collectedParams}
              values={entry.userParams}
              onChange={handleParamChange}
              disabled={isLocked}
            />
          )}
          {entry.path.includes("sample") && <TargetWindowButton />}
        </div>
      )}
    </div>
  );
}

export default memo(PipelineCard);
