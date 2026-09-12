import { create } from "zustand";
import { persist } from "zustand/middleware";

export type CaptureMode = "screen" | "window";
export type CaptureEngine = "gdi" | "dxgi" | "graphics";
export type CaptureArea = "frame" | "client";
export type TimingGranularity = "off" | "summary" | "detailed";
export type RecognitionDevice = "cpu" | "directml";

interface PreferencesState {
  mode: CaptureMode;
  saveToDisk: boolean;
  saveDebugImages: boolean;
  showFps: boolean;
  captureEngine: CaptureEngine;
  windowCaptureArea: CaptureArea;
  timingGranularity: TimingGranularity;
  recognitionDevice: RecognitionDevice;
  setMode: (mode: CaptureMode) => void;
  setSaveToDisk: (enabled: boolean) => void;
  setSaveDebugImages: (enabled: boolean) => void;
  setShowFps: (enabled: boolean) => void;
  setCaptureEngine: (engine: CaptureEngine) => void;
  setWindowCaptureArea: (area: CaptureArea) => void;
  setTimingGranularity: (granularity: TimingGranularity) => void;
  setRecognitionDevice: (device: RecognitionDevice) => void;
}

export const usePreferencesStore = create<PreferencesState>()(
  persist(
    (set) => ({
      mode: "screen",
      saveToDisk: false,
      saveDebugImages: false,
      showFps: false,
      captureEngine: "graphics",
      windowCaptureArea: "frame",
      timingGranularity: "summary",
      recognitionDevice: "cpu",
      setMode: (mode) => set({ mode }),
      setSaveToDisk: (saveToDisk) => set({ saveToDisk }),
      setSaveDebugImages: (saveDebugImages) => set({ saveDebugImages }),
      setShowFps: (showFps) => set({ showFps }),
      setCaptureEngine: (captureEngine) => set({ captureEngine }),
      setWindowCaptureArea: (windowCaptureArea) => set({ windowCaptureArea }),
      setTimingGranularity: (timingGranularity) => set({ timingGranularity }),
      setRecognitionDevice: (recognitionDevice) => set({ recognitionDevice }),
    }),
    {
      name: "watchdog:settings",
      partialize: ({ mode, saveToDisk, saveDebugImages, showFps, captureEngine, windowCaptureArea, timingGranularity, recognitionDevice }) => ({
        mode,
        saveToDisk,
        saveDebugImages,
        showFps,
        captureEngine,
        windowCaptureArea,
        timingGranularity,
        recognitionDevice,
      }),
    },
  ),
);
