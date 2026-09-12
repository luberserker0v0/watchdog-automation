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
