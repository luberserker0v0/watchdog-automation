import { beforeEach, expect, it } from "vitest";
import { hotkeyKeyCode, useRecorderStore } from "./useRecorderStore";

beforeEach(() => {
  useRecorderStore.setState({
    status: { state: "idle", session_id: null, elapsed_ms: 0, event_count: 0, target_label: null, reason: null, supported: true },
    actions: [], session: null, error: null, hotkeyError: null,
  });
});

it("extracts Windows key codes from configurable control shortcuts", () => {
  expect(hotkeyKeyCode("CommandOrControl+Alt+F9")).toBe(120);
  expect(hotkeyKeyCode("Ctrl+Shift+R")).toBe(82);
  expect(hotkeyKeyCode("Ctrl+Unknown")).toBeNull();
});

it("removes a review draft without changing native recording state", () => {
  useRecorderStore.setState({ actions: [
    { id: 1, offset_ms: 0, source: "mouse", action: { type: "Click", target: [10, 20] } },
    { id: 2, offset_ms: 100, source: "keyboard", action: { type: "ClickKey", key: [27] } },
  ] });
  useRecorderStore.getState().removeAction(1);
  expect(useRecorderStore.getState().actions.map((item) => item.id)).toEqual([2]);
});
