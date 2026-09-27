import { listen } from "@tauri-apps/api/event";
import { create } from "zustand";
import {
  clearActionRecording,
  getRecorderStatus,
  pauseActionRecording,
  resumeActionRecording,
  startActionRecording,
  stopActionRecording,
  type RecordedActionDraft,
  type RecorderStatus,
  type RecordingConfig,
  type RecordingSession,
  type RecordingTarget,
} from "../../../api/recorder";
import { usePreferencesStore } from "../../shell/stores/usePreferencesStore";
import { useScreenStore } from "../../capture/stores/useScreenStore";
import { useWindowStore } from "../../capture/stores/useWindowStore";
import { useStreamStore } from "../../capture/stores/useStreamStore";
import { useLogStore } from "../../observability/stores/useLogStore";
import { notifyTask } from "../../notifications/services/taskNotifier";
import { assertRuntimeStartAllowed } from "../../updater/services/runtimeGuard";

const IDLE_STATUS: RecorderStatus = {
  state: "idle", session_id: null, elapsed_ms: 0, event_count: 0,
  target_label: null, reason: null, supported: true,
};

interface RecorderStore {
  status: RecorderStatus;
  actions: RecordedActionDraft[];
  session: RecordingSession | null;
  error: string | null;
  hotkeyError: string | null;
  shortcutCaptureActive: boolean;
  initialize: () => Promise<void>;
  start: () => Promise<void>;
  pause: () => Promise<void>;
  resume: () => Promise<void>;
  stop: () => Promise<void>;
  toggle: () => Promise<void>;
  togglePause: () => Promise<void>;
  clear: () => Promise<void>;
  removeAction: (id: number) => void;
  setHotkeyError: (error: string | null) => void;
  setShortcutCaptureActive: (active: boolean) => void;
}

let listeners: Promise<void> | null = null;

