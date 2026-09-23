import { useEffect, useMemo, useRef, useState } from "react";
import { testNode, type NodeExecutionMode, type NodeTestResult } from "../../../api/nodeTest";
import type { Action, CandidateOrder, ParamType, PipelineTarget, Recognition } from "../../../api/pipeline";
import { usePreferencesStore } from "../../shell/stores/usePreferencesStore";
import { useDebugStore } from "../../capture/stores/useDebugStore";
import { useLogStore } from "../../observability/stores/useLogStore";
import { useOverlayStore } from "../../capture/stores/useOverlayStore";
import { resolveRecognitionOverlayRoi } from "../../capture/utils/recognitionOverlay";
import {
  buildNodeEmulator,
  defaultActionDraft,
  defaultParamDraft,
  duplicateItem,
  moveItem,
  type EmulatorActionDraft,
  type EmulatorParamDraft,
  type EmulatorTargetKind,
  type NodeEmulatorDraft,
} from "./nodeEmulatorModel";

const RECOGNITION_TYPES: Recognition["type"][] = ["DirectHit", "TemplateMatch", "OCR", "ColorMatch", "Custom"];
const ACTION_TYPES: Action["type"][] = ["DoNothing", "Wait", "Click", "LongPress", "Swipe", "Scroll", "ClickKey", "InputText", "StopTask", "Custom"];
const PARAM_TYPES: ParamType[] = ["text", "password", "number", "dropdown", "switch", "color"];
const TARGET_KINDS: EmulatorTargetKind[] = ["recognized", "node", "point", "region"];
const CANDIDATE_ORDERS: CandidateOrder[] = ["native", "top_to_bottom", "bottom_to_top", "left_to_right", "right_to_left", "confidence_desc"];

function ErrorText({ errors, path }: { errors: Record<string, string>; path: string }) {
  return errors[path] ? <span className="node-emulator-error">{errors[path]}</span> : null;
}

function TargetFields({
  prefix,
  kind,
  value,
  offset,
  onKind,
  onValue,
  onOffset,
  errors,
}: {
  prefix: string;
  kind: EmulatorTargetKind;
  value: string;
  offset: string;
  onKind: (value: EmulatorTargetKind) => void;
  onValue: (value: string) => void;
  onOffset: (value: string) => void;
  errors: Record<string, string>;
}) {
  return (
    <div className="node-emulator-target-fields">
      <label>Target form
        <select value={kind} onChange={(event) => onKind(event.target.value as EmulatorTargetKind)}>
          {TARGET_KINDS.map((item) => <option key={item} value={item}>{item}</option>)}
        </select>
      </label>
      {kind !== "recognized" && (
        <label>{kind === "node" ? "Node ID" : kind === "point" ? "Point (x,y)" : "Region (x,y,w,h)"}
          <input value={value} onChange={(event) => onValue(event.target.value)} />
          <ErrorText errors={errors} path={prefix} />
        </label>
      )}
      <label>Offset (x,y,w,h)
        <input value={offset} onChange={(event) => onOffset(event.target.value)} />
        <ErrorText errors={errors} path={`${prefix}_offset`} />
      </label>
    </div>
  );
}

