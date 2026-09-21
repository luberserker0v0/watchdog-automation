import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useDebugStore } from "../../capture/stores/useDebugStore";
import { useOverlayStore } from "../../capture/stores/useOverlayStore";
import { createNodeEmulatorDraft } from "./nodeEmulatorModel";

const api = vi.hoisted(() => ({ testNode: vi.fn() }));
vi.mock("../../../api/nodeTest", () => api);

import NodeEmulatorTool from "./NodeEmulatorTool";

const result = { success: true, matched: true, confidence: 1, box_region: null, actions_fired: 0, duration_ms: 4, error: null, candidates: [] };

describe("NodeEmulatorTool", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    api.testNode.mockResolvedValue(result);
    useDebugStore.setState({ nodeEmulatorDraft: createNodeEmulatorDraft({ type: "screen", index: 0 }), nodeEmulatorInitialized: true });
    useOverlayStore.setState({ roi: null, resultRoi: null, imageDimensions: { width: 500, height: 400 } });
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

  it("does not offer individual tests for Endpoint", () => {
    useDebugStore.setState({ nodeEmulatorDraft: { ...createNodeEmulatorDraft(), kind: "Endpoint" }, nodeEmulatorInitialized: true });
    render(<NodeEmulatorTool currentTarget={{ type: "screen", index: 0 }} />);
    expect(screen.getByRole("button", { name: "Recognize Only" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Run Full Node" })).toBeDisabled();
    expect(screen.queryByText("Recognition")).not.toBeInTheDocument();
    expect(screen.getByText(/structural pipeline nodes/)).toBeInTheDocument();
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

  it("publishes the effective recognition ROI and matched result to Preview", async () => {
    const draft = createNodeEmulatorDraft({ type: "screen", index: 0 });
    draft.recognition = {
      ...draft.recognition,
      roi: "10,20,100,50",
      roiOffset: "5,6,7,8",
    };
    useDebugStore.setState({ nodeEmulatorDraft: draft, nodeEmulatorInitialized: true });
    api.testNode.mockResolvedValue({
      ...result,
      box_region: { x: 830, y: 440, width: 50, height: 20 },
      capture_box_region: { x: 30, y: 40, width: 50, height: 20 },
      capture_roi: { x: 15, y: 26, width: 107, height: 58 },
    });
    const user = userEvent.setup();
    const view = render(<NodeEmulatorTool currentTarget={{ type: "screen", index: 0 }} />);

    await user.click(screen.getByRole("button", { name: "Recognize Only" }));

    expect(useOverlayStore.getState().roi).toEqual({ x: 15, y: 26, w: 107, h: 58 });
    expect(useOverlayStore.getState().resultRoi).toEqual({ x: 30, y: 40, w: 50, h: 20 });

    view.unmount();
    expect(useOverlayStore.getState().roi).toBeNull();
    expect(useOverlayStore.getState().resultRoi).toBeNull();
  });

  it("restores the backend ROI after Node Test replaces the stream session", async () => {
    let finish!: (value: typeof result & { capture_roi: { x: number; y: number; width: number; height: number } }) => void;
    api.testNode.mockReturnValue(new Promise((resolve) => { finish = resolve; }));
    const user = userEvent.setup();
    render(<NodeEmulatorTool currentTarget={{ type: "screen", index: 0 }} />);

    await user.click(screen.getByRole("button", { name: "Recognize Only" }));
    await waitFor(() => expect(api.testNode).toHaveBeenCalled());
    act(() => useOverlayStore.getState().clearRecognition());
    expect(useOverlayStore.getState().roi).toBeNull();

    act(() => finish({
      ...result,
      capture_roi: { x: 20, y: 30, width: 200, height: 100 },
    }));

    await waitFor(() => expect(useOverlayStore.getState().roi).toEqual({ x: 20, y: 30, w: 200, h: 100 }));
  });
});
