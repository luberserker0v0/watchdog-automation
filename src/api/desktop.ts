import { invoke } from "@tauri-apps/api/core";

export interface DesktopInfo {
  id: string;
  name: string | null;
  index: number;
  is_current: boolean;
}

export async function getDesktops(): Promise<DesktopInfo[]> {
  return await invoke("get_desktops");
}

export async function getCurrentDesktop(): Promise<DesktopInfo> {
  return await invoke("get_current_desktop");
}
