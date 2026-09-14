import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useDebugStore } from "../../stores/useDebugStore";
import { createNodeEmulatorDraft } from "./nodeEmulatorModel";

const api = vi.hoisted(() => ({ testNode: vi.fn() }));
vi.mock("../../../../api/nodeTest", () => api);

import NodeEmulatorTool from "./NodeEmulatorTool";

const result = { success: true, matched: true, confidence: 1, box_region: null, actions_fired: 0, duration_ms: 4, error: null, candidates: [] };

describe("NodeEmulatorTool", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    api.testNode.mockResolvedValue(result);
    useDebugStore.setState({ nodeEmulatorDraft: createNodeEmulatorDraft({ type: "screen", index: 0 }), nodeEmulatorInitialized: true });
  });

  it("uses explicit recognition-only and full execution modes", async () => {
    const user = userEvent.setup();
    render(<NodeEmulatorTool currentTarget={{ type: "screen", index: 0 }} />);
    await user.click(screen.getByRole("button", { name: "Recognize Only" }));
    expect(api.testNode).toHaveBeenLastCalledWith(expect.objectContaining({ id: "EmulatedNode" }), false, {}, false, "recognize_only");
    await user.click(screen.getByRole("button", { name: "Run Full Node" }));
    expect(api.testNode).toHaveBeenLastCalledWith(expect.objectContaining({ id: "EmulatedNode" }), false, {}, false, "full");
  });

  it("does not execute disabled or invalid drafts", async () => {
    useDebugStore.setState({ nodeEmulatorDraft: { ...createNodeEmulatorDraft(), enabled: false, id: "bad id" }, nodeEmulatorInitialized: true });
    render(<NodeEmulatorTool currentTarget={{ type: "screen", index: 0 }} />);
    expect(screen.getByRole("button", { name: "Recognize Only" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Run Full Node" })).toBeDisabled();
    expect(screen.getByText(/Disabled nodes cannot be executed/)).toBeInTheDocument();
    expect(api.testNode).not.toHaveBeenCalled();
  });

  it("keeps capture rendering isolated while emulator fields change", async () => {
    let frameRenders = 0;
    const CaptureFrame = () => { frameRenders += 1; return <div>capture frame</div>; };
    const user = userEvent.setup();
    render(<><CaptureFrame /><NodeEmulatorTool currentTarget={{ type: "screen", index: 0 }} /></>);
    const id = screen.getByLabelText("ID");
    await user.clear(id);
    await user.type(id, "EditedNode");
    expect(frameRenders).toBe(1);
    expect(useDebugStore.getState().nodeEmulatorDraft.id).toBe("EditedNode");
  });
});
