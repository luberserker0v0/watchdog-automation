import { memo, useCallback, useState } from "react";
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
    } : null;
  }));
  const isSelected = usePipelineStore((state) => state.selectedId === id);
  const selectPipeline = usePipelineStore((state) => state.selectPipeline);
  const toggleCheck = usePipelineStore((state) => state.toggleCheck);
  const removePipeline = usePipelineStore((state) => state.removePipeline);
  const updateParams = usePipelineStore((state) => state.updateParams);
  const [expanded, setExpanded] = useState(false);
  const handleParamChange = useCallback(
    (key: string, value: string | number | boolean) => updateParams(id, key, value),
    [id, updateParams],
  );

  if (!entry) return null;
  const isRunning = entry.status === "running";

  return (
    <div className={`pipeline-card ${isSelected ? "selected" : ""} ${isRunning ? "running" : ""}`}>
      <div className="pipeline-card-row" onClick={() => selectPipeline(id)}>
        <input
          type="checkbox"
          className="pipeline-card-checkbox"
          checked={entry.isChecked}
          onChange={(event) => {
            event.stopPropagation();
            toggleCheck(id);
          }}
          onClick={(event) => event.stopPropagation()}
        />
        <span className="pipeline-card-icon">📄</span>
        <span className="pipeline-card-name">{entry.name}</span>
        {isRunning && <span className="pipeline-card-status running">running</span>}
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
            setExpanded((value) => !value);
          }}
        >
          {expanded ? "▴" : "▾"}
        </button>
        <button
          className="pipeline-card-remove"
          onClick={(event) => {
            event.stopPropagation();
            removePipeline(id);
          }}
          title="Remove pipeline"
        >
          ✕
        </button>
      </div>

      {expanded && (
        <div className="pipeline-card-panel">
          {entry.collectedParams.length > 0 && (
            <ParamsForm
              params={entry.collectedParams}
              values={entry.userParams}
              onChange={handleParamChange}
            />
          )}
          {entry.path.includes("sample") && <TargetWindowButton />}
        </div>
      )}
    </div>
  );
}

export default memo(PipelineCard);
