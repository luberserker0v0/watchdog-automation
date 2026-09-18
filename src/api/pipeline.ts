import { invoke } from "@tauri-apps/api/core";

export type RecognitionDevice = "cpu" | "directml";

// ============ Recognition Types ============

type RecognitionKind =
  | { type: "DirectHit" }
  | { type: "TemplateMatch"; template: string; threshold?: number }
  | { type: "OCR"; expected?: string; threshold?: number }
  | { type: "ColorMatch"; lower: number[]; upper: number[] }
  | { type: "Custom"; recognition: string; param?: unknown };

export type Recognition = RecognitionKind & {
  roi?: [number, number, number, number] | string;
  roi_offset?: [number, number, number, number];
  candidate_selection?: CandidateSelection;
};

export type CandidateOrder =
  | "native"
  | "top_to_bottom"
  | "bottom_to_top"
  | "left_to_right"
  | "right_to_left"
  | "confidence_desc";

export interface CandidateSelection {
  order?: CandidateOrder;
  index?: number;
}

export type CaptureArea = "frame" | "client";

export type ActionTestContext =
  | {
      type: "screen";
      index: number;
      frame_width: number;
      frame_height: number;
    }
  | {
      type: "window";
      hwnd: number;
      capture_area: CaptureArea;
      frame_width: number;
      frame_height: number;
    };

// ============ Action Types ============

export type Target =
  | true
  | string
  | [number, number]
  | [number, number, number, number];

export type Action =
  | { type: "DoNothing" }
  | {
      type: "Click";
      target?: Target;
      target_offset?: [number, number, number, number];
    }
  | {
      type: "LongPress";
      target?: Target;
      target_offset?: [number, number, number, number];
      duration?: number;
    }
  | {
      type: "Swipe";
      begin?: Target;
      begin_offset?: [number, number, number, number];
      end?: Target;
      end_offset?: [number, number, number, number];
      duration?: number;
    }
  | {
      type: "Scroll";
      target?: Target;
      target_offset?: [number, number, number, number];
      dx?: number;
      dy?: number;
    }
  | { type: "ClickKey"; key: number[] }
  | { type: "InputText"; input_text: string }
  | { type: "StopTask" }
  | { type: "Custom"; action: string; param?: unknown };

// ============ Param Types ============

export type ParamType = "text" | "password" | "number" | "dropdown" | "switch" | "color";

export interface ParamDef {
  type: ParamType;
  key: string;
  label?: string;
  placeholder?: string;
  default?: string | number | boolean;
  options?: string[];
}

// ============ Model Types ============

export type PipelineTarget =
  | { type: "screen"; index?: number }
  | { type: "window"; process?: string; class?: string; title?: string; capture_area?: CaptureArea; window_size?: [number, number] };

export type PipelineStartError =
  | { code: "generic"; message: string }
  | {
      code: "window_size_mismatch";
      hwnd: number;
      title: string;
      expected: [number, number];
      before: [number, number];
      actual: [number, number];
      resize_attempted: boolean;
      diagnostic: string;
    };

