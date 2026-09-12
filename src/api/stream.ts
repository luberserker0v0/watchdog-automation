import { invoke } from "@tauri-apps/api/core";

export interface StreamStatus {
  is_streaming: boolean;
  fps: number;
  target_type: string;
  target_index: number;
  start_time: number | null;
  engine: string;
  capture_area: "frame" | "client";
}

export interface StreamStoppedPayload {
  reason: "target_closed" | "capture_closed" | string;
  target_type: "screen" | "window";
  target_index: number;
}

export async function startStream(
  fps: number,
  targetType: string,
  targetIndex: number,
  targetHwnd: number | null = null,
  engine: string = "graphics",
  captureArea: "frame" | "client" = "frame",
): Promise<void> {
  await invoke("start_stream", { fps, targetType, targetIndex, targetHwnd, engine, captureArea });
}

export async function stopStream(): Promise<void> {
  await invoke("stop_stream");
}

export async function getStreamStatus(): Promise<StreamStatus> {
  return await invoke("get_stream_status");
}
