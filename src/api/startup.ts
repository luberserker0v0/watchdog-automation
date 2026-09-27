import { invoke } from "@tauri-apps/api/core";

export interface StartupProgress {
  percent: number;
  stage: string;
  detail?: string | null;
  complete: boolean;
}

export function initializeApp(recognitionDevice: "cpu" | "directml"): Promise<StartupProgress> {
  return invoke("initialize_app", { recognitionDevice });
}

export function getStartupProgress(): Promise<StartupProgress> {
  return invoke("get_startup_progress");
}

export function markMainWindowReady(): Promise<void> {
  return invoke("mark_main_window_ready");
}
