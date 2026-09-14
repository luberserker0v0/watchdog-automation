import { invoke } from "@tauri-apps/api/core";

export interface StreamStatus {
  is_streaming: boolean;
  fps: number;
  target_type: string;
  target_index: number;
  start_time: number | null;
  engine: string;
  capture_area: "frame" | "client";
  owner: "none" | "manual" | "pipeline" | "node_test";
  owner_id: string | null;
  session_id: number;
  frame_width: number | null;
  frame_height: number | null;
  client_width: number | null;
  client_height: number | null;
}

export interface StreamFramePayload {
  session_id: number;
  sequence: number;
  target_type: "screen" | "window";
  target_index: number;
  capture_area: "frame" | "client";
  frame_width: number;
  frame_height: number;
  client_width: number | null;
  client_height: number | null;
}

export interface StreamStoppedPayload {
  reason: "target_closed" | "capture_closed" | string;
  target_type: "screen" | "window";
  target_index: number;
  session_id: number;
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