function ActionEditor({
  action,
  index,
  errors,
  onChange,
  onCommand,
}: {
  action: EmulatorActionDraft;
  index: number;
  errors: Record<string, string>;
  onChange: (patch: Partial<EmulatorActionDraft>) => void;
  onCommand: (command: "up" | "down" | "duplicate" | "remove") => void;
}) {
  const path = `actions.${index}`;
  const usesTarget = ["Click", "LongPress", "Scroll"].includes(action.type);
  return (
    <div className="node-emulator-row-card">
      <div className="node-emulator-row-header">
        <strong>Action {index + 1}</strong>
        <div><button onClick={() => onCommand("up")} disabled={index === 0}>↑</button><button onClick={() => onCommand("down")}>↓</button><button onClick={() => onCommand("duplicate")}>Duplicate</button><button onClick={() => onCommand("remove")}>Remove</button></div>
      </div>
      <label>Type
        <select value={action.type} onChange={(event) => onChange({ type: event.target.value as Action["type"] })}>
          {ACTION_TYPES.map((item) => <option key={item}>{item}</option>)}
        </select>
      </label>
      {usesTarget && <TargetFields prefix={`${path}.target`} kind={action.targetKind} value={action.target} offset={action.targetOffset} onKind={(targetKind) => onChange({ targetKind })} onValue={(target) => onChange({ target })} onOffset={(targetOffset) => onChange({ targetOffset })} errors={errors} />}
      {action.type === "Swipe" && <>
        <TargetFields prefix={`${path}.begin`} kind={action.targetKind} value={action.target} offset={action.targetOffset} onKind={(targetKind) => onChange({ targetKind })} onValue={(target) => onChange({ target })} onOffset={(targetOffset) => onChange({ targetOffset })} errors={errors} />
        <TargetFields prefix={`${path}.end`} kind={action.endKind} value={action.end} offset={action.endOffset} onKind={(endKind) => onChange({ endKind })} onValue={(end) => onChange({ end })} onOffset={(endOffset) => onChange({ endOffset })} errors={errors} />
      </>}
      {(action.type === "Wait" || action.type === "LongPress" || action.type === "Swipe") && <label>Duration (ms)<input type="number" min="0" value={action.duration} placeholder={action.type === "LongPress" ? "1000" : action.type === "Swipe" ? "200" : "500"} onChange={(event) => onChange({ duration: event.target.value })} /><ErrorText errors={errors} path={`${path}.duration`} /></label>}
      {action.type === "Scroll" && <div className="node-emulator-grid"><label>dx<input type="number" value={action.dx} onChange={(event) => onChange({ dx: event.target.value })} /><ErrorText errors={errors} path={`${path}.dx`} /></label><label>dy<input type="number" value={action.dy} onChange={(event) => onChange({ dy: event.target.value })} /><ErrorText errors={errors} path={`${path}.dy`} /></label></div>}
      {action.type === "ClickKey" && <label>Key codes<input value={action.keys} placeholder="13 or 17,67" onChange={(event) => onChange({ keys: event.target.value })} /><ErrorText errors={errors} path={`${path}.key`} /></label>}
      {action.type === "InputText" && <label>Input text<input value={action.inputText} onChange={(event) => onChange({ inputText: event.target.value })} /></label>}
      {action.type === "Custom" && <><label>Action name<input value={action.customName} onChange={(event) => onChange({ customName: event.target.value })} /><ErrorText errors={errors} path={`${path}.action`} /></label><label>Custom JSON<textarea value={action.customParam} onChange={(event) => onChange({ customParam: event.target.value })} /><ErrorText errors={errors} path={`${path}.param`} /></label></>}
    </div>
  );
}

function ParamEditor({ param, index, errors, onChange, onCommand }: { param: EmulatorParamDraft; index: number; errors: Record<string, string>; onChange: (patch: Partial<EmulatorParamDraft>) => void; onCommand: (command: "up" | "down" | "duplicate" | "remove") => void }) {
  const path = `params.${index}`;
  return <div className="node-emulator-row-card">
    <div className="node-emulator-row-header"><strong>Parameter {index + 1}</strong><div><button onClick={() => onCommand("up")} disabled={index === 0}>↑</button><button onClick={() => onCommand("down")}>↓</button><button onClick={() => onCommand("duplicate")}>Duplicate</button><button onClick={() => onCommand("remove")}>Remove</button></div></div>
    <div className="node-emulator-grid">
      <label>Type<select value={param.type} onChange={(event) => onChange({ type: event.target.value as ParamType })}>{PARAM_TYPES.map((item) => <option key={item}>{item}</option>)}</select></label>
      <label>Key<input value={param.key} onChange={(event) => onChange({ key: event.target.value })} /><ErrorText errors={errors} path={`${path}.key`} /></label>
      <label>Label<input value={param.label} onChange={(event) => onChange({ label: event.target.value })} /><ErrorText errors={errors} path={`${path}.label`} /></label>
      <label>Placeholder<input value={param.placeholder} onChange={(event) => onChange({ placeholder: event.target.value })} /></label>
      <label>Default<input value={param.defaultValue} onChange={(event) => onChange({ defaultValue: event.target.value })} /><ErrorText errors={errors} path={`${path}.default`} /></label>
      {param.type === "dropdown" && <label>Options (comma separated)<input value={param.options} onChange={(event) => onChange({ options: event.target.value })} /></label>}
    </div>
  </div>;
}

