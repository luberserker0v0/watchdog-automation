import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, it, vi } from "vitest";
import { useRecorderStore } from "../stores/useRecorderStore";
import ActionRecorderView from "./ActionRecorderView";

const defaultTogglePause = useRecorderStore.getState().togglePause;

beforeEach(() => {
  useRecorderStore.setState({
    status: { state: "idle", session_id: "recording-1", elapsed_ms: 900, event_count: 1, target_label: "Login", reason: null, supported: true },
    actions: [{ id: 1, offset_ms: 420, source: "keyboard", action: { type: "InputText", input_text: "{{recorded_text}}" }, warning: "Printable text was masked." }],
    session: null, error: null, hotkeyError: null, togglePause: defaultTogglePause,
  });
});

it("formats a masked action and lets the user remove the draft", async () => {
  const user = userEvent.setup();
  render(<ActionRecorderView />);
  expect(screen.getByText("Typed text (masked)")).toBeTruthy();
  expect(screen.getByText("Printable text was masked.")).toBeTruthy();
  await user.click(screen.getByRole("button", { name: "Remove" }));
  expect(screen.getByText(/Select a capture target/)).toBeTruthy();
});

it("shows recording controls for the active state and invokes pause", async () => {
  const user = userEvent.setup();
  const pause = vi.fn().mockResolvedValue(undefined);
  useRecorderStore.setState({
    status: { state: "recording", session_id: "recording-1", elapsed_ms: 900, event_count: 1, target_label: "Login", reason: null, supported: true },
    togglePause: pause,
  });
  render(<ActionRecorderView />);
  expect(screen.getByRole("button", { name: "Stop" })).toBeTruthy();
  await user.click(screen.getByRole("button", { name: "Pause" }));
  expect(pause).toHaveBeenCalledOnce();
});
