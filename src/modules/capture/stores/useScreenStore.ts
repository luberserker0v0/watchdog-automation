import { create } from "zustand";
import type { ScreenInfo } from "../../../api/types";
import { getScreens, captureScreen, saveCapture } from "../../../api/screen";
import { usePreferencesStore } from "../../shell/stores/usePreferencesStore";
import { getTimestamp } from "../utils";
import { useCaptureFrameStore } from "./useCaptureFrameStore";

interface ScreenState {
  screens: ScreenInfo[];
  selectedIndex: number | null;
  status: string;
  isLoading: boolean;
  fetchScreens: () => Promise<void>;
  setSelectedIndex: (index: number) => void;
  capture: () => Promise<void>;
}

let latestScreenRequest = 0;

export const useScreenStore = create<ScreenState>((set, get) => ({
  screens: [],
  selectedIndex: null,
  status: "就緒",
  isLoading: false,

  fetchScreens: async () => {
    const request = ++latestScreenRequest;
    set({ isLoading: true });
    try {
      const screens = await getScreens();
      if (request !== latestScreenRequest) return;
      const selectedIndex = get().selectedIndex;
      set({
        screens,
        selectedIndex: selectedIndex !== null && screens.some((screen) => screen.index === selectedIndex)
          ? selectedIndex
          : null,
        status: `已載入 ${screens.length} 個螢幕`,
        isLoading: false,
      });
    } catch (e) {
      if (request === latestScreenRequest) {
        set({ status: `載入失敗: ${e}`, isLoading: false });
      }
    }
  },

  setSelectedIndex: (index) => set({ selectedIndex: index }),

  capture: async () => {
    const { selectedIndex } = get();
    if (selectedIndex === null) {
      set({ status: "請先選擇目標" });
      return;
    }

    try {
      set({ status: "截圖中..." });
      const buf = await captureScreen(selectedIndex);
      const blob = new Blob([new Uint8Array(buf)], { type: "image/png" });
      const url = URL.createObjectURL(blob);
      useCaptureFrameStore.getState().setCapturedFrame("screen", url, buf);
      set({ status: "截圖成功" });

      const saveToDisk = usePreferencesStore.getState().saveToDisk;
      if (saveToDisk) {
        const filename = `screen_${getTimestamp()}.png`;
        await saveCapture(buf, filename);
      }
    } catch (e) {
      set({ status: `截圖失敗: ${e}` });
    }
  },
}));
