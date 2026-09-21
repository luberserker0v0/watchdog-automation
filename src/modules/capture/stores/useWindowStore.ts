import { create } from "zustand";
import type { WindowInfo } from "../../../api/types";
import type { WindowState } from "../../../api/window";
import { getWindows, captureWindowByHwnd } from "../../../api/window";
import { saveCapture } from "../../../api/screen";
import { usePreferencesStore } from "../../shell/stores/usePreferencesStore";
import { useDesktopStore } from "./useDesktopStore";
import { getTimestamp } from "../utils";
import { useCaptureFrameStore } from "./useCaptureFrameStore";

interface WindowStateStore {
  windows: WindowInfo[];
  allWindows: WindowInfo[];
  selectedIndex: number | null;
  windowStateFilter: WindowState;
  status: string;
  isLoading: boolean;
  setWindowStateFilter: (filter: WindowState) => void;
  fetchWindows: () => Promise<void>;
  setSelectedIndex: (index: number) => void;
  capture: () => Promise<void>;
}

let latestWindowRequest = 0;

export const useWindowStore = create<WindowStateStore>((set, get) => ({
  windows: [],
  allWindows: [],
  selectedIndex: null,
  windowStateFilter: "Foreground",
  status: "就緒",
  isLoading: false,

  setWindowStateFilter: (filter) => {
    set({ windowStateFilter: filter, selectedIndex: null });
    get().fetchWindows();
  },

  fetchWindows: async () => {
    const request = ++latestWindowRequest;
    const { windowStateFilter } = get();
    const { selectedDesktopId } = useDesktopStore.getState();
    const selectedWindow = get().selectedIndex === null
      ? null
      : get().windows[get().selectedIndex!];
    set({ isLoading: true });
    try {
      const windows = await getWindows(windowStateFilter, selectedDesktopId ?? undefined);
      if (request !== latestWindowRequest) return;
      const reselectedIndex = selectedWindow?.hwnd
        ? windows.findIndex((window) => window.hwnd === selectedWindow.hwnd)
        : -1;
      set({
        allWindows: windows,
        windows,
        selectedIndex: reselectedIndex >= 0 ? reselectedIndex : null,
        status: `已載入 ${windows.length} 個視窗`,
        isLoading: false,
      });
    } catch (e) {
      if (request === latestWindowRequest) {
        set({ status: `載入失敗: ${e}`, isLoading: false });
      }
    }
  },

  setSelectedIndex: (index) => set({ selectedIndex: index }),

  capture: async () => {
    const { selectedIndex, windows } = get();
    if (selectedIndex === null) {
      set({ status: "請先選擇目標" });
      return;
    }

    const win = windows[selectedIndex];
    if (!win?.hwnd) {
      set({ status: "視窗無效" });
      return;
    }

    try {
      set({ status: "截圖中..." });
      const buf = await captureWindowByHwnd(
        win.hwnd,
        usePreferencesStore.getState().windowCaptureArea,
      );
      const blob = new Blob([new Uint8Array(buf)], { type: "image/png" });
      const url = URL.createObjectURL(blob);
      useCaptureFrameStore.getState().setCapturedFrame("window", url, buf);
      set({ status: "截圖成功" });

      const saveToDisk = usePreferencesStore.getState().saveToDisk;
      if (saveToDisk) {
        const filename = `window_${getTimestamp()}.png`;
        await saveCapture(buf, filename);
      }
    } catch (e) {
      set({ status: `截圖失敗: ${e}` });
    }
  },
}));
