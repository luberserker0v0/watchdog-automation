export interface ScreenInfo {
  index: number;
  name: string;
  width: number;
  height: number;
  is_primary: boolean;
}

export interface WindowInfo {
  index: number;
  title: string;
  app_name: string;
  width: number;
  height: number;
  hwnd?: number;
  desktop_id?: string;
}
