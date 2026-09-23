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
  | { type: "Wait"; duration: number }
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
  kind?: "Normal" | "Entry" | "Endpoint";
  desc?: string;
  recognition: Recognition;
  actions?: Action[];
  params?: ParamDef[];
  target?: PipelineTarget;
  timeout?: number;
  error_timeout?: number;
  rate_limit?: number;
  pre_delay?: number;
  pre_wait_freezes?: FreezeWait;
  post_delay?: number;
  post_wait_freezes?: FreezeWait;
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
  kind?: "Normal" | "Entry" | "Endpoint";
  desc?: string;
  timeout?: number;
  error_timeout?: number;
  rate_limit?: number;
  pre_delay?: number;
  pre_wait_freezes?: FreezeWait;
  post_delay?: number;
  post_wait_freezes?: FreezeWait;
  enabled?: boolean;
  inverse?: boolean;
  max_hit?: number;
  repeat?: number;
  repeat_delay?: number;
}

export type FreezeWait = number | {
  time: number;
  threshold?: number;
  rate_limit?: number;
  timeout?: number;
  roi?: [number, number, number, number];
};

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
  patrol_state?: PatrolState;
  error?: string | null;
  cleanup_error?: string | null;
  cleanup_complete?: boolean;
  attempt_count?: number;
  effective_target?: PipelineTarget | null;
  hit_counts?: Record<string, number>;
  children_parent_id?: string | null;
  children_start_time_ms?: number | null;
  children_child_id?: string | null;
  children_timeout_ms?: number | null;
  revision?: number;
}

export interface PatrolState {
  action_attempts: Array<{
    id: number;
    node_id: string;
    action_ids: number[];
    expected_successors: string[];
    error_successor: string | null;
    target: string;
    source_session: number;
    before_frame: number;
    after_frame: number | null;
    started_ms: number;
    completed_ms: number | null;
    effect: PatrolActionEffect;
  }>;
  actions: Array<{
    id: number;
    attempt_id: number;
    node_id: string;
    repetition: number;
    action_index: number;
    kind: string;
    intent: string;
    target: string;
    source_session: number;
    source_frame: number;
    started_ms: number;
    completed_ms: number | null;
    dispatch: { status: "pending" | "sent" | "failed" | "blocked"; reason?: string };
    effect: PatrolActionEffect;
  }>;
  observations: Array<{
    expected_node_id: string;
    target: string;
    session: number;
    frame: number;
    elapsed_ms: number;
    recognizer_matched: boolean | null;
    recognizer_confidence: number | null;
  }>;
  decisions: Array<{
    expected_node_id: string;
    matched_node_id: string | null;
    target: string;
    session: number;
    frame: number;
    elapsed_ms: number;
    outcome: string;
    reason: string | null;
  }>;
  recoveries: Array<{
    policy: "hint_only" | "redirection_only";
    failure_node: string;
    target_node: string | null;
    decision: "suggest" | "redirect" | "no_recovery" | "inconclusive";
    confidence: number | null;
    reason: string;
    accepted: boolean;
    redirect_count: number;
  }>;
}

export type PatrolActionEffect =
  | { status: "unverified" }
  | { status: "achieved"; matched_successor: string; session: number; frame: number; confidence: number; reason: string }
  | { status: "unchanged" | "in_progress"; session: number; frame: number; confidence: number; reason: string }
  | { status: "unexpected"; matched_node: string | null; session: number; frame: number; confidence: number; reason: string }
  | { status: "inconclusive"; session: number; frame: number; reason: string };

export interface PipelineStatusEvent {
  run_id: string;
  revision: number;
  state: string;
  current_node_id: string | null;
  step_count: number;
  elapsed_ms: number;
  error?: string | null;
  cleanup_error?: string | null;
  cleanup_complete?: boolean;
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

export interface VlmConnection {
  endpoint: string;
  model: string;
  api_key?: string;
}

export interface PatrolConfig {
  enabled: boolean;
  cycle_seconds: number;
  k_hops: number;
  threshold: number;
  min_margin: number;
  recovery_policy: "hint_only" | "redirection_only";
  max_redirects: number;
}

