export interface GraphNode { id: string; label: string; x: number; y: number }
export interface GraphEdge { from: string; to: string; isError: boolean }
export interface BBox { minX: number; minY: number; maxX: number; maxY: number; width: number; height: number }

export const NODE_W = 120;
export const NODE_H = 36;
const NODE_SPACING = 100;
export const ZOOM_LEVELS = [0.25, 0.33, 0.5, 0.67, 0.75, 0.8, 0.9, 1, 1.1, 1.25, 1.5, 1.75, 2, 3];

export function parseMermaidGraph(mermaid: string): { nodes: GraphNode[]; edges: GraphEdge[] } {
  const nodes: GraphNode[] = [];
  const edges: GraphEdge[] = [];
  for (const source of mermaid.split("\n")) {
    const line = source.trim();
    if (!line || line.startsWith("%%")) continue;
    const node = line.match(/^(\w+)\[(?:"([^"]+)"|'([^']+)')\]|^(\w+)\((?:"([^"]+)"|'([^']+)')\)|^(\w+)$/);
    if (node) {
      const id = node[1] || node[4] || node[7];
      nodes.push({ id, label: node[2] || node[3] || node[5] || node[6] || id, x: 0, y: 0 });
      continue;
    }
    const edge = line.match(/^(\w+)\s*-->\s*(?:\|(\w+)\|)?\s*(\w+)/);
    if (edge) edges.push({ from: edge[1], to: edge[3], isError: edge[2] === "error" });
  }
  return { nodes, edges };
}

export function layoutGraph(nodes: GraphNode[], edges: GraphEdge[]) {
  const children = new Map<string, string[]>();
  for (const edge of edges) children.set(edge.from, [...(children.get(edge.from) ?? []), edge.to]);
  const visited = new Set<string>();
  const queue: Array<{ id: string; depth: number }> = [];
  const byId = new Map(nodes.map((node) => [node.id, node]));
  const incoming = new Set(edges.map((edge) => edge.to));
  const entry = nodes.find((node) => !incoming.has(node.id));
  if (entry) { queue.push({ id: entry.id, depth: 0 }); visited.add(entry.id); }
  let maxDepth = 0;
  while (queue.length) {
    const current = queue.shift()!;
    const node = byId.get(current.id);
    if (node) { node.x = 100; node.y = 40 + current.depth * NODE_SPACING; maxDepth = Math.max(maxDepth, current.depth); }
    for (const child of children.get(current.id) ?? []) {
      if (!visited.has(child)) { visited.add(child); queue.push({ id: child, depth: current.depth + 1 }); }
    }
  }
  let y = 40 + (maxDepth + 2) * NODE_SPACING;
  for (const node of nodes) if (!visited.has(node.id)) { node.x = 100; node.y = y; y += NODE_SPACING; }
  return nodes;
}

export function computeBBox(nodes: GraphNode[]): BBox {
  if (!nodes.length) return { minX: 0, minY: 0, maxX: 0, maxY: 0, width: 0, height: 0 };
  const minX = Math.min(...nodes.map((node) => node.x));
  const minY = Math.min(...nodes.map((node) => node.y));
  const maxX = Math.max(...nodes.map((node) => node.x + NODE_W));
  const maxY = Math.max(...nodes.map((node) => node.y + NODE_H));
  return { minX, minY, maxX, maxY, width: maxX - minX, height: maxY - minY };
}

export function zoomIndex(value: number) {
  return ZOOM_LEVELS.reduce((best, level, index) =>
    Math.abs(level - value) < Math.abs(ZOOM_LEVELS[best] - value) ? index : best, 0);
}

export function snapZoom(value: number) { return ZOOM_LEVELS[zoomIndex(value)]; }
