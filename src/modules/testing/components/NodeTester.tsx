import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { listNodeMetas, openWindow, testNode, type NodeMetaInfo, type NodeParamValue } from "../../../api/nodeTest";
import { useLogStore } from "../../observability/stores/useLogStore";
import { usePreferencesStore } from "../../shell/stores/usePreferencesStore";
import type { Recognition, Action, PipelineTarget } from "../../../api/pipeline";
import ParamsForm from "../../pipeline/authoring/components/ParamsForm";
import { defaultNodeParams } from "../models/nodeTestParams";

const PAGE_ORDER = ["Login", "Dashboard", "Settings", "Screen", "Other"] as const;
const TIMING_LABELS: Record<string, string> = {
  target_resolve: "target",
  target_settle: "startup",
  stream_ready: "stream",
  frame_acquire: "frame",
  frame_decode: "decode",
  recognition: "recognition",
  coordinate: "coordinates",
  pre_delay: "pre-delay",
  pre_wait_freezes: "pre-stability",
  actions: "actions",
  post_delay: "post-delay",
  post_wait_freezes: "post-stability",
  debug_image: "debug image",
};

export function nodeTestPage(node: NodeMetaInfo): string {
  if (node.target?.type === "screen") return "Screen";
  const title = node.target?.title ?? "";
  if (title.includes("Login")) return "Login";
  if (title.includes("Dashboard")) return "Dashboard";
  if (title.includes("Settings")) return "Settings";
  return "Other";
}

function nodeTestKey(node: NodeMetaInfo): string {
  return `${node.source ?? "built-in"}::${node.file_name}::${node.id}`;
}

function formatTimingDetails(timings: Record<string, number>): string {
  return Object.entries(timings)
    .map(([phase, duration]) => `${TIMING_LABELS[phase] ?? phase} ${duration}ms`)
    .join(" · ");
}

function formatTarget(target: unknown): string {
  if (target === true) return "Recognized region center";
  if (target === false) return "None";
  if (Array.isArray(target)) {
    if (target.length === 2) return `(${target[0]}, ${target[1]})`;
    if (target.length === 4) return `(${target[0]}, ${target[1]}) ${target[2]}x${target[3]}`;
  }
  if (typeof target === "string") return `Node: ${target}`;
  return String(target);
}

function formatPipelineTarget(target: PipelineTarget): string {
  if (target.type === "screen") return `Screen ${target.index ?? 0}`;
  const parts: string[] = [];
  if (target.process) parts.push(`process="${target.process}"`);
  if (target.class) parts.push(`class="${target.class}"`);
  if (target.title) parts.push(`title="${target.title}"`);
  parts.push(`capture_area="${target.capture_area ?? "frame"}"`);
  if (target.window_size) parts.push(`window_size=[${target.window_size.join(", ")}]`);
  return `Window { ${parts.join(", ")} }`;
}

function formatRecognition(recognition: Recognition): string {
  switch (recognition.type) {
    case "DirectHit": return "DirectHit";
    case "TemplateMatch": return `TemplateMatch (template="${recognition.template}", threshold=${recognition.threshold ?? 0.7})`;
    case "OCR": return `OCR (expected="${recognition.expected ?? ""}", threshold=${recognition.threshold ?? 0.6})`;
    case "ColorMatch": return `ColorMatch (lower=[${recognition.lower}], upper=[${recognition.upper}])`;
    case "Custom": return `Custom (name="${recognition.recognition}")`;
  }
}

