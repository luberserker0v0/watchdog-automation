import { describe, expect, it } from "vitest";
import { readRecognitionDevice } from "./SplashScreen";

describe("SplashScreen startup preferences", () => {
  it("preloads the persisted OCR device and safely falls back to CPU", () => {
    expect(readRecognitionDevice({ getItem: () => JSON.stringify({ state: { recognitionDevice: "directml" } }) })).toBe("directml");
    expect(readRecognitionDevice({ getItem: () => "not-json" })).toBe("cpu");
    expect(readRecognitionDevice({ getItem: () => JSON.stringify({ state: { recognitionDevice: "unknown" } }) })).toBe("cpu");
  });
});
