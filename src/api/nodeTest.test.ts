import { describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn().mockResolvedValue({}));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

import { testNode, type NodeMetaInfo } from "./nodeTest";

describe("testNode API", () => {
  it("passes the optional execution mode without changing the command name", async () => {
    const node: NodeMetaInfo = { id: "Example", file_name: "NodeEmulator.meta.json", source: "custom", recognition: { type: "DirectHit" }, actions: [], params: [], enabled: true };
    await testNode(node, true, {}, true, "recognize_only");
    expect(invoke).toHaveBeenCalledWith("test_node", { node, saveDebugImages: true, userParams: {}, detailedTimings: true, executionMode: "recognize_only" });
  });
});
