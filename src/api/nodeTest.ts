import { invoke } from "@tauri-apps/api/core";
import type { MatchCandidate, Node, PipelineTarget, Region } from "./pipeline";

export interface NodeMetaInfo extends Node {
  file_name: string;
  source?: "built-in" | "custom";
  actions: NonNullable<Node["actions"]>;
  params: NonNullable<Node["params"]>;
  enabled: boolean;
}

export interface NodeTestResult {
  success: boolean;
  matched: boolean;
  confidence: number;
  box_region: Region | null;
  actions_fired: number;
  duration_ms: number;
  error: string | null;
  candidates: MatchCandidate[];
  raw_candidate_count?: number;
  distinct_candidate_count?: number;
  selection_error?: string;
  screen_click_position?: [number, number];
  timings_ms?: Record<string, number>;
}

export async function listNodeMetas(): Promise<NodeMetaInfo[]> {
  return invoke<NodeMetaInfo[]>("list_node_metas");
}

export async function openWindow(target: PipelineTarget): Promise<string> {
  return invoke<string>("open_window", { target });
}

export type NodeParamValue = string | number | boolean;
export type NodeExecutionMode = "recognize_only" | "full";

export async function testNode(
  node: NodeMetaInfo,
  saveDebugImages?: boolean,
  userParams?: Record<string, NodeParamValue>,
  detailedTimings?: boolean,
  executionMode?: NodeExecutionMode,
): Promise<NodeTestResult> {
  return invoke<NodeTestResult>("test_node", {
    node,
    saveDebugImages,
    userParams,
    detailedTimings,
    executionMode,
  });
}
