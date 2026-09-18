import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useDebugStore } from "../../stores/useDebugStore";
import { useOverlayStore } from "../../stores/useOverlayStore";

const api = vi.hoisted(() => ({ testRecognition: vi.fn() }));
vi.mock("../../../../api/pipeline", async (importOriginal) => ({
  ...await importOriginal<typeof import("../../../../api/pipeline")>(),
  testRecognition: api.testRecognition,
}));

import RecognitionTab from "./RecognitionTab";

describe("RecognitionTab overlay ownership", () => {
  beforeEach(() => {
    api.testRecognition.mockResolvedValue({
      matched: true,
      confidence: 0.9,
      box_region: { x: 40, y: 50, width: 60, height: 20 },
      text: "match",
      candidates: [],
    });
    useDebugStore.setState({
      recogType: "OCR",
      recogFields: { roi: "10,20,100,80", expected: "match" },
    });
    useOverlayStore.setState({ roi: null, resultRoi: null });
  });

  it("does not draw a persisted draft until recognition is explicitly run", () => {
    render(<RecognitionTab isStreaming captureMode="window" />);
    expect(useOverlayStore.getState().roi).toBeNull();
    expect(useOverlayStore.getState().resultRoi).toBeNull();
  });

  it("publishes ROI and result for the current run, then clears them on exit", async () => {
    const user = userEvent.setup();
    const view = render(<RecognitionTab isStreaming captureMode="window" />);

    await user.click(screen.getByRole("button", { name: "執行辨識" }));

    expect(useOverlayStore.getState().roi).toEqual({ x: 10, y: 20, w: 100, h: 80 });
    expect(useOverlayStore.getState().resultRoi).toEqual({ x: 40, y: 50, w: 60, h: 20 });

    view.unmount();
    expect(useOverlayStore.getState().roi).toBeNull();
    expect(useOverlayStore.getState().resultRoi).toBeNull();
  });
});
