import { beforeEach, expect, it, vi } from "vitest";
import { ensurePatrolChatListener, usePatrolChatStore, type PatrolChatEvent } from "./usePatrolChatStore";

const listeners = vi.hoisted(() => new Map<string, (event: { payload: unknown }) => void>());
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (name: string, callback: (event: { payload: unknown }) => void) => {
    listeners.set(name, callback);
    return () => listeners.delete(name);
  }),
}));

beforeEach(() => usePatrolChatStore.setState({ messages: [], runStatuses: {}, selectedRunId: "latest" }));

it("collects Patrol chat events while the view is closed and ignores duplicates", async () => {
  expect(await ensurePatrolChatListener()).toBe(true);
  const event: PatrolChatEvent = {
    id: 9, run_id: "run-9", sample_id: "run-9:1:2", occurred_at_ms: 1000,
    sender: "patrol", kind: "capture", summary: "Captured screen", node_id: "A",
    target: "Screen 0", session: 1, frame: 2, tier: null, content: [],
  };
  listeners.get("patrol-chat-event")?.({ payload: event });
  listeners.get("patrol-chat-event")?.({ payload: event });
  listeners.get("pipeline-status")?.({ payload: { run_id: "run-9", state: "Running" } });
  expect(usePatrolChatStore.getState().messages).toEqual([event]);
  expect(usePatrolChatStore.getState().runStatuses["run-9"]).toBe("Running");
});
