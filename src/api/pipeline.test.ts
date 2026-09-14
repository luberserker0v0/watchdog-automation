import { describe, expect, it } from "vitest";
import { normalizePipelineStartError } from "./pipeline";

describe("pipeline start errors", () => {
  it("preserves a structured window-size mismatch", () => {
    const mismatch = {
      code: "window_size_mismatch" as const,
      hwnd: 42,
      title: "Target",
      expected: [1280, 720] as [number, number],
      before: [900, 600] as [number, number],
      actual: [900, 600] as [number, number],
      resize_attempted: true,
      diagnostic: "rejected",
    };
    expect(normalizePipelineStartError(mismatch)).toEqual(mismatch);
    expect(normalizePipelineStartError(JSON.stringify(mismatch))).toEqual(mismatch);
  });

  it("normalizes legacy string failures", () => {
    expect(normalizePipelineStartError("Invalid pipeline")).toEqual({
      code: "generic",
      message: "Invalid pipeline",
    });
  });
});