function formatAction(action: Action): string {
  const offset = (value?: [number, number, number, number]) => value
    ? `, offset=[${value.join(", ")}]`
    : "";
  switch (action.type) {
    case "DoNothing": return "DoNothing";
    case "Click": return `Click (target=${formatTarget(action.target ?? true)}${offset(action.target_offset)})`;
    case "LongPress": return `LongPress (target=${formatTarget(action.target ?? true)}${offset(action.target_offset)}, duration=${action.duration ?? 1000}ms)`;
    case "Swipe": return `Swipe (begin=${formatTarget(action.begin ?? true)}${offset(action.begin_offset)}, end=${formatTarget(action.end ?? true)}${offset(action.end_offset)}, duration=${action.duration ?? 200}ms)`;
    case "Scroll": return `Scroll (target=${formatTarget(action.target ?? true)}${offset(action.target_offset)}, dx=${action.dx ?? 0}, dy=${action.dy ?? 0})`;
    case "ClickKey": return `ClickKey (keys=[${action.key.join(", ")}])`;
    case "InputText": return `InputText (text="${action.input_text}")`;
    case "StopTask": return "StopTask";
    case "Custom": return `Custom (name="${action.action}")`;
  }
}

export default function NodeTester() {
  const [nodeMetas, setNodeMetas] = useState<NodeMetaInfo[]>([]);
  const [selectedNodeKey, setSelectedNodeKey] = useState<string>("");
  const [selectedNode, setSelectedNode] = useState<NodeMetaInfo | null>(null);
  const [paramValues, setParamValues] = useState<Record<string, NodeParamValue>>({});
  const [loading, setLoading] = useState(false);
  const [testing, setTesting] = useState(false);
  const listRequest = useRef(0);
  const appendLog = useLogStore((s) => s.appendLog);
  const groupedNodes = useMemo(() => {
    const groups = new Map<string, NodeMetaInfo[]>();
    for (const node of nodeMetas) {
      const page = nodeTestPage(node);
      groups.set(page, [...(groups.get(page) ?? []), node]);
    }
    return [...groups.entries()].sort(([left], [right]) => {
      const leftIndex = PAGE_ORDER.indexOf(left as typeof PAGE_ORDER[number]);
      const rightIndex = PAGE_ORDER.indexOf(right as typeof PAGE_ORDER[number]);
      return (leftIndex < 0 ? PAGE_ORDER.length : leftIndex)
        - (rightIndex < 0 ? PAGE_ORDER.length : rightIndex);
    });
  }, [nodeMetas]);

  const loadNodeMetas = useCallback(async () => {
    const request = ++listRequest.current;
    setLoading(true);
    try {
      const metas = await listNodeMetas();
      if (request !== listRequest.current) return;
      setNodeMetas(metas);
      setSelectedNodeKey((current) => current || (metas[0] ? nodeTestKey(metas[0]) : ""));
    } catch (error) {
      appendLog({ source: "debug", label: "NodeTest", detail: `Failed to load node list: ${error}`, result: "error" });
    } finally {
      if (request === listRequest.current) setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadNodeMetas();
    return () => { listRequest.current++; };
  }, [loadNodeMetas]);

  useEffect(() => {
    const node = nodeMetas.find((item) => nodeTestKey(item) === selectedNodeKey) ?? null;
    setSelectedNode(node);
    setParamValues(node ? defaultNodeParams(node) : {});
  }, [nodeMetas, selectedNodeKey]);

  const handleOpenWindow = async () => {
    if (!selectedNode?.target) {
      appendLog({ source: "debug", label: "NodeTest", detail: `${selectedNode?.id ?? "Node"}: no target window configured`, result: "error" });
      return;
    }
    try {
      const msg = await openWindow(selectedNode.target);
      appendLog({ source: "debug", label: "NodeTest", detail: `${selectedNode.id}: ${msg}`, result: "success" });
    } catch (e) {
      appendLog({ source: "debug", label: "NodeTest", detail: `${selectedNode.id}: open window failed: ${e}`, result: "error" });
    }
  };

  const handleStartNode = async () => {
    if (!selectedNode) return;
    if (!selectedNode.target) {
      appendLog({ source: "debug", label: "NodeTest", detail: `${selectedNode.id}: no target window configured`, result: "error" });
      return;
    }
    setTesting(true);
    appendLog({ source: "debug", label: "NodeTest", detail: `${selectedNode.id}: testing...`, result: "info" });
    try {
      const timingGranularity = usePreferencesStore.getState().timingGranularity;
      const result = await testNode(
        selectedNode,
        usePreferencesStore.getState().saveDebugImages,
        paramValues,
        timingGranularity === "detailed",
      );
      if (result.matched) {
        appendLog({ source: "debug", label: "NodeTest", detail: `${selectedNode.id}: matched (${(result.confidence * 100).toFixed(1)}%)`, result: "success" });
      } else {
        appendLog({ source: "debug", label: "NodeTest", detail: `${selectedNode.id}: recognition failed (${(result.confidence * 100).toFixed(1)}%)`, result: "error" });
      }
      if (result.actions_fired > 0) {
        appendLog({ source: "debug", label: "NodeTest", detail: `${selectedNode.id}: ${result.actions_fired} action(s) fired`, result: "success" });
      }
      if (result.error) {
        appendLog({ source: "debug", label: "NodeTest", detail: `${selectedNode.id}: ${result.error}`, result: "error" });
      }
      if (result.selection_error) {
        appendLog({ source: "debug", label: "NodeTest", detail: result.selection_error, result: "error" });
      }
      if (result.raw_candidate_count !== undefined) {
        appendLog({
          source: "debug",
          label: "Template candidates",
          detail: `${result.raw_candidate_count} raw → ${result.distinct_candidate_count ?? result.candidates.length} distinct`,
          result: "info",
        });
      }
      for (const candidate of result.candidates ?? []) {
        appendLog({
          source: "debug",
          label: `Candidate #${candidate.rank}${candidate.selected ? " selected" : ""}`,
          detail: `${candidate.text ?? "candidate"} · ${(candidate.confidence * 100).toFixed(1)}% · (${candidate.region.x}, ${candidate.region.y}) ${candidate.region.width}×${candidate.region.height}`,
          result: candidate.selected ? "success" : "info",
        });
      }
      if (result.screen_click_position) {
        appendLog({ source: "debug", label: "Screen click", detail: `(${result.screen_click_position[0]}, ${result.screen_click_position[1]})`, result: "info" });
      }
      if (timingGranularity !== "off") {
        appendLog({
          source: "debug",
          label: "NodeTest",
          detail: `${selectedNode.id}: ${result.success ? "passed" : "failed"} in ${result.duration_ms}ms`,
          result: result.success ? "success" : "error",
        });
      }
      if (timingGranularity === "detailed" && result.timings_ms) {
        appendLog({
          source: "debug",
          label: "Timing",
          detail: formatTimingDetails(result.timings_ms),
          result: "info",
        });
      }
    } catch (e) {
      appendLog({ source: "debug", label: "NodeTest", detail: `${selectedNode.id}: test failed: ${e}`, result: "error" });
    } finally {
      setTesting(false);
    }
  };

  const handleParamChange = useCallback((key: string, value: NodeParamValue) => {
    setParamValues((current) => ({ ...current, [key]: value }));
  }, []);

  return (
    <div className="testbed-node-tester">
      <div className="testbed-node-tester-header">
        <div className="testbed-node-select-row">
          <label>Node</label>
          <select
            value={selectedNodeKey}
            onChange={(e) => setSelectedNodeKey(e.target.value)}
            disabled={loading}
          >
            {nodeMetas.length === 0 && <option value="">No nodes found...</option>}
            {groupedNodes.map(([page, nodes]) => (
              <optgroup key={page} label={page}>
                {nodes.map((n) => (
                  <option key={nodeTestKey(n)} value={nodeTestKey(n)}>
                    {n.source === "custom" ? `${n.id} (custom)` : n.id}
                  </option>
                ))}
              </optgroup>
            ))}
          </select>
          <button
            className="testbed-node-refresh"
            onClick={() => void loadNodeMetas()}
            disabled={loading}
            title="Refresh node list"
          >
            ↻
          </button>
        </div>
      </div>

      {selectedNode && (
        <>
          <div className="tester-panel">
            <div className="tester-row">
              <span className="tester-label">ID</span>
              <code className="tester-value">{selectedNode.id}</code>
            </div>
            {selectedNode.target && (
              <div className="tester-row">
                <span className="tester-label">Target</span>
                <code className="tester-value">{formatPipelineTarget(selectedNode.target)}</code>
              </div>
            )}
            <div className="tester-row">
              <span className="tester-label">Recognition</span>
              <code className="tester-value">{formatRecognition(selectedNode.recognition)}</code>
            </div>
            {selectedNode.recognition.roi && (
              <div className="tester-row">
                <span className="tester-label">ROI</span>
                <code className="tester-value">{typeof selectedNode.recognition.roi === "string" ? selectedNode.recognition.roi : `[${selectedNode.recognition.roi.join(", ")}]`}</code>
              </div>
            )}
            {selectedNode.recognition.roi_offset && (
              <div className="tester-row">
                <span className="tester-label">ROI Offset</span>
                <code className="tester-value">[{selectedNode.recognition.roi_offset.join(", ")}]</code>
              </div>
            )}
            {selectedNode.recognition.candidate_selection && (
              <div className="tester-row">
                <span className="tester-label">Candidate</span>
                <code className="tester-value">
                  {selectedNode.recognition.candidate_selection.order ?? "native"} #{selectedNode.recognition.candidate_selection.index ?? 0}
                </code>
              </div>
            )}
            {selectedNode.actions.length > 0 && (
              <div className="tester-row">
                <span className="tester-label">Actions</span>
                <code className="tester-value">
                  {selectedNode.actions.map((a, i) => (
                    <span key={i}>{formatAction(a)}{i < selectedNode.actions.length - 1 ? ", " : ""}</span>
                  ))}
                </code>
              </div>
            )}
            <div className="tester-row">
              <span className="tester-label">Timeout</span>
              <code className="tester-value">{selectedNode.timeout ?? 20000}ms</code>
            </div>
            <div className="tester-row">
              <span className="tester-label">Rate Limit</span>
              <code className="tester-value">{selectedNode.rate_limit ?? 1000}ms</code>
            </div>
            <div className="tester-row">
              <span className="tester-label">Enabled</span>
              <span className={`tester-badge ${selectedNode.enabled ? "matched" : "unmatched"}`}>
                {selectedNode.enabled ? "YES" : "NO"}
              </span>
            </div>
            <div className="tester-row">
              <span className="tester-label">Max Hit</span>
              <code className="tester-value">{selectedNode.max_hit ?? 1}</code>
            </div>
          </div>

          {selectedNode.params.length > 0 && (
            <ParamsForm
              params={selectedNode.params}
              values={paramValues}
              onChange={handleParamChange}
            />
          )}

          <div className="testbed-node-tester-actions">
            <button
              className="tester-run-btn"
              onClick={() => void handleOpenWindow()}
              disabled={loading || !selectedNode.target}
              title={!selectedNode.target
                ? "No capture target configured"
                : selectedNode.target.type === "screen"
                  ? "Select this screen"
                  : "Open the target window"}
            >
              {selectedNode.target?.type === "screen" ? "Select Screen" : "Open Window"}
            </button>
            <button
              className="tester-run-btn"
              onClick={() => void handleStartNode()}
              disabled={loading || testing || !selectedNode.target || !selectedNode.enabled}
              title={!selectedNode.enabled
                ? "This node is disabled"
                : !selectedNode.target
                  ? "No target window configured"
                  : "Run recognition + actions"}
            >
              {testing ? "Testing..." : "Start This Node"}
            </button>
          </div>
        </>
      )}

      {!selectedNode && !loading && (
        <div className="testbed-node-empty">
          {nodeMetas.length === 0
            ? "No node configs found in resources/pipeline-nodes/"
            : "Select a node to test"}
        </div>
      )}
    </div>
  );
}
