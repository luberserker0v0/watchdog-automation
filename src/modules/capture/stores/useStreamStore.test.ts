import { beforeEach, describe, expect, it } from "vitest";
import { useCaptureFrameStore } from "./useCaptureFrameStore";
import { useStreamStore } from "./useStreamStore";

describe("stream lifecycle", () => {
  beforeEach(() => {
    useStreamStore.setState({
      isStreaming: true,
      actualFps: 5,
      startTime: Date.now() - 1_000,
      elapsedTime: "00:01",
    });
    useCaptureFrameStore.getState().setStreamFrameUrl("window", "/frame?t=1");
  });

  it("clears streaming state and the stale frame when the backend target closes", () => {
    useStreamStore.getState().handleBackendStopped("window");

    expect(useStreamStore.getState()).toMatchObject({
      isStreaming: false,
      actualFps: 0,
      startTime: null,
      elapsedTime: "00:00",
    });
    expect(useCaptureFrameStore.getState().frames.window.previewUrl).toBeNull();
  });
});
