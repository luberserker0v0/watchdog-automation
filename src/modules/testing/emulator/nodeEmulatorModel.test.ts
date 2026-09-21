import { describe, expect, it } from "vitest";
import { escapeExactRegex } from "../../capture/target";
import {
  buildNodeEmulator,
  createNodeEmulatorDraft,
  defaultActionDraft,
  defaultParamDraft,
  duplicateItem,
  moveItem,
} from "./nodeEmulatorModel";

describe("node emulator metadata model", () => {
  it("creates compatible defaults and an exact current-window preset", () => {
    const target = { type: "window" as const, title: escapeExactRegex("Target UI [Login]"), capture_area: "client" as const };
    const draft = createNodeEmulatorDraft(target);
    const result = buildNodeEmulator(draft);

    expect(draft.target).toEqual({ type: "window", title: "^Target UI \\[Login\\]$", capture_area: "client" });
    expect(draft.recognition.type).toBe("DirectHit");
    expect(draft.actions[0].type).toBe("DoNothing");
    expect(draft.timeout).toBe("20000");
    expect(result.errors).toEqual({});
    expect(result.value?.metadata.target).toEqual(target);
    expect(result.value?.metadata.nodes.EmulatedNode).toMatchObject({
      enabled: true,
      inverse: false,
      recognition: { type: "DirectHit", roi: [0, 0, 0, 0], candidate_selection: { order: "native", index: 0 } },
      repeat: 1,
    });
  });

  it("accepts a filterless window target that inherits the active window", () => {
    const draft = createNodeEmulatorDraft({ type: "window", capture_area: "client" });
    const result = buildNodeEmulator(draft);
    expect(result.errors).toEqual({});
    expect(result.value?.metadata.target).toEqual({
      type: "window",
      capture_area: "client",
    });
  });

  it("serializes a prior-node ROI reference and rejects invalid IDs", () => {
    const draft = createNodeEmulatorDraft();
    draft.recognition.roi = "FindPanel";
    expect(buildNodeEmulator(draft).value?.metadata.nodes.EmulatedNode.recognition.roi).toBe("FindPanel");
    draft.recognition.roi = "Find-Panel";
    expect(buildNodeEmulator(draft).value?.metadata.nodes.EmulatedNode.recognition.roi).toBe("Find-Panel");
    draft.recognition.roi = "bad id";
    expect(buildNodeEmulator(draft).errors["recognition.roi"]).toMatch(/valid earlier node ID/);
  });

  it("serializes window_size on the metadata target rather than the node", () => {
    const draft = createNodeEmulatorDraft({
      type: "window",
      title: "^Target$",
      capture_area: "frame",
      window_size: [1280, 720],
    });
    const result = buildNodeEmulator(draft);
    expect(result.errors).toEqual({});
    expect(result.value?.metadata.target).toMatchObject({ window_size: [1280, 720] });
    expect(result.value?.metadata.nodes.EmulatedNode.target).toBeUndefined();
    expect(result.value?.node.target).toMatchObject({ window_size: [1280, 720] });
  });

  it("rejects non-positive window dimensions", () => {
    const draft = createNodeEmulatorDraft({
      type: "window",
      capture_area: "client",
      window_size: [0, 720],
    });
    expect(buildNodeEmulator(draft).errors["target.window_size"]).toMatch(/positive integers/);
  });

  it("serializes every action, parameter, timing, offset, and attach field", () => {
    const action = (type: ReturnType<typeof defaultActionDraft>["type"], patch = {}) => ({ ...defaultActionDraft(), type, ...patch });
    const parameter = (type: ReturnType<typeof defaultParamDraft>["type"], key: string, defaultValue: string) => ({ ...defaultParamDraft(), type, key, label: key, defaultValue, options: "one,two" });
    const draft = createNodeEmulatorDraft({ type: "screen", index: 2 });
    draft.id = "AllFields";
    draft.inverse = true;
    draft.recognition = { ...draft.recognition, type: "OCR", expected: "^Card 1$", threshold: "0.75", roi: "1,2,30,40", roiOffset: "5,6,7,8", candidateOrder: "right_to_left", candidateIndex: "2" };
    draft.actions = [
      action("DoNothing"),
      action("Click", { targetKind: "point", target: "10,20", targetOffset: "1,2,3,4" }),
      action("LongPress", { targetKind: "node", target: "Other", duration: "900" }),
      action("Swipe", { targetKind: "region", target: "1,2,3,4", endKind: "point", end: "50,60", duration: "300" }),
      action("Scroll", { dx: "-2", dy: "4" }),
      action("ClickKey", { keys: "17,67" }),
      action("InputText", { inputText: "${account}" }),
      action("StopTask"),
      action("Custom", { customName: "plugin_action", customParam: "{\"flag\":true}" }),
    ];
    draft.params = [parameter("text", "text", "abc"), parameter("password", "password", "secret"), parameter("number", "number", "3.5"), parameter("dropdown", "choice", "one"), parameter("switch", "enabled", "true"), parameter("color", "color", "#ffffff")];
    Object.assign(draft, { kind: "Normal", desc: "Runs the flow", timeout: "11", errorTimeout: "12", rateLimit: "13", preDelay: "14", preWaitFreezes: "250", postDelay: "15", postWaitFreezes: "500", maxHit: "16", repeat: "2", repeatDelay: "17", attach: "{\"owner\":\"tester\"}" });

    const result = buildNodeEmulator(draft);
    expect(result.errors).toEqual({});
    const node = result.value!.metadata.nodes.AllFields;
    expect(node.actions?.map((item) => item.type)).toEqual(["DoNothing", "Click", "LongPress", "Swipe", "Scroll", "ClickKey", "InputText", "StopTask", "Custom"]);
    expect(node.params?.map((item) => item.type)).toEqual(["text", "password", "number", "dropdown", "switch", "color"]);
    expect(node).toMatchObject({ kind: "Normal", desc: "Runs the flow", timeout: 11, error_timeout: 12, rate_limit: 13, pre_delay: 14, pre_wait_freezes: 250, post_delay: 15, post_wait_freezes: 500, max_hit: 16, repeat: 2, repeat_delay: 17, attach: { owner: "tester" } });
    expect(result.value!.paramValues).toMatchObject({ number: 3.5, enabled: true, choice: "one" });
  });

  it("serializes Endpoint without recognition settings or actions", () => {
    const draft = createNodeEmulatorDraft();
    draft.kind = "Endpoint";
    draft.recognition = { ...draft.recognition, type: "OCR", expected: "ignored" };
    const result = buildNodeEmulator(draft);
    expect(result.errors).toEqual({});
    expect(result.value!.node).toMatchObject({ kind: "Endpoint", recognition: { type: "DirectHit" }, actions: [] });
  });

  it.each([
    ["DirectHit", {}],
    ["TemplateMatch", { template: "Common/Button.png", threshold: "0.8" }],
    ["OCR", { expected: "^Login$", threshold: "0.6" }],
    ["ColorMatch", { lower: "1,2,3", upper: "250,251,252" }],
    ["Custom", { customName: "contributor_recognizer", customParam: "{\"mode\":1}" }],
  ] as const)("serializes %s recognition", (type, patch) => {
    const draft = createNodeEmulatorDraft();
    draft.recognition = { ...draft.recognition, type, ...patch };
    const result = buildNodeEmulator(draft);
    expect(result.errors).toEqual({});
    expect(result.value?.metadata.nodes.EmulatedNode.recognition.type).toBe(type);
  });

  it("reports invalid regex, JSON, numeric ranges, duplicates, and incomplete fields", () => {
    const draft = createNodeEmulatorDraft({ type: "window", title: "[", capture_area: "frame" });
    draft.id = "bad id";
    draft.recognition = { ...draft.recognition, type: "TemplateMatch", template: "", threshold: "2", lower: "-1,0,999", customParam: "{" };
    draft.actions = [{ ...defaultActionDraft(), type: "ClickKey", keys: "" }, { ...defaultActionDraft(), type: "Custom", customName: "", customParam: "{" }];
    draft.params = [{ ...defaultParamDraft(), key: "same", label: "A" }, { ...defaultParamDraft(), key: "same", label: "B" }];
    draft.attach = "{";
    const { errors, value } = buildNodeEmulator(draft);
    expect(value).toBeUndefined();
    expect(errors).toMatchObject({ id: expect.any(String), "target.title": expect.any(String), "recognition.template": expect.any(String), "recognition.threshold": expect.any(String), "actions.0.key": expect.any(String), "actions.1.action": expect.any(String), "actions.1.param": expect.any(String), "params.1.key": expect.any(String), attach: expect.any(String) });
  });

  it("validates OCR regular expressions and RGB channel ranges", () => {
    const ocr = createNodeEmulatorDraft();
    ocr.recognition = { ...ocr.recognition, type: "OCR", expected: "[" };
    expect(buildNodeEmulator(ocr).errors).toHaveProperty("recognition.expected");
    const color = createNodeEmulatorDraft();
    color.recognition = { ...color.recognition, type: "ColorMatch", lower: "-1,0,0", upper: "255,256,0" };
    expect(buildNodeEmulator(color).errors).toMatchObject({ "recognition.lower": expect.any(String), "recognition.upper": expect.any(String) });
  });

  it("moves and duplicates repeatable rows without mutating the source", () => {
    const source = [{ id: 1 }, { id: 2 }, { id: 3 }];
    expect(moveItem(source, 2, 0).map((item) => item.id)).toEqual([3, 1, 2]);
    const duplicated = duplicateItem(source, 1);
    expect(duplicated.map((item) => item.id)).toEqual([1, 2, 2, 3]);
    expect(duplicated[2]).not.toBe(source[1]);
    expect(source.map((item) => item.id)).toEqual([1, 2, 3]);
  });
});
