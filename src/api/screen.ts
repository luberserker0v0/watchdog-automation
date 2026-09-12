import { invoke } from "@tauri-apps/api/core";
import type { ScreenInfo } from "./types";

export async function getScreens(): Promise<ScreenInfo[]> {
  return invoke("get_screens");
}

export async function captureScreen(index: number): Promise<number[]> {
  return invoke("capture_screen", { index });
}

export async function saveCapture(buf: number[], filename: string): Promise<string> {
  return invoke("save_capture", { buf, filename });
}
