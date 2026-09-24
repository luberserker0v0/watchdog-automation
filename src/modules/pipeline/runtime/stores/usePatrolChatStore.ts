import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { create } from "zustand";

export type PatrolChatContent =
  | { type: "text"; label: string; text: string }
  | { type: "image"; label: string; image_id: number }
  | { type: "score"; node_id: string; score: number }
  | { type: "field"; label: string; value: string };

export interface PatrolChatEvent {
  id: number;
  run_id: string;
  sample_id: string;
  occurred_at_ms: number;
  sender: "patrol" | "vlm";
  kind: PatrolChatKind;
  summary: string;
  node_id: string;
  target: string;
  session: number;
  frame: number;
  tier: "local" | "expanded" | "action" | "recovery" | null;
  content: PatrolChatContent[];
}

export const PATROL_CHAT_KINDS = [
  "trigger", "capture", "search", "reference", "request", "response", "expansion", "report",
  "reference_save", "error", "action_evidence", "action_request", "action_response", "action_report",
  "location_handoff", "location_unresolved",
  "format_retry", "transport_retry", "validation_error", "recovery_trigger", "recovery_candidates", "recovery_request",
  "recovery_response", "recovery_report",
] as const;
export type PatrolChatKind = typeof PATROL_CHAT_KINDS[number];

interface PatrolChatState {
  messages: PatrolChatEvent[];
  runStatuses: Record<string, string>;
  selectedRunId: "latest" | "all" | string;
  autoScroll: boolean;
  append: (event: PatrolChatEvent) => void;
  setRunStatus: (runId: string, status: string) => void;
  selectRun: (runId: string) => void;
  setAutoScroll: (enabled: boolean) => void;
  clear: () => Promise<void>;
}

const MAX_MESSAGES = 1000;

export const usePatrolChatStore = create<PatrolChatState>((set) => ({
  messages: [],
  runStatuses: {},
  selectedRunId: "latest",
  autoScroll: true,
  append: (event) => set((state) => ({
    messages: state.messages.some((message) => message.id === event.id)
      ? state.messages
      : [...state.messages, event].slice(-MAX_MESSAGES),
  })),
  setRunStatus: (runId, status) => set((state) => ({ runStatuses: { ...state.runStatuses, [runId]: status } })),
  selectRun: (selectedRunId) => set({ selectedRunId }),
  setAutoScroll: (autoScroll) => set({ autoScroll }),
  clear: async () => {
    set({ messages: [], runStatuses: {}, selectedRunId: "latest" });
    try { await invoke("clear_patrol_chat_images"); } catch { /* Browser preview has no Tauri bridge. */ }
  },
}));

let starting: Promise<boolean> | null = null;
let listeners: UnlistenFn[] = [];

export function ensurePatrolChatListener(): Promise<boolean> {
  if (listeners.length === 2) return Promise.resolve(true);
  if (starting) return starting;
  starting = Promise.allSettled([
    listen<PatrolChatEvent>("patrol-chat-event", ({ payload }) => usePatrolChatStore.getState().append(payload)),
    listen<{ run_id: string; state: string }>("pipeline-status", ({ payload }) =>
      usePatrolChatStore.getState().setRunStatus(payload.run_id, payload.state)),
  ]).then((results) => {
    if (results.every((result) => result.status === "fulfilled")) {
      listeners = results.map((result) => (result as PromiseFulfilledResult<UnlistenFn>).value);
      return true;
    }
    for (const result of results) if (result.status === "fulfilled") result.value();
    return false;
  }).finally(() => { starting = null; });
  return starting;
}

export async function loadPatrolChatImage(imageId: number): Promise<string> {
  return invoke<string>("get_patrol_chat_image", { imageId });
}
