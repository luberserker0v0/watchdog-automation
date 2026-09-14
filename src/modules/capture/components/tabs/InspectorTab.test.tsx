import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const screenApi = vi.hoisted(() => ({ saveRegionCapture: vi.fn() }));
vi.mock("../../../../api/screen", () => screenApi);
vi.mock("../../../../api/window", () => ({ getWindowDetails: vi.fn() }));

import { useCaptureFrameStore } from "../../stores/useCaptureFrameStore";
import { useStreamStore } from "../../stores/useStreamStore";
import InspectorTab from "./InspectorTab";

describe("InspectorTab region capture", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useStreamStore.setState({ isStreaming: false });
    useCaptureFrameStore.setState({
      frames: {
        screen: { previewUrl: null, captureBuf: null, objectUrl: false },
        window: { previewUrl: "/capture", captureBuf: [1, 2, 3], objectUrl: false },
      },
    });
  });

  afterEach(cleanup);

  it("saves the selected natural-image region from a still capture", async () => {
    screenApi.saveRegionCapture.mockResolvedValue("C:\\Users\\tester\\.watch_dog\\visions\\region.png");
    render(<InspectorTab captureMode="window" selection={{ x: 10, y: 20, w: 30, h: 40 }} />);

    fireEvent.click(screen.getByRole("button", { name: "區域截圖" }));

    await waitFor(() => expect(screen.getByText(/已儲存/)).toBeInTheDocument());
    expect(screenApi.saveRegionCapture).toHaveBeenCalledWith(
      [1, 2, 3],
      { x: 10, y: 20, w: 30, h: 40 },
      expect.stringMatching(/^region_.*\.png$/),
    );
  });

  it("uses the backend frame cache while streaming", async () => {
    useStreamStore.setState({ isStreaming: true });
    screenApi.saveRegionCapture.mockResolvedValue("region.png");
    render(<InspectorTab captureMode="window" selection={{ x: 1, y: 2, w: 3, h: 4 }} />);

    fireEvent.click(screen.getByRole("button", { name: "區域截圖" }));

    await waitFor(() => expect(screenApi.saveRegionCapture).toHaveBeenCalled());
    expect(screenApi.saveRegionCapture.mock.calls[0][0]).toBeNull();
  });
});
