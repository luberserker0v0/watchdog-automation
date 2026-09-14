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
  capturePanelVisible: boolean;
  previewWindowSize: [number, number];
  setMode: (mode: CaptureMode) => void;
  setSaveToDisk: (enabled: boolean) => void;
  setSaveDebugImages: (enabled: boolean) => void;
  setShowFps: (enabled: boolean) => void;
  setCaptureEngine: (engine: CaptureEngine) => void;
  setWindowCaptureArea: (area: CaptureArea) => void;
  setTimingGranularity: (granularity: TimingGranularity) => void;
  setRecognitionDevice: (device: RecognitionDevice) => void;
  setCapturePanelVisible: (visible: boolean) => void;
  setPreviewWindowSize: (size: [number, number]) => void;
}

export function normalizePreviewWindowSize(size: unknown): [number, number] {
  if (!Array.isArray(size) || size.length !== 2) return [1200, 800];
  const width = Number(size[0]);
  const height = Number(size[1]);
  if (!Number.isFinite(width) || !Number.isFinite(height)) return [1200, 800];
  return [Math.max(600, Math.round(width)), Math.max(400, Math.round(height))];
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
      capturePanelVisible: true,
      previewWindowSize: [1200, 800],
      setMode: (mode) => set({ mode }),
      setSaveToDisk: (saveToDisk) => set({ saveToDisk }),
      setSaveDebugImages: (saveDebugImages) => set({ saveDebugImages }),
      setShowFps: (showFps) => set({ showFps }),
      setCaptureEngine: (captureEngine) => set({ captureEngine }),
      setWindowCaptureArea: (windowCaptureArea) => set({ windowCaptureArea }),
      setTimingGranularity: (timingGranularity) => set({ timingGranularity }),
      setRecognitionDevice: (recognitionDevice) => set({ recognitionDevice }),
      setCapturePanelVisible: (capturePanelVisible) => set({ capturePanelVisible }),
      setPreviewWindowSize: (size) => set({ previewWindowSize: normalizePreviewWindowSize(size) }),
    }),
    {
      name: "watchdog:settings",
      partialize: ({ mode, saveToDisk, saveDebugImages, showFps, captureEngine, windowCaptureArea, timingGranularity, recognitionDevice, capturePanelVisible, previewWindowSize }) => ({
        mode,
        saveToDisk,
        saveDebugImages,
        showFps,
        captureEngine,
        windowCaptureArea,
        timingGranularity,
        recognitionDevice,
        capturePanelVisible,
        previewWindowSize,
      }),
      merge: (persisted, current) => {
        const stored = persisted as Partial<PreferencesState>;
        return {
          ...current,
          ...stored,
          capturePanelVisible: stored.capturePanelVisible ?? true,
          previewWindowSize: normalizePreviewWindowSize(stored.previewWindowSize),
        };
      },
    },
  ),
);