function errorText(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export function hotkeyKeyCode(hotkey: string): number | null {
  const parts = hotkey.split("+");
  const key = parts[parts.length - 1]?.trim().toUpperCase();
  if (!key) return null;
  const f = /^F(\d{1,2})$/.exec(key);
  if (f) {
    const number = Number(f[1]);
    return number >= 1 && number <= 24 ? 111 + number : null;
  }
  return key.length === 1 ? key.charCodeAt(0) : null;
}

function selectedTarget(): RecordingTarget {
  const preferences = usePreferencesStore.getState();
  const stream = useStreamStore.getState();
  if (preferences.mode === "screen") {
    const { selectedIndex, screens } = useScreenStore.getState();
    if (selectedIndex === null) throw new Error("Select a screen before starting Action Recorder.");
    const screen = screens.find((item) => item.index === selectedIndex);
    return { type: "screen", index: selectedIndex, label: screen?.name || `Screen ${selectedIndex + 1}` };
  }
  const { selectedIndex, windows } = useWindowStore.getState();
  if (selectedIndex === null) throw new Error("Select a window before starting Action Recorder.");
  const window = windows[selectedIndex];
  if (!window?.hwnd) throw new Error("The selected window is no longer available.");
  const size = stream.frameSize ?? [window.width, window.height];
  return {
    type: "window", hwnd: window.hwnd, title: window.title || window.app_name,
    capture_area: preferences.windowCaptureArea, frame_width: size[0], frame_height: size[1],
  };
}

export const useRecorderStore = create<RecorderStore>((set, get) => ({
  status: IDLE_STATUS,
  actions: [],
  session: null,
  error: null,
  hotkeyError: null,
  shortcutCaptureActive: false,

  initialize: async () => {
    if (!listeners) {
      listeners = Promise.all([
        listen<RecorderStatus>("recorder-status", ({ payload }) => set({ status: payload })),
        listen<RecordedActionDraft>("recorder-action", ({ payload }) => set((state) => {
          const index = state.actions.findIndex((item) => item.id === payload.id);
          if (index < 0) return { actions: [...state.actions, payload] };
          const actions = [...state.actions];
          actions[index] = payload;
          return { actions };
        })),
        listen<string>("recorder-error", ({ payload }) => set({ error: payload })),
      ]).then(() => undefined).catch(() => undefined);
    }
    await listeners;
    try { set({ status: await getRecorderStatus() }); } catch { /* Browser preview has no native recorder. */ }
  },

  start: async () => {
    try {
      assertRuntimeStartAllowed();
      const preferences = usePreferencesStore.getState();
      const keys = [hotkeyKeyCode(preferences.recorderStartStopHotkey), hotkeyKeyCode(preferences.recorderPauseHotkey)]
        .filter((value): value is number => value !== null);
      const config: RecordingConfig = {
        target: selectedTarget(), capture_text: preferences.recorderCaptureText,
        excluded_key_codes: [...new Set(keys)],
      };
      const status = await startActionRecording(config);
      set({ status, actions: [], session: null, error: null });
      useLogStore.getState().appendLog({ source: "debug", label: "Action Recorder", detail: `Recording started for ${status.target_label}`, result: "info" });
      if (preferences.recorderNotificationsEnabled && status.session_id) {
        void notifyTask({ id: `recorder-start:${status.session_id}`, type: "recording_started", title: "Action recording started", body: status.target_label || "Recording selected target" })
          .then((sent) => { if (!sent) useLogStore.getState().appendLog({ source: "debug", label: "Task Notifier", detail: "Recorder start notification was not delivered. Check notification permission in Settings.", result: "info" }); });
      }
    } catch (error) {
      const message = errorText(error);
      set({ error: message });
      useLogStore.getState().appendLog({ source: "debug", label: "Action Recorder", detail: message, result: "error" });
    }
  },

  pause: async () => {
    try { set({ status: await pauseActionRecording(), error: null }); }
    catch (error) { set({ error: errorText(error) }); }
  },
  resume: async () => {
    try { set({ status: await resumeActionRecording(), error: null }); }
    catch (error) { set({ error: errorText(error) }); }
  },
  stop: async () => {
    try {
      const session = await stopActionRecording();
      set({ session, actions: session.actions, status: { ...get().status, state: "idle", elapsed_ms: session.duration_ms, event_count: session.actions.length, reason: null }, error: null });
      useLogStore.getState().appendLog({ source: "debug", label: "Action Recorder", detail: `Recording stopped with ${session.actions.length} action(s)`, result: "success" });
      if (usePreferencesStore.getState().recorderNotificationsEnabled) {
        void notifyTask({ id: `recorder-stop:${session.id}`, type: "recording_stopped", title: "Action recording stopped", body: `${session.actions.length} actions captured in ${Math.round(session.duration_ms / 1000)}s.` })
          .then((sent) => { if (!sent) useLogStore.getState().appendLog({ source: "debug", label: "Task Notifier", detail: "Recorder stop notification was not delivered. Check notification permission in Settings.", result: "info" }); });
      }
    } catch (error) { set({ error: errorText(error) }); }
  },
  toggle: async () => {
    if (get().status.state === "idle") await get().start(); else await get().stop();
  },
  togglePause: async () => {
    if (get().status.state === "recording") await get().pause();
    else if (get().status.state === "paused") await get().resume();
  },
  clear: async () => {
    try { await clearActionRecording(); set({ actions: [], session: null, error: null, status: IDLE_STATUS }); }
    catch (error) { set({ error: errorText(error) }); }
  },
  removeAction: (id) => set((state) => ({ actions: state.actions.filter((item) => item.id !== id) })),
  setHotkeyError: (hotkeyError) => set({ hotkeyError }),
  setShortcutCaptureActive: (shortcutCaptureActive) => set({ shortcutCaptureActive }),
}));

export function resetRecorderListenersForTests(): void {
  listeners = null;
}
