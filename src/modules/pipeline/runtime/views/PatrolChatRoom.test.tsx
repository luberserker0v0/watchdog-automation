import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import PatrolChatRoom from "./PatrolChatRoom";
import { usePatrolChatStore, type PatrolChatEvent } from "../stores/usePatrolChatStore";

const invoke = vi.hoisted(() => vi.fn(async () => "data:image/jpeg;base64,aGVsbG8="));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

function event(overrides: Partial<PatrolChatEvent> = {}): PatrolChatEvent {
  return {
    id: 1, run_id: "run-1", sample_id: "run-1:1:12", occurred_at_ms: Date.now(),
    sender: "patrol", kind: "request", summary: "Sent local search to VLM",
    node_id: "RecognizeTitle", target: "Screen 0", session: 1, frame: 12, tier: "local",
    content: [
      { type: "text", label: "System prompt", text: "Exact system instruction" },
      { type: "image", label: "Current capture", image_id: 42 },
    ],
    ...overrides,
  };
}

beforeEach(() => {
  invoke.mockClear();
  usePatrolChatStore.setState({ messages: [], runStatuses: {}, selectedRunId: "latest", autoScroll: false });
});

describe("Patrol Chat Room", () => {
  it("places Patrol and VLM messages on opposite sides and keeps details and images closed", async () => {
    const user = userEvent.setup();
    usePatrolChatStore.getState().append(event());
    usePatrolChatStore.getState().append(event({
      id: 2, sender: "vlm", kind: "response", summary: "VLM scored 2 candidates",
      content: [{ type: "score", node_id: "RecognizeTitle", score: 0.98 }, { type: "text", label: "Exact VLM response", text: "{scores: ...}" }],
    }));
    render(<PatrolChatRoom />);
    expect(screen.getByRole("article", { name: "patrol request" })).toHaveClass("patrol");
    expect(screen.getByRole("article", { name: "vlm response" })).toHaveClass("vlm");
    expect(screen.getAllByText(/Run run-1 · RecognizeTitle · frame 12/)).toHaveLength(1);
    expect(screen.queryByText("Exact system instruction")).toBeNull();
    expect(invoke).not.toHaveBeenCalled();
    await user.click(within(screen.getByRole("article", { name: "patrol request" })).getByText("Show details"));
    expect(screen.getByText("Exact system instruction")).toBeVisible();
    expect(await screen.findByRole("img", { name: "Current capture" })).toBeVisible();
    expect(invoke).toHaveBeenCalledWith("get_patrol_chat_image", { imageId: 42 });
    await user.click(within(screen.getByRole("article", { name: "vlm response" })).getByText("Show details"));
    expect(screen.getByText("98%")).toBeVisible();
    expect(screen.getByText("{scores: ...}")).toBeVisible();
  });

  it("filters by run and clears session history", async () => {
    const user = userEvent.setup();
    usePatrolChatStore.getState().append(event());
    usePatrolChatStore.getState().append(event({ id: 3, run_id: "run-2", sample_id: "run-2:1:4", frame: 4, summary: "Second run capture" }));
    render(<PatrolChatRoom />);
    expect(screen.getByText("Second run capture")).toBeVisible();
    expect(screen.queryByText("Sent local search to VLM")).toBeNull();
    await user.selectOptions(screen.getByRole("combobox", { name: "Run" }), "all");
    expect(screen.getByText("Sent local search to VLM")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "Clear session" }));
    expect(screen.getByText(/No Patrol exchange yet/)).toBeVisible();
    expect(invoke).toHaveBeenCalledWith("clear_patrol_chat_images");
  });

  it("explains when an expanded image was evicted from the session cache", async () => {
    invoke.mockRejectedValueOnce(new Error("evicted"));
    usePatrolChatStore.getState().append(event());
    render(<PatrolChatRoom />);
    expect(invoke).not.toHaveBeenCalled();
    await userEvent.setup().click(screen.getByText("Show details"));
    expect(await screen.findByRole("status")).toHaveTextContent("Image preview is no longer available");
  });
});
