import { useCallback, useEffect, useState } from "react";
import { useShallow } from "zustand/react/shallow";
import { useTranslation } from "react-i18next";
import { listPipelines, type PipelineInfo } from "../../../../api/pipeline";
import MermaidGraph from "../../authoring/components/MermaidGraph";
import NodeDetailModal from "../components/NodeDetailModal";
import PipelineCard from "../components/PipelineCard";
import PipelinePreviewModal from "../../authoring/components/PipelinePreviewModal";
import {
  selectDisplayedPipelineId,
  usePipelineStore,
} from "../stores/usePipelineStore";
import "../../styles/pipeline.css";
import "../../styles/testbed.css";

function formatTime(ms: number) {
  const seconds = Math.floor(ms / 1000);
  const minutes = Math.floor(seconds / 60);
  return minutes > 0 ? `${minutes}m ${seconds % 60}s` : `${seconds}s`;
}

function PipelineExecutionHeader() {
  const { t } = useTranslation();
  const summary = usePipelineStore(useShallow((state) => {
    const entries = state.pipelineIds.map((id) => state.pipelinesById[id]);
    const active = state.activeId ? state.pipelinesById[state.activeId] : undefined;
    return {
      isExecuting: state.isExecuting,
      completed: entries.filter((entry) => entry.status === "completed").length,
      total: entries.length,
      checked: entries.filter((entry) => entry.isChecked).length,
      elapsedMs: active?.elapsedMs ?? 0,
      activeStatus: state.activeId ? state.pipelinesById[state.activeId]?.status : undefined,
    };
  }));
  const start = usePipelineStore((state) => state.startExecution);
  const pause = usePipelineStore((state) => state.pauseExecution);
  const stop = usePipelineStore((state) => state.stopExecution);

  return (
    <div className="pipeline-header">
      <h2>{t("pipeline.title")}</h2>
      <div className="pipeline-execution-controls">
        <button className="pipeline-btn pipeline-btn-primary" onClick={start} disabled={summary.isExecuting || summary.checked === 0}>▶ {t("common.start")}</button>
        <button className="pipeline-btn pipeline-btn-secondary" onClick={pause} disabled={!summary.isExecuting || summary.activeStatus === "stopping"}>
          {summary.activeStatus === "paused" ? `▶ ${t("common.resume")}` : `⏸ ${t("common.pause")}`}
        </button>
        <button className="pipeline-btn pipeline-btn-danger" onClick={stop} disabled={!summary.isExecuting || summary.activeStatus === "stopping"}>■ {t("common.stop")}</button>
        <span className="pipeline-status-text">
          {summary.isExecuting
            ? `${summary.completed}/${summary.total}  ${formatTime(summary.elapsedMs)}`
            : t("pipeline.completed", { completed: summary.completed, total: summary.total })}
        </span>
      </div>
    </div>
  );
}

