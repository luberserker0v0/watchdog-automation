import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { usePreferencesStore } from "../../core/stores/usePreferencesStore";
import { useCaptureFrameStore } from "../stores/useCaptureFrameStore";
import { useOverlayStore } from "../stores/useOverlayStore";
import { useStreamStore } from "../stores/useStreamStore";
import { useWindowStore } from "../stores/useWindowStore";
import { usePipelineStore } from "../../pipeline/stores/usePipelineStore";
import TargetInfo from "./TargetInfo";

describe("TargetInfo capture dimensions", () => {
  beforeEach(() => {
    usePreferencesStore.setState({ mode: "window", windowCaptureArea: "client" });
    useStreamStore.setState({ isStreaming: false, frameSize: null, clientSize: null });
    useOverlayStore.getState().setImageDimensions({ width: 1280, height: 720 });
    useCaptureFrameStore.getState().setStreamFrameUrl("window", "/frame");
    useWindowStore.setState({
      selectedIndex: 0,
      windows: [{ index: 0, title: "Target", app_name: "Target", width: 1296, height: 759, hwnd: 42 }],
    });
    usePipelineStore.setState({ activeId: null, resizeIssue: null });
  });

  afterEach(() => {
    cleanup();
    useCaptureFrameStore.getState().clearFrame("window");
    useOverlayStore.getState().setImageDimensions(null);
  });

  it("shows a client screenshot as both content and encoded capture size", () => {
    render(<TargetInfo />);

    expect(screen.getByText("Content 1280×720")).toBeInTheDocument();
    expect(screen.getByText("Capture 1280×720")).toBeInTheDocument();
  });

  it("uses live backend dimensions while streaming", () => {
    useStreamStore.setState({
      isStreaming: true,
      frameSize: [1296, 759],
      clientSize: [1280, 720],
    });

    render(<TargetInfo
      targetName="Target"
      mode="window"
      engine="graphics"
      statusClass="streaming"
      statusLabel="Streaming"
      expectedSize={[1280, 720]}
    />);

    expect(screen.getByText("Content 1280×720")).toBeInTheDocument();
    expect(screen.getByText("Capture 1296×759")).toBeInTheDocument();
    expect(screen.getByText("Expected 1280×720 ✓")).toBeInTheDocument();
  });

  it("does not show stale image dimensions after the frame is cleared", () => {
    useCaptureFrameStore.getState().clearFrame("window");
    render(<TargetInfo />);

    expect(screen.queryByText(/Capture 1280×720/)).not.toBeInTheDocument();
    expect(screen.queryByText(/Content 1280×720/)).not.toBeInTheDocument();
  });

});
