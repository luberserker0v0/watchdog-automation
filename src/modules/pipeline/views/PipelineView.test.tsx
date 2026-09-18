import { render } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import PipelineView from "./PipelineView";
import { type PipelineEntry, usePipelineStore } from "../stores/usePipelineStore";

vi.mock("../../../api/pipeline", async (importOriginal) => {
  const original = await importOriginal<typeof import("../../../api/pipeline")>();
  return { ...original, listPipelines: vi.fn().mockResolvedValue([]) };
});

vi.mock("../components/MermaidGraph", () => ({
  default: () => <div data-testid="pipeline-graph">Graph content</div>,
}));

const pipeline: PipelineEntry = {
  id: "sample",
  path: "sample/sample.md",
  name: "Sample pipeline",
  graph: null,
  mermaid: "flowchart TD\nA --> B",
  meta: "",
  paramsByNode: {},
  userParams: {},
  isChecked: true,
  status: "idle",
  runId: null,
  currentNodeId: null,
  elapsedMs: 0,
};

beforeEach(() => {
  usePipelineStore.setState({
    pipelineIds: [pipeline.id],
    pipelinesById: { [pipeline.id]: pipeline },
    selectedId: pipeline.id,
    activeId: null,
    isExecuting: false,
    batchIds: [],
    resizeIssue: null,
  });
});

describe("PipelineView workspace", () => {
  it("lets the user hide the graph for more editing space and restore it", async () => {
    const user = userEvent.setup();
    const view = render(<PipelineView />);

    expect(view.getByRole("region", { name: "Pipeline graph" })).toBeVisible();
    await user.click(view.getByRole("button", { name: "Hide graph" }));
    expect(view.queryByRole("region", { name: "Pipeline graph" })).toBeNull();
    expect(view.getByRole("button", { name: "Show graph" })).toBeVisible();

    await user.click(view.getByRole("button", { name: "Show graph" }));
    expect(view.getByRole("region", { name: "Pipeline graph" })).toBeVisible();
  });

  it("provides a graph focus-mode switch", async () => {
    const user = userEvent.setup();
    const view = render(<PipelineView />);
    const focus = view.getByRole("switch", { name: "Focus current" });

    expect(focus).not.toBeChecked();
    await user.click(focus);
    expect(focus).toBeChecked();
  });
});