export default function NodeEmulatorTool({ currentTarget }: { currentTarget?: PipelineTarget }) {
  const draft = useDebugStore((state) => state.nodeEmulatorDraft);
  const setDraft = useDebugStore((state) => state.setNodeEmulatorDraft);
  const initializeTarget = useDebugStore((state) => state.initializeNodeEmulatorTarget);
  const useTarget = useDebugStore((state) => state.useNodeEmulatorTarget);
  const appendLog = useLogStore((state) => state.appendLog);
  const setRoi = useOverlayStore((state) => state.setRoi);
  const setResultRoi = useOverlayStore((state) => state.setResultRoi);
  const clearRecognition = useOverlayStore((state) => state.clearRecognition);
  const imageDimensions = useOverlayStore((state) => state.imageDimensions);
  const built = useMemo(() => buildNodeEmulator(draft), [draft]);
  const [loading, setLoading] = useState<NodeExecutionMode | null>(null);
  const [result, setResult] = useState<NodeTestResult | null>(null);
  const [runError, setRunError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const executionGeneration = useRef(0);
  const resultGeneration = useRef(0);

  useEffect(() => { if (currentTarget) initializeTarget(currentTarget); }, [currentTarget, initializeTarget]);
  useEffect(() => () => {
    executionGeneration.current += 1;
    clearRecognition();
  }, [clearRecognition]);
  useEffect(() => {
    executionGeneration.current += 1;
    setResult(null);
    setRunError(null);
    setLoading(null);
    clearRecognition();
  }, [clearRecognition, draft.recognition, draft.target]);
  useEffect(() => {
    if (!result || !built.value || resultGeneration.current !== executionGeneration.current) return;
    const effectiveRoi = result.capture_roi;
    setRoi(effectiveRoi ? {
      x: effectiveRoi.x,
      y: effectiveRoi.y,
      w: effectiveRoi.width,
      h: effectiveRoi.height,
    } : resolveRecognitionOverlayRoi(built.value.node.recognition, imageDimensions));
    const captureResult = result.capture_box_region
      ?? result.candidates.find((candidate) => candidate.selected)?.region
      ?? null;
    setResultRoi(captureResult ? {
      x: captureResult.x,
      y: captureResult.y,
      w: captureResult.width,
      h: captureResult.height,
    } : null);
  }, [built.value, imageDimensions, result, setResultRoi, setRoi]);
  const update = (patch: Partial<NodeEmulatorDraft>) => setDraft({ ...draft, ...patch });
  const updateRecognition = (patch: Partial<NodeEmulatorDraft["recognition"]>) => update({ recognition: { ...draft.recognition, ...patch } });
  const updateAction = (index: number, patch: Partial<EmulatorActionDraft>) => update({ actions: draft.actions.map((item, itemIndex) => itemIndex === index ? { ...item, ...patch } : item) });
  const updateParam = (index: number, patch: Partial<EmulatorParamDraft>) => update({ params: draft.params.map((item, itemIndex) => itemIndex === index ? { ...item, ...patch } : item) });
  const command = <T,>(items: T[], index: number, operation: "up" | "down" | "duplicate" | "remove") => operation === "duplicate" ? duplicateItem(items, index) : operation === "remove" ? items.filter((_, itemIndex) => itemIndex !== index) : moveItem(items, index, operation === "up" ? index - 1 : index + 1);

  const execute = async (mode: NodeExecutionMode) => {
    if (!built.value || !draft.enabled) return;
    const generation = ++executionGeneration.current;
    setRoi(resolveRecognitionOverlayRoi(built.value.node.recognition, imageDimensions));
    setResultRoi(null);
    setLoading(mode); setResult(null); setRunError(null);
    try {
      const preferences = usePreferencesStore.getState();
      const next = await testNode(built.value.node, preferences.saveDebugImages, built.value.paramValues, preferences.timingGranularity === "detailed", mode);
      if (generation !== executionGeneration.current) return;
      resultGeneration.current = generation;
      setResult(next);
      appendLog({ source: "debug", label: mode === "recognize_only" ? "Node recognize" : "Node run", detail: `${draft.id}: ${next.success ? "success" : next.error ?? "failed"}`, result: next.success ? "success" : "error" });
    } catch (error) {
      if (generation !== executionGeneration.current) return;
      setRunError(String(error));
      appendLog({ source: "debug", label: "Node emulator", detail: String(error), result: "error" });
    } finally {
      if (generation === executionGeneration.current) setLoading(null);
    }
  };

  const metadataJson = built.value ? JSON.stringify(built.value.metadata, null, 2) : "Fix validation errors to generate canonical metadata.";
  const needsPriorNode = built.value && typeof built.value.node.recognition.roi === "string";
  const structural = draft.kind !== "Normal";
  const screenTarget = draft.target.type === "screen" ? draft.target : null;
  const windowTarget = draft.target.type === "window" ? draft.target : null;
  return <div className="node-emulator-tool">
    <div className="node-emulator-actions"><button disabled={!currentTarget} onClick={() => currentTarget && useTarget(currentTarget)}>Use Current Target</button><button disabled={!built.value || !draft.enabled || !!loading || !!needsPriorNode || structural} onClick={() => void execute("recognize_only")}>{loading === "recognize_only" ? "Recognizing…" : "Recognize Only"}</button><button className="action-execute-btn" disabled={!built.value || !draft.enabled || !!loading || !!needsPriorNode || structural} onClick={() => void execute("full")}>{loading === "full" ? "Running…" : "Run Full Node"}</button></div>
    {structural && <div className="node-emulator-error">Entry and Endpoint are structural pipeline nodes. They have no recognition or actions and cannot run as individual node tests.</div>}
    {needsPriorNode && <div className="node-emulator-error">A node-referenced ROI needs an earlier successful pipeline node. Copy this metadata into a flow to test it; independent emulation cannot resolve the reference.</div>}
    {!draft.enabled && <div className="node-emulator-error">Disabled nodes cannot be executed.</div>}
    <details open><summary>Node</summary><div className="node-emulator-grid">
      <label>ID<input value={draft.id} onChange={(event) => update({ id: event.target.value })} /><ErrorText errors={built.errors} path="id" /></label>
      <label>Kind<select value={draft.kind} onChange={(event) => update({ kind: event.target.value as NodeEmulatorDraft["kind"] })}><option>Normal</option><option>Entry</option><option>Endpoint</option></select></label>
      <label>Description<input value={draft.desc} placeholder="What this node does" onChange={(event) => update({ desc: event.target.value })} /></label>
      <label className="node-emulator-check"><input type="checkbox" checked={draft.enabled} onChange={(event) => update({ enabled: event.target.checked })} /> Enabled</label>
      <label className="node-emulator-check"><input type="checkbox" checked={draft.inverse} onChange={(event) => update({ inverse: event.target.checked })} /> Inverse result</label>
    </div></details>
    <details open><summary>Capture target</summary>{screenTarget ? <div className="node-emulator-grid"><label>Type<select value="screen" onChange={(event) => update({ target: event.target.value === "screen" ? screenTarget : { type: "window", title: "", capture_area: "frame" } })}><option value="screen">Screen</option><option value="window">Window</option></select></label><label>Screen index<input type="number" min="0" value={screenTarget.index ?? 0} onChange={(event) => update({ target: { ...screenTarget, index: Number(event.target.value) } })} /><ErrorText errors={built.errors} path="target.index" /></label></div> : windowTarget && <div className="node-emulator-grid">
      <label>Type<select value="window" onChange={(event) => update({ target: event.target.value === "screen" ? { type: "screen", index: 0 } : windowTarget })}><option value="screen">Screen</option><option value="window">Window</option></select></label>
      <label>Capture area<select value={windowTarget.capture_area ?? "frame"} onChange={(event) => update({ target: { ...windowTarget, capture_area: event.target.value as "frame" | "client" } })}><option value="frame">Frame</option><option value="client">Client</option></select></label>
      <label className="node-emulator-check"><input type="checkbox" checked={!!windowTarget.window_size} onChange={(event) => update({ target: { ...windowTarget, window_size: event.target.checked ? [1280, 720] : undefined } })} /> Require client size</label>
      {windowTarget.window_size && <>
        <label>Client width<input type="number" min="1" value={windowTarget.window_size[0]} onChange={(event) => update({ target: { ...windowTarget, window_size: [Number(event.target.value), windowTarget.window_size![1]] } })} /></label>
        <label>Client height<input type="number" min="1" value={windowTarget.window_size[1]} onChange={(event) => update({ target: { ...windowTarget, window_size: [windowTarget.window_size![0], Number(event.target.value)] } })} /><ErrorText errors={built.errors} path="target.window_size" /></label>
      </>}
      {(["process", "class", "title"] as const).map((key) => <label key={key}>{key} regex<input value={windowTarget[key] ?? ""} onChange={(event) => update({ target: { ...windowTarget, [key]: event.target.value || undefined } })} /><ErrorText errors={built.errors} path={`target.${key}`} /></label>)}<ErrorText errors={built.errors} path="target" />
    </div>}</details>
    {!structural && <details open><summary>Recognition</summary><div className="node-emulator-grid">
      <label>Type<select value={draft.recognition.type} onChange={(event) => updateRecognition({ type: event.target.value as Recognition["type"] })}>{RECOGNITION_TYPES.map((item) => <option key={item}>{item}</option>)}</select></label>
      {draft.recognition.type === "TemplateMatch" && <label>Template<input value={draft.recognition.template} onChange={(event) => updateRecognition({ template: event.target.value })} /><ErrorText errors={built.errors} path="recognition.template" /></label>}
      {draft.recognition.type === "OCR" && <label>Expected regex<input value={draft.recognition.expected} onChange={(event) => updateRecognition({ expected: event.target.value })} /><ErrorText errors={built.errors} path="recognition.expected" /></label>}
      {(draft.recognition.type === "OCR" || draft.recognition.type === "TemplateMatch") && <label>Threshold (0–1)<input type="number" min="0" max="1" step="0.01" value={draft.recognition.threshold} onChange={(event) => updateRecognition({ threshold: event.target.value })} /><ErrorText errors={built.errors} path="recognition.threshold" /></label>}
      {draft.recognition.type === "ColorMatch" && <><label>Lower RGB<input value={draft.recognition.lower} onChange={(event) => updateRecognition({ lower: event.target.value })} /><ErrorText errors={built.errors} path="recognition.lower" /></label><label>Upper RGB<input value={draft.recognition.upper} onChange={(event) => updateRecognition({ upper: event.target.value })} /><ErrorText errors={built.errors} path="recognition.upper" /></label></>}
      {draft.recognition.type === "Custom" && <><label>Recognizer name<input value={draft.recognition.customName} onChange={(event) => updateRecognition({ customName: event.target.value })} /><ErrorText errors={built.errors} path="recognition.recognition" /></label><label>Custom JSON<textarea value={draft.recognition.customParam} onChange={(event) => updateRecognition({ customParam: event.target.value })} /><ErrorText errors={built.errors} path="recognition.param" /></label></>}
      <label>ROI (x,y,w,h or earlier NodeId)<input value={draft.recognition.roi} onChange={(event) => updateRecognition({ roi: event.target.value })} /><ErrorText errors={built.errors} path="recognition.roi" /></label>
      <label>ROI offset<input value={draft.recognition.roiOffset} onChange={(event) => updateRecognition({ roiOffset: event.target.value })} /><ErrorText errors={built.errors} path="recognition.roi_offset" /></label>
      <label>Candidate order<select value={draft.recognition.candidateOrder} onChange={(event) => updateRecognition({ candidateOrder: event.target.value as CandidateOrder })}>{CANDIDATE_ORDERS.map((item) => <option key={item} value={item}>{item}</option>)}</select></label>
      <label>Candidate index<input type="number" min="0" value={draft.recognition.candidateIndex} onChange={(event) => updateRecognition({ candidateIndex: event.target.value })} /><ErrorText errors={built.errors} path="recognition.candidate_selection.index" /></label>
    </div></details>}
    {!structural && <details><summary>Actions ({draft.actions.length})</summary>{draft.actions.map((action, index) => <ActionEditor key={index} action={action} index={index} errors={built.errors} onChange={(patch) => updateAction(index, patch)} onCommand={(operation) => update({ actions: command(draft.actions, index, operation) })} />)}<button onClick={() => update({ actions: [...draft.actions, defaultActionDraft()] })}>Add action</button></details>}
    <details><summary>Parameters ({draft.params.length})</summary>{draft.params.map((param, index) => <ParamEditor key={index} param={param} index={index} errors={built.errors} onChange={(patch) => updateParam(index, patch)} onCommand={(operation) => update({ params: command(draft.params, index, operation) })} />)}<button onClick={() => update({ params: [...draft.params, defaultParamDraft()] })}>Add parameter</button></details>
    <details><summary>Timing, repeat, and attach</summary><div className="node-emulator-grid">
      <label>Timeout (ms)<small>Reserved in this single-run emulator</small><input type="number" min="0" value={draft.timeout} onChange={(event) => update({ timeout: event.target.value })} /><ErrorText errors={built.errors} path="timeout" /></label>
      <label>Error timeout (ms)<small>Reserved in this single-run emulator</small><input type="number" min="0" value={draft.errorTimeout} onChange={(event) => update({ errorTimeout: event.target.value })} /><ErrorText errors={built.errors} path="error_timeout" /></label>
      <label>Rate limit (ms)<small>Reserved in this single-run emulator</small><input type="number" min="0" value={draft.rateLimit} onChange={(event) => update({ rateLimit: event.target.value })} /><ErrorText errors={built.errors} path="rate_limit" /></label>
      <label>Pre-delay (ms)<input type="number" min="0" value={draft.preDelay} onChange={(event) => update({ preDelay: event.target.value })} /><ErrorText errors={built.errors} path="pre_delay" /></label>
      <label>Pre-wait freezes (ms)<input type="number" min="0" max="30000" value={draft.preWaitFreezes} onChange={(event) => update({ preWaitFreezes: event.target.value })} /><ErrorText errors={built.errors} path="pre_wait_freezes" /></label>
      <label>Post-delay (ms)<input type="number" min="0" value={draft.postDelay} onChange={(event) => update({ postDelay: event.target.value })} /><ErrorText errors={built.errors} path="post_delay" /></label>
      <label>Post-wait freezes (ms)<input type="number" min="0" max="30000" value={draft.postWaitFreezes} onChange={(event) => update({ postWaitFreezes: event.target.value })} /><ErrorText errors={built.errors} path="post_wait_freezes" /></label>
      <label>Repeat<input type="number" min="0" value={draft.repeat} onChange={(event) => update({ repeat: event.target.value })} /><ErrorText errors={built.errors} path="repeat" /></label>
      <label>Repeat delay (ms)<input type="number" min="0" value={draft.repeatDelay} onChange={(event) => update({ repeatDelay: event.target.value })} /><ErrorText errors={built.errors} path="repeat_delay" /></label>
      <label>Max hit<small>Displayed only; not accumulated across runs</small><input type="number" min="0" value={draft.maxHit} onChange={(event) => update({ maxHit: event.target.value })} /><ErrorText errors={built.errors} path="max_hit" /></label>
      <label className="node-emulator-wide">Attach JSON<textarea value={draft.attach} placeholder="Optional JSON" onChange={(event) => update({ attach: event.target.value })} /><ErrorText errors={built.errors} path="attach" /></label>
    </div></details>
    {Object.keys(built.errors).length > 0 && <div className="node-emulator-validation"><strong>Fix {Object.keys(built.errors).length} validation issue(s)</strong>{Object.entries(built.errors).map(([path, message]) => <div key={path}>{path}: {message}</div>)}</div>}
    <details open><summary>Metadata JSON</summary><textarea className="node-emulator-json" readOnly value={metadataJson} /><button disabled={!built.value} onClick={async () => { await navigator.clipboard.writeText(metadataJson); setCopied(true); window.setTimeout(() => setCopied(false), 1500); }}>{copied ? "Copied" : "Copy JSON"}</button></details>
    {runError && <div className="action-result error">{runError}</div>}
    {result && <div className="node-emulator-result"><strong>{result.success ? "Success" : "Failed"}</strong><span>matched: {String(result.matched)}</span><span>confidence: {(result.confidence * 100).toFixed(1)}%</span>{result.raw_candidate_count !== undefined && <span>candidates: {result.raw_candidate_count} raw → {result.distinct_candidate_count ?? result.candidates.length} distinct</span>}<span>actions fired: {result.actions_fired}</span><span>duration: {result.duration_ms} ms</span>{result.screen_click_position && <span>screen position: ({result.screen_click_position.join(", ")})</span>}{result.error && <span>{result.error}</span>}{result.candidates.map((candidate) => <span key={`${candidate.rank}-${candidate.region.x}-${candidate.region.y}`}>#{candidate.rank}{candidate.selected ? " selected" : ""}: {candidate.text ?? "candidate"} · {(candidate.confidence * 100).toFixed(1)}% · ({candidate.region.x},{candidate.region.y},{candidate.region.width},{candidate.region.height})</span>)}</div>}
  </div>;
}
