import type {
  Action,
  CandidateOrder,
  Node,
  ParamDef,
  ParamType,
  PipelineTarget,
  Recognition,
  Target,
} from "../../../../api/pipeline";
import type { NodeMetaInfo } from "../../../../api/nodeTest";

export type EmulatorTargetKind = "recognized" | "node" | "point" | "region";

export interface EmulatorRecognitionDraft {
  type: Recognition["type"];
  template: string;
  expected: string;
  threshold: string;
  lower: string;
  upper: string;
  customName: string;
  customParam: string;
  roi: string;
  roiOffset: string;
  candidateOrder: CandidateOrder;
  candidateIndex: string;
}

export interface EmulatorActionDraft {
  type: Action["type"];
  targetKind: EmulatorTargetKind;
  target: string;
  targetOffset: string;
  endKind: EmulatorTargetKind;
  end: string;
  endOffset: string;
  duration: string;
  dx: string;
  dy: string;
  keys: string;
  inputText: string;
  customName: string;
  customParam: string;
}

export interface EmulatorParamDraft {
  type: ParamType;
  key: string;
  label: string;
  placeholder: string;
  defaultValue: string;
  options: string;
}

export interface NodeEmulatorDraft {
  id: string;
  enabled: boolean;
  inverse: boolean;
  target: PipelineTarget;
  recognition: EmulatorRecognitionDraft;
  actions: EmulatorActionDraft[];
  params: EmulatorParamDraft[];
  timeout: string;
  rateLimit: string;
  preDelay: string;
  postDelay: string;
  maxHit: string;
  repeat: string;
  repeatDelay: string;
  attach: string;
}

export interface BuiltNodeEmulator {
  node: NodeMetaInfo;
  metadata: { target: PipelineTarget; nodes: Record<string, Omit<Node, "id">>; edges: Record<string, never> };
  paramValues: Record<string, string | number | boolean>;
}

export const defaultRecognitionDraft = (): EmulatorRecognitionDraft => ({
  type: "DirectHit",
  template: "",
  expected: "",
  threshold: "",
  lower: "0,0,0",
  upper: "255,255,255",
  customName: "",
  customParam: "{}",
  roi: "0,0,0,0",
  roiOffset: "0,0,0,0",
  candidateOrder: "native",
  candidateIndex: "0",
});

export const defaultActionDraft = (): EmulatorActionDraft => ({
  type: "DoNothing",
  targetKind: "recognized",
  target: "",
  targetOffset: "0,0,0,0",
  endKind: "recognized",
  end: "",
  endOffset: "0,0,0,0",
  duration: "",
  dx: "0",
  dy: "0",
  keys: "",
  inputText: "",
  customName: "",
  customParam: "{}",
});

export const defaultParamDraft = (): EmulatorParamDraft => ({
  type: "text",
  key: "",
  label: "",
  placeholder: "",
  defaultValue: "",
  options: "",
});

export function createNodeEmulatorDraft(target?: PipelineTarget): NodeEmulatorDraft {
  return {
    id: "EmulatedNode",
    enabled: true,
    inverse: false,
    target: target ?? { type: "screen", index: 0 },
    recognition: defaultRecognitionDraft(),
    actions: [defaultActionDraft()],
    params: [],
    timeout: "20000",
    rateLimit: "1000",
    preDelay: "200",
    postDelay: "200",
    maxHit: "4294967295",
    repeat: "1",
    repeatDelay: "0",
    attach: "",
  };
}

export function escapeExactRegex(value: string): string {
  return `^${value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}$`;
}

function tuple(value: string, size: number, path: string, errors: Record<string, string>): number[] {
  const values = value.split(",").map((part) => Number(part.trim()));
  if (values.length !== size || values.some((item) => !Number.isInteger(item) || item < -2147483648 || item > 2147483647)) {
    errors[path] = `Enter ${size} comma-separated 32-bit integers`;
    return Array(size).fill(0);
  }
  return values;
}

function regionTuple(value: string, path: string, errors: Record<string, string>): number[] {
  const values = tuple(value, 4, path, errors);
  if (values[2] < 0 || values[3] < 0) errors[path] = "Region width and height cannot be negative";
  return values;
}

function rgbTuple(value: string, path: string, errors: Record<string, string>): number[] {
  const values = tuple(value, 3, path, errors);
  if (values.some((item) => !Number.isInteger(item) || item < 0 || item > 255)) {
    errors[path] = "Enter three RGB integers between 0 and 255";
  }
  return values;
}

function keyCodes(value: string, path: string, errors: Record<string, string>): number[] {
  if (!value.trim()) {
    errors[path] = "Enter at least one key code";
    return [];
  }
  const values = value.split(",").map((part) => Number(part.trim()));
  if (values.some((item) => !Number.isInteger(item) || item < 0 || item > 4294967295)) {
    errors[path] = "Enter comma-separated unsigned 32-bit key codes";
  }
  return values;
}

