import { describe, expect, it } from "vitest";
import { resolveRecognitionOverlayRoi } from "./recognitionOverlay";

describe("recognition overlay geometry", () => {
  it("resolves full-frame ROI and applies offsets like the backend", () => {
    expect(resolveRecognitionOverlayRoi({
      type: "DirectHit",
      roi: [0, 0, 0, 0],
      roi_offset: [10, 20, -30, -40],
    }, { width: 500, height: 400 })).toEqual({ x: 10, y: 20, w: 470, h: 360 });
  });

  it("clips an offset ROI to the captured frame", () => {
    expect(resolveRecognitionOverlayRoi({
      type: "OCR",
      roi: [-20, -10, 100, 80],
      roi_offset: [5, 2, 0, 0],
    }, { width: 60, height: 50 })).toEqual({ x: 0, y: 0, w: 60, h: 50 });
  });

  it("does not invent full-frame geometry before image dimensions are known", () => {
    expect(resolveRecognitionOverlayRoi({ type: "DirectHit" }, null)).toBeNull();
  });
});
