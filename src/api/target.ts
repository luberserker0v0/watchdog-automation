import { invoke } from "@tauri-apps/api/core";

export async function openTargetWindow(): Promise<void> {
  return invoke("open_target_window");
}