function integer(value: string, path: string, errors: Record<string, string>, min = 0, max = Number.MAX_SAFE_INTEGER): number {
  const parsed = Number(value);
  if (!Number.isSafeInteger(parsed) || parsed < min || parsed > max) {
    errors[path] = `Enter an integer between ${min} and ${max}`;
    return min;
  }
  return parsed;
}

function json(value: string, path: string, errors: Record<string, string>): unknown {
  try {
    return JSON.parse(value);
  } catch {
    errors[path] = "Enter valid JSON";
    return null;
  }
}

function actionTarget(
  kind: EmulatorTargetKind,
  value: string,
  path: string,
  errors: Record<string, string>,
): Target {
  if (kind === "recognized") return true;
  if (kind === "node") {
    if (!value.trim()) errors[path] = "Enter a node ID";
    return value.trim();
  }
  const values = kind === "point" ? tuple(value, 2, path, errors) : regionTuple(value, path, errors);
  return values as [number, number] | [number, number, number, number];
}

function optionalOffset(value: string, path: string, errors: Record<string, string>) {
  const values = tuple(value || "0,0,0,0", 4, path, errors) as [number, number, number, number];
  return values.every((item) => item === 0) ? undefined : values;
}

function buildRecognition(draft: EmulatorRecognitionDraft, errors: Record<string, string>): Recognition {
  const roi = regionTuple(draft.roi, "recognition.roi", errors) as [number, number, number, number];
  const roiOffset = optionalOffset(draft.roiOffset, "recognition.roi_offset", errors);
  const threshold = draft.threshold === "" ? undefined : Number(draft.threshold);
  if (threshold !== undefined && (!Number.isFinite(threshold) || threshold < 0 || threshold > 1)) {
    errors["recognition.threshold"] = "Threshold must be between 0 and 1";
  }
  const candidate_selection = {
    order: draft.candidateOrder,
    index: integer(draft.candidateIndex, "recognition.candidate_selection.index", errors),
  };
  const common = { roi, roi_offset: roiOffset, candidate_selection };
  switch (draft.type) {
    case "DirectHit": return { type: "DirectHit", ...common };
    case "TemplateMatch":
      if (!draft.template.trim()) errors["recognition.template"] = "Template is required";
      return { type: "TemplateMatch", template: draft.template.trim(), threshold, ...common };
    case "OCR":
      if (draft.expected) {
        try { new RegExp(draft.expected); } catch { errors["recognition.expected"] = "Enter a valid regular expression"; }
      }
      return { type: "OCR", expected: draft.expected || undefined, threshold, ...common };
    case "ColorMatch": return {
      type: "ColorMatch",
      lower: rgbTuple(draft.lower, "recognition.lower", errors),
      upper: rgbTuple(draft.upper, "recognition.upper", errors),
      ...common,
    };
    case "Custom":
      if (!draft.customName.trim()) errors["recognition.recognition"] = "Recognizer name is required";
      return {
        type: "Custom",
        recognition: draft.customName.trim(),
        param: json(draft.customParam, "recognition.param", errors),
        ...common,
      };
  }
}

function buildAction(draft: EmulatorActionDraft, index: number, errors: Record<string, string>): Action {
  const path = `actions.${index}`;
  const target = () => actionTarget(draft.targetKind, draft.target, `${path}.target`, errors);
  const targetOffset = () => optionalOffset(draft.targetOffset, `${path}.target_offset`, errors);
  switch (draft.type) {
    case "DoNothing": return { type: "DoNothing" };
    case "Click": return { type: "Click", target: target(), target_offset: targetOffset() };
    case "LongPress": return {
      type: "LongPress",
      target: target(),
      target_offset: targetOffset(),
      duration: integer(draft.duration || "1000", `${path}.duration`, errors),
    };
    case "Swipe": return {
      type: "Swipe",
      begin: actionTarget(draft.targetKind, draft.target, `${path}.begin`, errors),
      begin_offset: optionalOffset(draft.targetOffset, `${path}.begin_offset`, errors),
      end: actionTarget(draft.endKind, draft.end, `${path}.end`, errors),
      end_offset: optionalOffset(draft.endOffset, `${path}.end_offset`, errors),
      duration: integer(draft.duration || "200", `${path}.duration`, errors),
    };
    case "Scroll": return {
      type: "Scroll",
      target: target(),
      target_offset: targetOffset(),
      dx: integer(draft.dx, `${path}.dx`, errors, -2147483648, 2147483647),
      dy: integer(draft.dy, `${path}.dy`, errors, -2147483648, 2147483647),
    };
    case "ClickKey": return {
      type: "ClickKey",
      key: keyCodes(draft.keys, `${path}.key`, errors),
    };
    case "InputText": return { type: "InputText", input_text: draft.inputText };
    case "StopTask": return { type: "StopTask" };
    case "Custom":
      if (!draft.customName.trim()) errors[`${path}.action`] = "Action name is required";
      return { type: "Custom", action: draft.customName.trim(), param: json(draft.customParam, `${path}.param`, errors) };
  }
}

