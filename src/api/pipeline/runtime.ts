import { invoke } from "@tauri-apps/api/core";
import type { PatrolConfig, PipelineStartError, PipelineStatus, VlmConnection } from "./models";

export async function startPipeline(path: string, userParams?: Record<string, Record<string, unknown>>, saveDebugImages?: boolean, detailedTimings?: boolean, patrolConfig?: PatrolConfig, vlmConfig?: VlmConnection): Promise<string> {
  return invoke<string>("start_pipeline", { path, userParams, saveDebugImages, detailedTimings, patrolConfig, vlmConfig });
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

