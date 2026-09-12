import { invoke } from "@tauri-apps/api/core";
import type { WindowInfo } from "./types";

export type WindowState = "Foreground" | "Background";

export interface WindowDetailInfo {
  hwnd: number;
  title: string;
  class_name: string;
  process_name: string;
  width: number;
  height: number;
}

export async function getWindows(filter?: WindowState, desktopId?: string): Promise<WindowInfo[]> {
  return invoke("get_windows", { filter, desktopId: desktopId ?? null });
}

export async function captureWindowByHwnd(hwnd: number, captureArea: "frame" | "client" = "frame"): Promise<number[]> {
  return invoke("capture_window_by_hwnd", { hwnd, captureArea });
}

export async function getWindowDetails(): Promise<WindowDetailInfo[]> {
  return invoke("get_window_details");
}