export function moveItem<T>(items: T[], from: number, to: number): T[] {
  if (from === to || from < 0 || to < 0 || from >= items.length || to >= items.length) return items;
  const next = [...items];
  const [item] = next.splice(from, 1);
  next.splice(to, 0, item);
  return next;
}

export function duplicateItem<T>(items: T[], index: number): T[] {
  if (index < 0 || index >= items.length) return items;
  const clone = structuredClone(items[index]);
  return [...items.slice(0, index + 1), clone, ...items.slice(index + 1)];
}

function buildParam(draft: EmulatorParamDraft, index: number, errors: Record<string, string>): ParamDef {
  const path = `params.${index}`;
  if (!draft.key.trim()) errors[`${path}.key`] = "Key is required";
  if (!draft.label.trim()) errors[`${path}.label`] = "Label is required";
  let defaultValue: string | number | boolean = draft.defaultValue;
  if (draft.type === "number") {
    defaultValue = Number(draft.defaultValue || 0);
    if (!Number.isFinite(defaultValue)) errors[`${path}.default`] = "Enter a number";
  } else if (draft.type === "switch") {
    defaultValue = draft.defaultValue === "true";
  }
  return {
    type: draft.type,
    key: draft.key.trim(),
    label: draft.label.trim(),
    placeholder: draft.placeholder,
    default: defaultValue,
    options: draft.options.split(",").map((value) => value.trim()).filter(Boolean),
  };
}

export function buildNodeEmulator(draft: NodeEmulatorDraft): {
  value?: BuiltNodeEmulator;
  errors: Record<string, string>;
} {
  const errors: Record<string, string> = {};
  if (!/^[A-Za-z_][A-Za-z0-9_-]*$/.test(draft.id)) {
    errors.id = "Use a Mermaid-compatible node ID";
  }
  if (draft.target.type === "window") {
    for (const [name, value] of [["process", draft.target.process], ["class", draft.target.class], ["title", draft.target.title]] as const) {
      if (!value) continue;
      try { new RegExp(value); } catch { errors[`target.${name}`] = "Enter a valid regular expression"; }
    }
    if (draft.target.window_size && (
      !Number.isInteger(draft.target.window_size[0])
      || !Number.isInteger(draft.target.window_size[1])
      || draft.target.window_size[0] <= 0
      || draft.target.window_size[1] <= 0
    )) errors["target.window_size"] = "Width and height must be positive integers";
  }
  if (draft.target.type === "screen" && (draft.target.index === undefined || !Number.isInteger(draft.target.index) || draft.target.index < 0)) {
    errors["target.index"] = "Enter a non-negative screen index";
  }

  const recognition = buildRecognition(draft.recognition, errors);
  const actions = draft.actions.map((action, index) => buildAction(action, index, errors));
  const params = draft.params.map((param, index) => buildParam(param, index, errors));
  const keys = new Set<string>();
  params.forEach((param, index) => {
    if (keys.has(param.key)) errors[`params.${index}.key`] = "Parameter keys must be unique";
    keys.add(param.key);
  });
  const attach = draft.attach.trim() ? json(draft.attach, "attach", errors) : undefined;
  const config: Omit<Node, "id"> = {
    recognition,
    actions,
    params,
    target: draft.target,
    timeout: integer(draft.timeout, "timeout", errors),
    rate_limit: integer(draft.rateLimit, "rate_limit", errors),
    pre_delay: integer(draft.preDelay, "pre_delay", errors),
    post_delay: integer(draft.postDelay, "post_delay", errors),
    enabled: draft.enabled,
    inverse: draft.inverse,
    max_hit: integer(draft.maxHit, "max_hit", errors, 0, 4294967295),
    repeat: integer(draft.repeat, "repeat", errors, 0, 4294967295),
    repeat_delay: integer(draft.repeatDelay, "repeat_delay", errors),
    attach,
  };
  if (Object.keys(errors).length > 0) return { errors };
  const node: NodeMetaInfo = {
    id: draft.id,
    file_name: "NodeEmulator.meta.json",
    source: "custom",
    ...config,
    actions,
    params,
    enabled: draft.enabled,
  };
  const metadataConfig = { ...config, target: undefined };
  const paramValues = Object.fromEntries(params.map((param) => [param.key, param.default ?? ""]));
  return {
    errors,
    value: { node, metadata: { target: draft.target, nodes: { [draft.id]: metadataConfig }, edges: {} }, paramValues },
  };
}
