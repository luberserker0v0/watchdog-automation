import { beforeEach, describe, expect, it, vi } from "vitest";
import { usePipelineStore } from "./usePipelineStore";

vi.mock("../../../api/pipeline", () => ({
  loadPipeline: vi.fn(async (path: string) => ({
    graph: {
      version: "1",
      nodes: [{
        id: "start",
        recognition: { type: "DirectHit" },
        params: [{ type: "text", key: "name", label: "Name", default: path }],
      }],
      edges: [],
    },
    mermaid: "start",
    meta: "",
  })),
  startPipeline: vi.fn(async () => "run-1"),
  stopPipeline: vi.fn(async () => undefined),
  pausePipeline: vi.fn(async () => undefined),
  getPipelineStatus: vi.fn(async () => ({
    state: "Completed",
    current_node_id: null,
    elapsed_ms: 300,
    step_results: [],
  })),
}));

beforeEach(() => {
  usePipelineStore.setState({
    pipelineIds: [],
    pipelinesById: {},
    selectedId: null,
    activeId: null,
    isExecuting: false,
  });
});

describe("usePipelineStore", () => {
  it("normalizes added pipelines and preserves sibling identity", async () => {
    await usePipelineStore.getState().addPipeline("one.yml");
    await usePipelineStore.getState().addPipeline("two.yml");
    const before = usePipelineStore.getState();
    const [firstId, secondId] = before.pipelineIds;
    const second = before.pipelinesById[secondId];

    before.updateParams(firstId, "name", "changed");
    const after = usePipelineStore.getState();

    expect(after.pipelinesById[firstId].userParams.name).toBe("changed");
    expect(after.pipelinesById[secondId]).toBe(second);
  });

  it("updates selection and removes the selected entity consistently", async () => {
    await usePipelineStore.getState().addPipeline("one.yml");
    await usePipelineStore.getState().addPipeline("two.yml");
    const [firstId, secondId] = usePipelineStore.getState().pipelineIds;
    usePipelineStore.getState().selectPipeline(secondId);
    usePipelineStore.getState().removePipeline(secondId);
    expect(usePipelineStore.getState().pipelineIds).toEqual([firstId]);
    expect(usePipelineStore.getState().selectedId).toBe(firstId);
    expect(usePipelineStore.getState().pipelinesById[secondId]).toBeUndefined();
  });

  it("finishes a sequential run with a single non-overlapping polling timer", async () => {
    vi.useFakeTimers();
    await usePipelineStore.getState().addPipeline("one.yml");
    const id = usePipelineStore.getState().pipelineIds[0];
    const execution = usePipelineStore.getState().startExecution();
    await Promise.resolve();
    await vi.advanceTimersByTimeAsync(300);
    await execution;
    expect(usePipelineStore.getState().isExecuting).toBe(false);
    expect(usePipelineStore.getState().pipelinesById[id].status).toBe("completed");
    vi.useRealTimers();
  });
});
