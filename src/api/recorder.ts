import { invoke } from "@tauri-apps/api/core";
import type { Action, CaptureArea } from "./pipeline";

export type RecorderState = "idle" | "recording" | "paused" | "stopping";

export type RecordingTarget =
  | { type: "screen"; index: number; label: string }
  | { type: "window"; hwnd: number; title: string; capture_area: CaptureArea; frame_width: number; frame_height: number };

export interface RecordingConfig {
  target: RecordingTarget;
  capture_text: boolean;
  excluded_key_codes: number[];
}

export interface RecordedActionDraft {
  id: number;
  offset_ms: number;
  action: Action;
  source: string;
  warning?: string;
}

export interface RecordingSession {
  id: string;
  started_unix_ms: number;
  duration_ms: number;
  target: RecordingTarget;
  actions: RecordedActionDraft[];
}

export interface RecorderStatus {
  state: RecorderState;
  session_id: string | null;
  elapsed_ms: number;
  event_count: number;
  target_label: string | null;
  reason: string | null;
  supported: boolean;
}

export const getRecorderStatus = () => invoke<RecorderStatus>("get_recorder_status");
export const startActionRecording = (config: RecordingConfig) => invoke<RecorderStatus>("start_action_recording", { config });
export const pauseActionRecording = () => invoke<RecorderStatus>("pause_action_recording");
export const resumeActionRecording = () => invoke<RecorderStatus>("resume_action_recording");
export const stopActionRecording = () => invoke<RecordingSession>("stop_action_recording");
export const clearActionRecording = () => invoke<void>("clear_action_recording");