function PipelinePicker() {
  const { t } = useTranslation();
  const [available, setAvailable] = useState<PipelineInfo[]>([]);
  const [listError, setListError] = useState<string | null>(null);
  const [addError, setAddError] = useState<string | null>(null);
  const paths = usePipelineStore(useShallow((state) =>
    state.pipelineIds.map((id) => state.pipelinesById[id].path),
  ));
  const addPipeline = usePipelineStore((state) => state.addPipeline);

  useEffect(() => {
    let active = true;
    listPipelines().then((items) => {
      if (active) { setAvailable(items); setListError(null); }
    }).catch((error) => {
      if (active) setListError(String(error));
    });
    return () => { active = false; };
  }, []);

  return (
    <div className="pipeline-add-row">
      {listError && (
        <div className="pipeline-error-banner">{listError}</div>
      )}
      {addError && (
        <div className="pipeline-error-banner">{addError}</div>
      )}
      <select
        className="pipeline-add-select"
        value=""
        onChange={(event) => {
          const path = event.target.value;
          if (path) {
            setAddError(null);
            void addPipeline(path).catch((error) => setAddError(String(error)));
          }
        }}
      >
        <option value="">{t("pipeline.add")}</option>
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

const EMPTY_HIT_COUNTS: Record<string, number> = {};

function PipelineGraphPanel({ onCollapse }: { onCollapse: () => void }) {
  const { t } = useTranslation();
  const [focusMode, setFocusMode] = useState(false);
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [detailNodeId, setDetailNodeId] = useState<string | null>(null);
  const displayedId = usePipelineStore(selectDisplayedPipelineId);
  const graph = usePipelineStore(useShallow((state) => {
    const entry = displayedId ? state.pipelinesById[displayedId] : undefined;
    return {
      name: entry?.name ?? "Pipeline graph",
      path: entry?.path ?? "",
      mermaid: entry?.mermaid ?? "",
      flow: entry?.graph ?? null,
      currentNodeId: entry?.currentNodeId ?? null,
      layoutWritable: entry?.layoutWritable ?? false,
      hitCounts: entry?.hitCounts ?? EMPTY_HIT_COUNTS,
      nodeElapsedMs: entry?.nodeElapsedMs ?? 0,
      childrenParentId: entry?.childrenParentId ?? null,
      childrenStartTime: entry?.childrenStartTime ?? null,
      childrenChildId: entry?.childrenChildId ?? null,
      childrenTimeoutMs: entry?.childrenTimeoutMs ?? null,
    };
  }));
  const detailNode = detailNodeId ? graph.flow?.nodes.find((n) => n.id === detailNodeId) : undefined;
  const detailLastStep = usePipelineStore(useShallow((state) => {
    if (!displayedId || !detailNodeId) return undefined;
    const entry = state.pipelinesById[displayedId];
    const results = entry?.lastStepResults;
    if (!results) return undefined;
    for (let i = results.length - 1; i >= 0; i--) {
      if (results[i].node_id === detailNodeId) return results[i];
    }
    return undefined;
  }));
  return (
    <section className="pipeline-graph-pane" aria-label={t("pipeline.graph")}>
      <div className="pipeline-pane-header">
        <div className="pipeline-pane-title">
          <span className="pipeline-pane-eyebrow">{t("pipeline.graph")}</span>
          <strong title={graph.name}>{graph.name}</strong>
        </div>
        <div className="pipeline-pane-actions">
          <label className="pipeline-focus-switch" title="Keep the current running node centered">
            <input
              type="checkbox"
              role="switch"
              checked={focusMode}
              onChange={(event) => setFocusMode(event.target.checked)}
            />
            <span className="pipeline-focus-toggle" aria-hidden="true"><span /></span>
            <span className="pipeline-focus-label">{t("pipeline.focus")}</span>
          </label>
          <button type="button" className="pipeline-pane-action" onClick={onCollapse} title={t("pipeline.hideGraph")}>
            {t("pipeline.hideGraph")}
          </button>
        </div>
      </div>
      <div className="pipeline-graph-panel">
        {graph.mermaid ? (
        <MermaidGraph
          mermaid={graph.mermaid}
          flow={graph.flow}
          currentNodeId={graph.currentNodeId}
          selectedNodeId={selectedNodeId}
          onNodeClick={setSelectedNodeId}
          onNodeDoubleClick={setDetailNodeId}
          focusCurrentNode={focusMode}
          layoutKey={graph.path}
          allowNodeLayout
          layoutWritable={graph.layoutWritable}
          hitCounts={graph.hitCounts}
          nodeTimeoutProgress={graph.currentNodeId ? (() => {
            const parentId = graph.childrenParentId;
            if (parentId) {
              // Active quota comes from the backend (parent timeout for
              // normal children, parent error_timeout for error targets).
              const timeout = graph.childrenTimeoutMs ?? 20000;
              const elapsed = graph.childrenStartTime ? Date.now() - graph.childrenStartTime : 0;
              return Math.min(elapsed / timeout, 1);
            }
            const nodeDef = graph.flow?.nodes.find((n) => n.id === graph.currentNodeId);
            const timeout = nodeDef?.timeout ?? 20000;
            return Math.min(graph.nodeElapsedMs / timeout, 1);
          })() : undefined}
          childrenParentId={graph.childrenParentId}
          childrenChildId={graph.childrenChildId}
        />
        ) : (
          <div className="pipeline-graph-empty">{t("pipeline.emptyGraph")}</div>
        )}
        {detailNode && (
          <NodeDetailModal
            node={detailNode}
            hitCount={graph.hitCounts[detailNode.id] ?? 0}
            lastStep={detailLastStep}
            onClose={() => setDetailNodeId(null)}
          />
        )}
      </div>
    </section>
  );
}

function Preview({ id, onClose }: { id: string; onClose: () => void }) {
  const preview = usePipelineStore(useShallow((state) => {
    const entry = state.pipelinesById[id];
    return { name: entry?.name ?? "", mermaid: entry?.mermaid ?? "", graph: entry?.graph ?? null };
  }));
  if (!preview.mermaid) return null;
  return <PipelinePreviewModal name={preview.name} mermaid={preview.mermaid} graph={preview.graph} onClose={onClose} />;
}

export default function PipelineView() {
  const { t } = useTranslation();
  const [previewId, setPreviewId] = useState<string | null>(null);
  const [graphCollapsed, setGraphCollapsed] = useState(false);
  const pipelineCount = usePipelineStore((state) => state.pipelineIds.length);
  const showPreview = useCallback((id: string) => setPreviewId(id), []);

  return (
    <div className="pipeline-view">
      <PipelineExecutionHeader />
      <div className={`pipeline-workspace ${graphCollapsed ? "graph-collapsed" : ""}`}>
        <section className="pipeline-browser-pane" aria-label={t("pipeline.pipelines")}>
          <div className="pipeline-pane-header">
            <div className="pipeline-pane-title">
              <span className="pipeline-pane-eyebrow">{t("pipeline.pipelines")}</span>
              <strong>{t("pipeline.loaded", { count: pipelineCount })}</strong>
            </div>
            {graphCollapsed && (
              <button type="button" className="pipeline-pane-action" onClick={() => setGraphCollapsed(false)}>
                {t("pipeline.showGraph")}
              </button>
            )}
          </div>
          <div className="pipeline-list">
            <PipelineList onPreview={showPreview} />
          </div>
        </section>
        {!graphCollapsed && <PipelineGraphPanel onCollapse={() => setGraphCollapsed(true)} />}
      </div>
      {previewId && <Preview id={previewId} onClose={() => setPreviewId(null)} />}
    </div>
  );
}