export interface Region {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface MatchResult {
  matched: boolean;
  confidence: number;
  box_region: Region | null;
  text: string | null;
  candidates: MatchCandidate[];
  raw_candidate_count?: number;
  distinct_candidate_count?: number;
  selection_error?: string;
}

export interface MatchCandidate {
  rank: number;
  region: Region;
  confidence: number;
  text?: string;
  selected: boolean;
}

export interface Node {
  id: string;
  label?: string;
  kind?: "Normal" | "Entry";
  recognition: Recognition;
  actions?: Action[];
  params?: ParamDef[];
  target?: PipelineTarget;
  timeout?: number;
  error_timeout?: number;
  rate_limit?: number;
  pre_delay?: number;
  post_delay?: number;
  enabled?: boolean;
  inverse?: boolean;
  max_hit?: number;
  repeat?: number;
  repeat_delay?: number;
  attach?: unknown;
}

export interface InstanceConfig {
  node: string;
  params?: ParamDef[];
  param_values?: Record<string, unknown>;
  kind?: "Normal" | "Entry";
  timeout?: number;
  error_timeout?: number;
  rate_limit?: number;
  pre_delay?: number;
  post_delay?: number;
  enabled?: boolean;
  inverse?: boolean;
  max_hit?: number;
  repeat?: number;
  repeat_delay?: number;
}

export interface LaunchConfig {
  exe?: string;
  cmd?: string;
  args?: string[];
  shell?: boolean;
  cwd?: string;
  env?: Record<string, string>;
  wait_for_window?: boolean;
  wait_timeout_ms?: number;
}

export type EdgeType = "next" | "error";

export interface Edge {
  from: string;
  to: string;
  edge_type?: EdgeType;
}

export interface PipelineGraph {
  version: string;
  desktop?: string;
  target?: PipelineTarget;
  launch?: LaunchConfig;
  nodes: Node[];
  edges: Edge[];
  subroutines?: Record<string, PipelineGraph>;
}

export interface PipelineNodePosition {
  x: number;
  y: number;
}

export interface PipelineLayout {
  nodes: Record<string, PipelineNodePosition>;
}

// ============ Command Types ============

export interface LoadResult {
  graph: PipelineGraph;
  mermaid: string;
  meta: string;
  layout?: PipelineLayout | null;
  layout_writable: boolean;
}

export interface ValidationError {
  error_type: string;
  message: string;
}

export interface StepResult {
  node_id: string;
  matched: boolean;
  confidence: number;
  box_region: Region | null;
  actions_fired: number;
  skipped?: boolean;
  duration_ms: number;
  error: string | null;
  timings_ms?: Record<string, number>;
}

export interface EngineResult {
  success: boolean;
  steps: StepResult[];
  error: string | null;
}

export interface PipelineStatus {
  run_id: string;
  state: string;
  current_node_id: string | null;
  step_count: number;
  elapsed_ms: number;
  step_results: StepResult[];
  error?: string | null;
  attempt_count?: number;
  effective_target?: PipelineTarget | null;
  hit_counts?: Record<string, number>;
  children_parent_id?: string | null;
  children_start_time_ms?: number | null;
  children_child_id?: string | null;
  children_timeout_ms?: number | null;
  revision?: number;
}

export interface PipelineStatusEvent {
  run_id: string;
  revision: number;
  state: string;
  current_node_id: string | null;
  step_count: number;
  elapsed_ms: number;
  error?: string | null;
  attempt_count?: number;
  effective_target?: PipelineTarget | null;
  hit_counts?: Record<string, number>;
  children_parent_id?: string | null;
  children_start_time_ms?: number | null;
  children_child_id?: string | null;
  children_timeout_ms?: number | null;
  appended_step?: StepResult;
}

export interface PipelineInfo {
  name: string;
  path: string;
  source: string;
}

// ============ API Functions ============

export async function loadPipeline(path: string): Promise<LoadResult> {
  return invoke<LoadResult>("load_pipeline", { path });
}

export async function savePipeline(
  graph: PipelineGraph,
  path: string,
): Promise<boolean> {
  return invoke<boolean>("save_pipeline", { graph, path });
}

export async function savePipelineLayout(
  path: string,
  positions: Record<string, PipelineNodePosition>,
): Promise<boolean> {
  return invoke<boolean>("save_pipeline_layout", { path, positions });
}

export async function validatePipeline(
  graph: PipelineGraph,
): Promise<ValidationError[]> {
  return invoke<ValidationError[]>("validate_pipeline", { graph });
}

export async function listPipelines(): Promise<PipelineInfo[]> {
  return invoke<PipelineInfo[]>("list_pipelines");
}

export async function startPipeline(path: string, userParams?: Record<string, Record<string, unknown>>, saveDebugImages?: boolean, detailedTimings?: boolean): Promise<string> {
  return invoke<string>("start_pipeline", { path, userParams, saveDebugImages, detailedTimings });
}

export function normalizePipelineStartError(error: unknown): PipelineStartError {
  let value = error;
  if (typeof value === "string") {
    try { value = JSON.parse(value); } catch { return { code: "generic", message: String(error) }; }
  }
  if (value && typeof value === "object") {
    const candidate = value as Partial<PipelineStartError> & { message?: unknown };
    const isSize = (size: unknown): size is [number, number] =>
      Array.isArray(size) && size.length === 2 && size.every((item) => typeof item === "number");
    if (
      candidate.code === "window_size_mismatch"
      && isSize(candidate.expected)
      && isSize(candidate.before)
      && isSize(candidate.actual)
    ) return candidate as PipelineStartError;
    if (candidate.code === "generic" && typeof candidate.message === "string") {
      return { code: "generic", message: candidate.message };
    }
    if (value instanceof Error) return { code: "generic", message: value.message };
  }
  return { code: "generic", message: String(error) };
}

export async function stopPipeline(runId: string): Promise<boolean> {
  return invoke<boolean>("stop_pipeline", { runId });
}

export async function pausePipeline(runId: string): Promise<boolean> {
  return invoke<boolean>("pause_pipeline", { runId });
}

export async function getPipelineStatus(
  runId: string,
): Promise<PipelineStatus> {
  return invoke<PipelineStatus>("get_pipeline_status", { runId });
}

export async function testAction(action: Action, context?: ActionTestContext): Promise<string> {
  return invoke<string>("test_action", { action, context });
}

export async function testRecognition(
  recognition: Recognition,
  roi: [number, number, number, number] | undefined,
  imageBytes: number[] | null,
  candidateSelection?: CandidateSelection,
): Promise<MatchResult> {
  return invoke<MatchResult>("test_recognition", { recognition, roi, imageBytes, candidateSelection });
}

export async function setRecognitionDevice(device: RecognitionDevice): Promise<string> {
  return invoke<string>("set_recognition_device", { device });
}
