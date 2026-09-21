import { fireEvent, render } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import MermaidGraph from "./MermaidGraph";
import { useGraphLayoutStore } from "../stores/useGraphLayoutStore";

const { savePipelineLayoutMock } = vi.hoisted(() => ({
  savePipelineLayoutMock: vi.fn().mockResolvedValue(true),
}));

vi.mock("../../../../api/pipeline", async (importOriginal) => ({
  ...await importOriginal<typeof import("../../../../api/pipeline")>(),
  savePipelineLayout: savePipelineLayoutMock,
}));

const originalResizeObserver = globalThis.ResizeObserver;

beforeEach(() => {
  savePipelineLayoutMock.mockClear();
  useGraphLayoutStore.setState({ layouts: {}, minimapVisible: true });
  globalThis.ResizeObserver = class {
    observe() {}
    unobserve() {}
    disconnect() {}
  };
});

it("persists the user's minimap visibility from one button", () => {
  const view = render(
    <MermaidGraph
      mermaid={'A["Account"]'}
      currentNodeId={null}
      selectedNodeId={null}
      onNodeClick={vi.fn()}
      layoutKey="sample/sample.md"
      allowNodeLayout
    />,
  );
  expect(view.container.querySelector(".testbed-minimap")).toBeInTheDocument();
  fireEvent.click(view.getByRole("button", { name: "Hide minimap" }));
  expect(view.container.querySelector(".testbed-minimap")).toBeNull();
  expect(view.getByRole("button", { name: "Show minimap" })).toHaveAttribute("aria-pressed", "false");
  expect(useGraphLayoutStore.getState().minimapVisible).toBe(false);
});

it("keeps Entry and Endpoint visually distinct while selected or running", () => {
  const flow = {
    version: "1",
    nodes: [
      { id: "Start", kind: "Entry" as const, recognition: { type: "DirectHit" as const } },
      { id: "Done", kind: "Endpoint" as const, recognition: { type: "DirectHit" as const } },
    ],
    edges: [{ from: "Start", to: "Done", edge_type: "next" as const }],
  };
  const view = render(<MermaidGraph mermaid="" flow={flow} currentNodeId="Start" selectedNodeId="Done" onNodeClick={vi.fn()} />);
  const entry = view.getByText("Start").closest("g")!;
  const endpoint = view.getByText("Done").closest("g")!;
  expect(entry.querySelector('rect[fill="#047857"]')).toBeInTheDocument();
  expect(endpoint.querySelector('rect[fill="#be123c"]')).toBeInTheDocument();
  expect(entry.querySelector('path[aria-label="Entry"]')).toBeInTheDocument();
  expect(endpoint.querySelector('rect[aria-label="Endpoint"]')).toBeInTheDocument();
  expect(entry.querySelector('rect[stroke="#2563eb"]')).toBeInTheDocument();
  expect(endpoint.querySelector('rect[stroke="#0284c7"]')).toBeInTheDocument();
});

it("commits dragged positions to the shareable layout file and can restore automatic layout", () => {
  const requestFrame = vi.spyOn(window, "requestAnimationFrame").mockImplementation((callback) => {
    callback(0);
    return 1;
  });
  const view = render(
    <MermaidGraph
      mermaid={'A["Account"]\nB["Password"]\nA --> B'}
      currentNodeId={null}
      selectedNodeId={null}
      onNodeClick={vi.fn()}
      layoutKey="sample/sample.md"
      allowNodeLayout
    />,
  );
  fireEvent.click(view.getByRole("checkbox", { name: "Edit layout" }));
  const node = view.getByText("Account").closest("g")!;
  const graph = view.container.querySelector(".testbed-graph-flow")!;
  fireEvent.mouseDown(node, { button: 0, clientX: 100, clientY: 100 });
  fireEvent.mouseMove(graph, { clientX: 150, clientY: 200 });
  fireEvent.mouseUp(graph);

  expect(useGraphLayoutStore.getState().layouts["sample/sample.md"].A).toEqual({ x: 150, y: 140 });
  expect(node.querySelector("rect")).toHaveAttribute("x", "150");
  expect(node.querySelector("rect")).toHaveAttribute("y", "140");
  expect(savePipelineLayoutMock).toHaveBeenLastCalledWith("sample/sample.md", {
    A: { x: 150, y: 140 },
  });

  fireEvent.click(view.getByRole("button", { name: "Auto align" }));
  expect(useGraphLayoutStore.getState().layouts["sample/sample.md"]).toBeUndefined();
  expect(node.querySelector("rect")).toHaveAttribute("x", "100");
  expect(node.querySelector("rect")).toHaveAttribute("y", "40");
  expect(savePipelineLayoutMock).toHaveBeenLastCalledWith("sample/sample.md", {});
  requestFrame.mockRestore();
});

