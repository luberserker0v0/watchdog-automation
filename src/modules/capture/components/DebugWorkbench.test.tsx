import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useDebugStore } from "../stores/useDebugStore";

vi.mock("./tabs/InspectorTab", () => ({ default: () => <div>coordinate tool body</div> }));
vi.mock("./tabs/ActionTesterTab", () => ({ default: () => <div>action tool body</div> }));
vi.mock("./tabs/RecognitionTab", () => ({ default: () => <div>recognition tool body</div> }));
vi.mock("./tools/NodeEmulatorTool", () => ({ default: () => <div>emulator tool body</div> }));

import DebugWorkbench, { DEBUG_TOOLS } from "./DebugWorkbench";

const context = {
  captureMode: "screen" as const,
  isStreaming: false,
  captureBuf: [1, 2, 3],
  hasFrame: true,
  currentTarget: { type: "screen" as const, index: 0 },
  actionContext: { type: "screen" as const, index: 0, frame_width: 100, frame_height: 50 },
};

describe("DebugWorkbench", () => {
  beforeEach(() => useDebugStore.setState({ activeToolId: "inspector" }));

  it("registers all tools and mounts only the active tool", () => {
    expect(DEBUG_TOOLS.map((tool) => tool.id)).toEqual(["inspector", "action", "recognition", "node-emulator"]);
    render(<DebugWorkbench context={context} captureControl={{ value: "screen", readOnly: true }} />);
    expect(screen.getByText("coordinate tool body")).toBeInTheDocument();
    expect(screen.queryByText("action tool body")).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Node Emulator" }));
    expect(screen.getByText("emulator tool body")).toBeInTheDocument();
    expect(screen.queryByText("coordinate tool body")).not.toBeInTheDocument();
  });

  it("supports compact select navigation and reports unmet requirements", () => {
    render(<DebugWorkbench context={{ ...context, hasFrame: false }} captureControl={{ value: "screen", label: "Screen", readOnly: true }} />);
    fireEvent.change(screen.getByLabelText("Debug tool"), { target: { value: "recognition" } });
    expect(screen.getByText("Capture or stream a frame to use this tool.")).toBeInTheDocument();
    expect(screen.getByText("Screen")).toBeInTheDocument();
  });
});
