import { describe, expect, it } from "vitest";
import { computeBBox, layoutGraph, parseMermaidGraph } from "./mermaidGraphModel";

describe("mermaid graph model", () => {
  it("parses, lays out, and bounds graph data without React state", () => {
    const graph = parseMermaidGraph('start["Start"]\nend["End"]\nstart --> end');
    const nodes = layoutGraph(graph.nodes, graph.edges);
    expect(nodes.map((node) => node.id)).toEqual(["start", "end"]);
    expect(nodes[1].y).toBeGreaterThan(nodes[0].y);
    expect(computeBBox(nodes).height).toBeGreaterThan(36);
  });
});

it("automatically aligns sibling branches without overlap", () => {
  const nodes = [
    { id: "start", label: "Start", x: 0, y: 0 },
    { id: "next", label: "Next", x: 0, y: 0 },
    { id: "error", label: "Error", x: 0, y: 0 },
  ];
  const arranged = layoutGraph(nodes, [
    { from: "start", to: "next", isError: false },
    { from: "start", to: "error", isError: true },
  ]);
  const next = arranged.find((node) => node.id === "next")!;
  const error = arranged.find((node) => node.id === "error")!;
  expect(next.y).toBe(error.y);
  expect(Math.abs(next.x - error.x)).toBeGreaterThanOrEqual(120);
});