it("selects a rectangular region and moves all selected nodes together", () => {
  const requestFrame = vi.spyOn(window, "requestAnimationFrame").mockImplementation((callback) => {
    callback(0);
    return 1;
  });
  const view = render(
    <MermaidGraph
      mermaid={'A["Account"]\nB["Password"]\nA --> B'}
      currentNodeId={null}
      selectedNodeId={null}
      onNodeClick={vi.fn()}
      layoutKey="sample/sample.md"
      allowNodeLayout
    />,
  );
  fireEvent.click(view.getByRole("checkbox", { name: "Region select" }));
  const graph = view.container.querySelector(".testbed-graph-flow")!;
  fireEvent.mouseDown(graph, { button: 0, clientX: 90, clientY: 30 });
  fireEvent.mouseMove(graph, { clientX: 230, clientY: 190 });
  expect(view.container.querySelector(".testbed-region-selection")).toBeInTheDocument();
  fireEvent.mouseUp(graph);
  expect(view.container.querySelectorAll(".layout-selected")).toHaveLength(2);

  const account = view.getByText("Account").closest("g")!;
  fireEvent.mouseDown(account, { button: 0, clientX: 100, clientY: 100 });
  fireEvent.mouseMove(graph, { clientX: 150, clientY: 150 });
  fireEvent.mouseUp(graph);

  expect(useGraphLayoutStore.getState().layouts["sample/sample.md"]).toEqual({
    A: { x: 150, y: 90 },
    B: { x: 150, y: 190 },
  });
  expect(savePipelineLayoutMock).toHaveBeenLastCalledWith("sample/sample.md", {
    A: { x: 150, y: 90 },
    B: { x: 150, y: 190 },
  });
  requestFrame.mockRestore();
});

it("shows an SVG grid without changing node positions", () => {
  const view = render(
    <MermaidGraph
      mermaid={'A["Account"]'}
      currentNodeId={null}
      selectedNodeId={null}
      onNodeClick={vi.fn()}
      layoutKey="sample/sample.md"
      allowNodeLayout
    />,
  );
  expect(view.container.querySelector('rect[fill="url(#graph-grid)"]')).toBeNull();
  fireEvent.click(view.getByRole("checkbox", { name: "Grid" }));
  expect(view.container.querySelector('rect[fill="url(#graph-grid)"]')).toBeInTheDocument();
  expect(view.getByText("Account").closest("g")?.querySelector("rect")).toHaveAttribute("x", "100");
});

afterEach(() => {
  globalThis.ResizeObserver = originalResizeObserver;
});

it("centers the current node when focus mode is enabled", () => {
  const props = {
    mermaid: 'A["Account"]\nB["Password"]\nA --> B',
    currentNodeId: "B",
    selectedNodeId: null,
    onNodeClick: vi.fn(),
  };
  const view = render(<MermaidGraph {...props} focusCurrentNode={false} />);
  const graphSvg = view.container.querySelector(".testbed-graph-flow svg")!;
  expect(graphSvg).toHaveAttribute("viewBox", "0 0 600 400");

  view.rerender(<MermaidGraph {...props} focusCurrentNode />);
  expect(graphSvg).toHaveAttribute("viewBox", "-140 -42 600 400");
});

it("follows a newly reported current node", () => {
  const props = {
    mermaid: 'A["Account"]\nB["Password"]\nA --> B',
    selectedNodeId: null,
    onNodeClick: vi.fn(),
    focusCurrentNode: true,
  };
  const view = render(<MermaidGraph {...props} currentNodeId="A" />);
  const graphSvg = view.container.querySelector(".testbed-graph-flow svg")!;
  expect(graphSvg).toHaveAttribute("viewBox", "-140 -142 600 400");

  view.rerender(<MermaidGraph {...props} currentNodeId="B" />);
  expect(graphSvg).toHaveAttribute("viewBox", "-140 -42 600 400");
});
