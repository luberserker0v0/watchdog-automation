import { describe, expect, it } from "vitest";
import type { RecordedActionDraft } from "../../../api/recorder";
import { recordedDelayBefore, toPipelineActions } from "./recordingTiming";

const actions: RecordedActionDraft[] = [
  { id: 1, offset_ms: 100, source: "mouse", action: { type: "Click", target: [10, 20] } },
  { id: 2, offset_ms: 900, source: "mouse", action: { type: "LongPress", target: [30, 40], duration: 300 } },
  { id: 3, offset_ms: 1_400, source: "keyboard", action: { type: "ClickKey", key: [13] } },
];

describe("recording timing", () => {
  it("derives idle gaps without counting gesture duration twice", () => {
    expect(recordedDelayBefore(actions, 0)).toBe(0);
    expect(recordedDelayBefore(actions, 1)).toBe(500);
    expect(recordedDelayBefore(actions, 2)).toBe(500);
  });

  it("exports visible Wait actions between recorded actions", () => {
    expect(toPipelineActions(actions)).toEqual([
      actions[0].action,
      { type: "Wait", duration: 500 },
      actions[1].action,
      { type: "Wait", duration: 500 },
      actions[2].action,
    ]);
  });
});
