import { create } from "zustand";
import { persist } from "zustand/middleware";

export type CaptureMode = "screen" | "window";
export type CaptureEngine = "gdi" | "dxgi" | "graphics";
export type CaptureArea = "frame" | "client";
export type TimingGranularity = "off" | "summary" | "detailed";
export type RecognitionDevice = "cpu" | "directml";
export type PatrolRecoveryPolicy = "hint_only" | "redirection_only";

interface PreferencesState {
  mode: CaptureMode;
  saveToDisk: boolean;
  saveDebugImages: boolean;
  showFps: boolean;
  captureEngine: CaptureEngine;
  windowCaptureArea: CaptureArea;
  timingGranularity: TimingGranularity;
  recognitionDevice: RecognitionDevice;
  llmEndpoint: string;
  llmModel: string;
  llmApiKey: string;
  patrolEnabled: boolean;
  patrolCycleSeconds: number;
  patrolKHops: number;
  patrolThreshold: number;
  patrolMinMargin: number;
  patrolRecoveryPolicy: PatrolRecoveryPolicy;
  patrolMaxRedirects: number;
  taskNotificationsEnabled: boolean;
  recorderNotificationsEnabled: boolean;
  recorderStartStopHotkey: string;
  recorderPauseHotkey: string;
  recorderCaptureText: boolean;
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
  setLlmEndpoint: (endpoint: string) => void;
  setLlmModel: (model: string) => void;
  setLlmApiKey: (apiKey: string) => void;
  setPatrolEnabled: (enabled: boolean) => void;
  setPatrolCycleSeconds: (seconds: number) => void;
  setPatrolKHops: (hops: number) => void;
  setPatrolThreshold: (threshold: number) => void;
  setPatrolMinMargin: (margin: number) => void;
  setPatrolRecoveryPolicy: (policy: PatrolRecoveryPolicy) => void;
  setPatrolMaxRedirects: (count: number) => void;
  setTaskNotificationsEnabled: (enabled: boolean) => void;
  setRecorderNotificationsEnabled: (enabled: boolean) => void;
  setRecorderStartStopHotkey: (hotkey: string) => void;
  setRecorderPauseHotkey: (hotkey: string) => void;
  setRecorderCaptureText: (enabled: boolean) => void;
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
      llmEndpoint: "",
      llmModel: "",
      llmApiKey: "",
      patrolEnabled: false,
      patrolCycleSeconds: 20,
      patrolKHops: 1,
      patrolThreshold: 0.85,
      patrolMinMargin: 0.1,
      patrolRecoveryPolicy: "hint_only",
      patrolMaxRedirects: 1,
      taskNotificationsEnabled: false,
      recorderNotificationsEnabled: false,
      recorderStartStopHotkey: "CommandOrControl+Alt+F9",
      recorderPauseHotkey: "CommandOrControl+Alt+F10",
      recorderCaptureText: false,
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
      setLlmEndpoint: (llmEndpoint) => set({ llmEndpoint }),
      setLlmModel: (llmModel) => set({ llmModel }),
      setLlmApiKey: (llmApiKey) => set({ llmApiKey }),
      setPatrolEnabled: (patrolEnabled) => set({ patrolEnabled }),
      setPatrolCycleSeconds: (seconds) => set({ patrolCycleSeconds: Number.isFinite(seconds) ? Math.min(3600, Math.max(1, Math.round(seconds))) : 20 }),
      setPatrolKHops: (hops) => set({ patrolKHops: Number.isFinite(hops) ? Math.min(4, Math.max(1, Math.round(hops))) : 1 }),
      setPatrolThreshold: (threshold) => set({ patrolThreshold: Number.isFinite(threshold) ? Math.min(1, Math.max(0, threshold)) : 0.85 }),
      setPatrolMinMargin: (margin) => set({ patrolMinMargin: Number.isFinite(margin) ? Math.min(1, Math.max(0, margin)) : 0.1 }),
      setPatrolRecoveryPolicy: (patrolRecoveryPolicy) => set({ patrolRecoveryPolicy }),
      setPatrolMaxRedirects: (count) => set({ patrolMaxRedirects: Number.isFinite(count) ? Math.min(10, Math.max(1, Math.round(count))) : 1 }),
      setTaskNotificationsEnabled: (taskNotificationsEnabled) => set({ taskNotificationsEnabled }),
      setRecorderNotificationsEnabled: (recorderNotificationsEnabled) => set({ recorderNotificationsEnabled }),
      setRecorderStartStopHotkey: (recorderStartStopHotkey) => set({ recorderStartStopHotkey }),
      setRecorderPauseHotkey: (recorderPauseHotkey) => set({ recorderPauseHotkey }),
      setRecorderCaptureText: (recorderCaptureText) => set({ recorderCaptureText }),
      setCapturePanelVisible: (capturePanelVisible) => set({ capturePanelVisible }),
      setPreviewWindowSize: (size) => set({ previewWindowSize: normalizePreviewWindowSize(size) }),
    }),
    {
      name: "watchdog:settings",
      // saveToDisk and saveDebugImages are session-only: they reset to off
      // on every window open, so they are never written to storage.
      partialize: ({ mode, showFps, captureEngine, windowCaptureArea, timingGranularity, recognitionDevice, llmEndpoint, llmModel, patrolEnabled, patrolCycleSeconds, patrolKHops, patrolThreshold, patrolMinMargin, patrolRecoveryPolicy, patrolMaxRedirects, taskNotificationsEnabled, recorderNotificationsEnabled, recorderStartStopHotkey, recorderPauseHotkey, recorderCaptureText, capturePanelVisible, previewWindowSize }) => ({
        mode,
        showFps,
        captureEngine,
        windowCaptureArea,
        timingGranularity,
        recognitionDevice,
        llmEndpoint,
        llmModel,
        patrolEnabled,
        patrolCycleSeconds,
        patrolKHops,
        patrolThreshold,
        patrolMinMargin,
        patrolRecoveryPolicy,
        patrolMaxRedirects,
        taskNotificationsEnabled,
        recorderNotificationsEnabled,
        recorderStartStopHotkey,
        recorderPauseHotkey,
        recorderCaptureText,
        capturePanelVisible,
        previewWindowSize,
      }),
      merge: (persisted, current) => {
        const { patrolMode: legacyPatrolMode, ...stored } = persisted as Partial<PreferencesState> & { patrolMode?: string };
        return {
          ...current,
          ...stored,
          // Ignore stale true values written by older versions.
          saveToDisk: false,
          saveDebugImages: false,
          llmApiKey: "",
          patrolEnabled: stored.patrolEnabled === true && legacyPatrolMode !== "record",
          patrolCycleSeconds: Number.isFinite(stored.patrolCycleSeconds) ? Math.min(3600, Math.max(1, Math.round(stored.patrolCycleSeconds!))) : 20,
          patrolKHops: Number.isFinite(stored.patrolKHops) ? Math.min(4, Math.max(1, Math.round(stored.patrolKHops!))) : 1,
          patrolThreshold: Number.isFinite(stored.patrolThreshold) ? Math.min(1, Math.max(0, stored.patrolThreshold!)) : 0.85,
          patrolMinMargin: Number.isFinite(stored.patrolMinMargin) ? Math.min(1, Math.max(0, stored.patrolMinMargin!)) : 0.1,
          patrolRecoveryPolicy: stored.patrolRecoveryPolicy === "redirection_only" ? "redirection_only" : "hint_only",
          patrolMaxRedirects: Number.isFinite(stored.patrolMaxRedirects) ? Math.min(10, Math.max(1, Math.round(stored.patrolMaxRedirects!))) : 1,
          taskNotificationsEnabled: stored.taskNotificationsEnabled === true,
          recorderNotificationsEnabled: stored.recorderNotificationsEnabled === true,
          recorderStartStopHotkey: stored.recorderStartStopHotkey || "CommandOrControl+Alt+F9",
          recorderPauseHotkey: stored.recorderPauseHotkey || "CommandOrControl+Alt+F10",
          recorderCaptureText: stored.recorderCaptureText === true,
          capturePanelVisible: stored.capturePanelVisible ?? true,
          previewWindowSize: normalizePreviewWindowSize(stored.previewWindowSize),
        };
      },
    },
  ),
);
