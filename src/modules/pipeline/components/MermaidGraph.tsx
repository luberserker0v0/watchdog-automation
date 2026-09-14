import { useState, useEffect, useRef, useCallback, useMemo, memo } from "react";
import {
  computeBBox,
  layoutGraph,
  NODE_H,
  NODE_W,
  parseMermaidGraph,
  snapZoom,
  ZOOM_LEVELS,
  zoomIndex,
  type BBox,
  type GraphEdge,
  type GraphNode,
} from "./mermaidGraphModel";
import { savePipelineLayout, type PipelineGraph } from "../../../api/pipeline";
import { useGraphLayoutStore } from "../stores/useGraphLayoutStore";

const EMPTY_LAYOUT: Record<string, { x: number; y: number }> = {};

export interface MermaidGraphProps {
  mermaid: string;
  flow?: PipelineGraph | null;
  currentNodeId: string | null;
  selectedNodeId: string | null;
  onNodeClick: (id: string | null) => void;
  focusCurrentNode?: boolean;
  layoutKey?: string;
  allowNodeLayout?: boolean;
  layoutWritable?: boolean;
}

export default function MermaidGraph({
  mermaid,
  flow,
  currentNodeId,
  selectedNodeId,
  onNodeClick,
  focusCurrentNode = false,
  layoutKey,
  allowNodeLayout = false,
  layoutWritable = true,
}: MermaidGraphProps) {
  const [zoom, setZoom] = useState(1.0);
  const [panX, setPanX] = useState(0);
  const [panY, setPanY] = useState(0);
  const [isPanning, setIsPanning] = useState(false);
  const [spaceHeld, setSpaceHeld] = useState(false);
  const [containerSize, setContainerSize] = useState({ w: 600, h: 400 });
  const [layoutEditing, setLayoutEditing] = useState(false);
  const [regionSelecting, setRegionSelecting] = useState(false);
  const [gridVisible, setGridVisible] = useState(false);
  const [selectedLayoutNodeIds, setSelectedLayoutNodeIds] = useState<string[]>([]);
  const [selectionRect, setSelectionRect] = useState<{ x: number; y: number; width: number; height: number } | null>(null);
  const [dragPositions, setDragPositions] = useState<Record<string, { x: number; y: number }>>({});
  const [layoutSaveStatus, setLayoutSaveStatus] = useState("");
  const savedLayout = useGraphLayoutStore((state) => layoutKey ? state.layouts[layoutKey] ?? EMPTY_LAYOUT : EMPTY_LAYOUT);
  const setNodePositions = useGraphLayoutStore((state) => state.setNodePositions);
  const replaceLayout = useGraphLayoutStore((state) => state.replaceLayout);
  const clearLayout = useGraphLayoutStore((state) => state.clearLayout);
  const minimapVisible = useGraphLayoutStore((state) => state.minimapVisible);
  const setMinimapVisible = useGraphLayoutStore((state) => state.setMinimapVisible);

  const containerRef = useRef<HTMLDivElement>(null);
  const panStartRef = useRef({ x: 0, y: 0, panX: 0, panY: 0 });
  const didPanRef = useRef(false);
  const mouseDownOnNodeRef = useRef(false);
  const panFrameRef = useRef(0);
  const pendingPanRef = useRef({ x: 0, y: 0 });
  const nodeDragFrameRef = useRef(0);
  const nodeDragRef = useRef<{ startX: number; startY: number; positions: Record<string, { x: number; y: number }> } | null>(null);
  const pendingNodePositionsRef = useRef<Record<string, { x: number; y: number }> | null>(null);
  const didDragNodeRef = useRef(false);
  const regionSelectionRef = useRef<{ x: number; y: number } | null>(null);

  const baseGraph = useMemo(() => {
    if (flow) {
      const nodes = flow.nodes.map((node) => ({ id: node.id, label: node.label || node.id, x: 0, y: 0 }));
      const edges = flow.edges.map((edge) => ({ from: edge.from, to: edge.to, isError: edge.edge_type === "error" }));
      return { nodes: layoutGraph(nodes, edges), edges };
    }
    if (!mermaid) return { nodes: [], edges: [] };
    const { nodes, edges } = parseMermaidGraph(mermaid);
    return { nodes: layoutGraph(nodes, edges), edges };
  }, [flow, mermaid]);

  const edges = baseGraph.edges;
  const nodes = useMemo(() => baseGraph.nodes.map((node) => {
    const position = dragPositions[node.id] ?? savedLayout[node.id];
    return position ? { ...node, x: position.x, y: position.y } : node;
  }), [baseGraph.nodes, dragPositions, savedLayout]);
  const bbox = useMemo(() => computeBBox(nodes), [nodes]);

  useEffect(() => {
    if (!focusCurrentNode || !currentNodeId || nodeDragRef.current) return;
    const current = nodes.find((node) => node.id === currentNodeId);
    if (!current) return;
    setPanX(containerSize.w / 2 - (current.x + NODE_W / 2) * zoom);
    setPanY(containerSize.h / 2 - (current.y + NODE_H / 2) * zoom);
  }, [containerSize.h, containerSize.w, currentNodeId, focusCurrentNode, nodes, zoom]);

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    const ro = new ResizeObserver((entries) => {
      const { width, height } = entries[0].contentRect;
      setContainerSize({ w: width, h: height });
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  const zoomAt = useCallback((newZoomRaw: number, clientX: number, clientY: number) => {
    const newZoom = snapZoom(Math.max(0.25, Math.min(3.0, newZoomRaw)));
    const el = containerRef.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    const sx = clientX - rect.left;
    const sy = clientY - rect.top;
    const gx = (sx - panX) / zoom;
    const gy = (sy - panY) / zoom;
    setPanX(sx - gx * newZoom);
    setPanY(sy - gy * newZoom);
    setZoom(newZoom);
  }, [zoom, panX, panY]);

  const onWheel = useCallback((e: WheelEvent) => {
    e.preventDefault();
    const idx = zoomIndex(zoom);
    if (e.deltaY < 0) {
      const next = Math.min(ZOOM_LEVELS.length - 1, idx + 1);
      zoomAt(ZOOM_LEVELS[next], e.clientX, e.clientY);
    } else {
      const prev = Math.max(0, idx - 1);
      zoomAt(ZOOM_LEVELS[prev], e.clientX, e.clientY);
    }
  }, [zoom, zoomAt]);

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;
    container.addEventListener("wheel", onWheel, { passive: false });
    return () => container.removeEventListener("wheel", onWheel);
  }, [flow, mermaid, onWheel]);

  const clientToGraph = useCallback((clientX: number, clientY: number) => {
    const rect = containerRef.current?.getBoundingClientRect();
    if (!rect) return null;
    return {
      x: (clientX - rect.left - panX) / zoom,
      y: (clientY - rect.top - panY) / zoom,
    };
  }, [panX, panY, zoom]);

  const onMouseDown = useCallback((e: React.MouseEvent) => {
    if (e.button === 1) {
      e.preventDefault();
      didPanRef.current = false;
      mouseDownOnNodeRef.current = false;
      panStartRef.current = { x: e.clientX, y: e.clientY, panX, panY };
      setIsPanning(true);
      return;
    }
    if (e.button === 0 && (spaceHeld || !mouseDownOnNodeRef.current)) {
      if (spaceHeld || !mouseDownOnNodeRef.current) {
        e.preventDefault();
        didPanRef.current = false;
        panStartRef.current = { x: e.clientX, y: e.clientY, panX, panY };
        setIsPanning(true);
      }
    }
  }, [panX, panY, spaceHeld]);

  const onMouseMove = useCallback((e: React.MouseEvent) => {
    if (regionSelectionRef.current) {
      const current = clientToGraph(e.clientX, e.clientY);
      if (!current) return;
      const start = regionSelectionRef.current;
      setSelectionRect({
        x: Math.min(start.x, current.x),
        y: Math.min(start.y, current.y),
        width: Math.abs(current.x - start.x),
        height: Math.abs(current.y - start.y),
      });
      return;
    }
    if (nodeDragRef.current) {
      const drag = nodeDragRef.current;
      const clientDx = e.clientX - drag.startX;
      const clientDy = e.clientY - drag.startY;
      const dx = clientDx / zoom;
      const dy = clientDy / zoom;
      if (Math.abs(clientDx) > 2 || Math.abs(clientDy) > 2) didDragNodeRef.current = true;
      pendingNodePositionsRef.current = Object.fromEntries(Object.entries(drag.positions).map(([id, position]) => [
        id,
        { x: position.x + dx, y: position.y + dy },
      ]));
      if (!nodeDragFrameRef.current) {
        nodeDragFrameRef.current = requestAnimationFrame(() => {
          nodeDragFrameRef.current = 0;
          setDragPositions(pendingNodePositionsRef.current ?? {});
        });
      }
      return;
    }
    if (!isPanning) return;
    const dx = e.clientX - panStartRef.current.x;
    const dy = e.clientY - panStartRef.current.y;
    if (Math.abs(dx) > 2 || Math.abs(dy) > 2) didPanRef.current = true;
    pendingPanRef.current = { x: panStartRef.current.panX + dx, y: panStartRef.current.panY + dy };
    if (!panFrameRef.current) {
      panFrameRef.current = requestAnimationFrame(() => {
        panFrameRef.current = 0;
        setPanX(pendingPanRef.current.x);
        setPanY(pendingPanRef.current.y);
      });
    }
  }, [clientToGraph, isPanning, zoom]);

  const onMouseUp = useCallback(() => {
    if (regionSelectionRef.current && selectionRect) {
      setSelectedLayoutNodeIds(nodes.filter((node) => (
        node.x < selectionRect.x + selectionRect.width
        && node.x + NODE_W > selectionRect.x
        && node.y < selectionRect.y + selectionRect.height
        && node.y + NODE_H > selectionRect.y
      )).map((node) => node.id));
    }
    regionSelectionRef.current = null;
    setSelectionRect(null);

    const finalPositions = pendingNodePositionsRef.current;
    if (nodeDragRef.current && finalPositions && layoutKey) {
      setNodePositions(layoutKey, finalPositions);
      const completeLayout = { ...savedLayout, ...finalPositions };
      setLayoutSaveStatus("Saving…");
      void savePipelineLayout(layoutKey, completeLayout)
        .then(() => setLayoutSaveStatus("Saved to layout file"))
        .catch((error) => {
          replaceLayout(layoutKey, savedLayout);
          setLayoutSaveStatus(`Save failed: ${String(error)}`);
        });
    }
    nodeDragRef.current = null;
    pendingNodePositionsRef.current = null;
    setDragPositions({});
    setIsPanning(false);
  }, [layoutKey, nodes, replaceLayout, savedLayout, selectionRect, setNodePositions]);

  useEffect(() => () => {
    if (panFrameRef.current) cancelAnimationFrame(panFrameRef.current);
    if (nodeDragFrameRef.current) cancelAnimationFrame(nodeDragFrameRef.current);
  }, []);

  const onNodeMouseDown = useCallback((event: React.MouseEvent, node: GraphNode) => {
    mouseDownOnNodeRef.current = true;
    if (!layoutEditing || !layoutWritable || event.button !== 0 || spaceHeld) return;
    event.preventDefault();
    event.stopPropagation();
    didDragNodeRef.current = false;
    pendingNodePositionsRef.current = null;
    const draggedIds = selectedLayoutNodeIds.includes(node.id) ? selectedLayoutNodeIds : [node.id];
    if (!selectedLayoutNodeIds.includes(node.id)) setSelectedLayoutNodeIds([node.id]);
    nodeDragRef.current = {
      startX: event.clientX,
      startY: event.clientY,
      positions: Object.fromEntries(nodes
        .filter((candidate) => draggedIds.includes(candidate.id))
        .map((candidate) => [candidate.id, { x: candidate.x, y: candidate.y }])),
    };
  }, [layoutEditing, layoutWritable, nodes, selectedLayoutNodeIds, spaceHeld]);

  const onBgMouseDown = useCallback(() => {
    mouseDownOnNodeRef.current = false;
  }, []);

  const onGraphMouseDown = useCallback((event: React.MouseEvent) => {
    onBgMouseDown();
    if (layoutEditing && regionSelecting && event.button === 0 && !spaceHeld) {
      const start = clientToGraph(event.clientX, event.clientY);
      if (!start) return;
      event.preventDefault();
      regionSelectionRef.current = start;
      setSelectionRect({ x: start.x, y: start.y, width: 0, height: 0 });
      return;
    }
    onMouseDown(event);
  }, [clientToGraph, layoutEditing, onBgMouseDown, onMouseDown, regionSelecting, spaceHeld]);

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.code === "Space" && !e.repeat && !(e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement || e.target instanceof HTMLSelectElement)) {
        e.preventDefault();
        setSpaceHeld(true);
      }
    };
    const onKeyUp = (e: KeyboardEvent) => {
      if (e.code === "Space") {
        setSpaceHeld(false);
        setIsPanning(false);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    window.addEventListener("keyup", onKeyUp);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("keyup", onKeyUp);
    };
  }, []);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!(e.ctrlKey || e.metaKey)) return;
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement || e.target instanceof HTMLSelectElement) return;
      const el = containerRef.current;
      if (!el) return;
      const rect = el.getBoundingClientRect();
      const cx = rect.left + rect.width / 2;
      const cy = rect.top + rect.height / 2;
      if (e.key === "=" || e.key === "+") {
        e.preventDefault();
        const idx = Math.min(ZOOM_LEVELS.length - 1, zoomIndex(zoom) + 1);
        zoomAt(ZOOM_LEVELS[idx], cx, cy);
      } else if (e.key === "-") {
        e.preventDefault();
        const idx = Math.max(0, zoomIndex(zoom) - 1);
        zoomAt(ZOOM_LEVELS[idx], cx, cy);
      } else if (e.key === "1") {
        e.preventDefault();
        zoomAt(1.0, cx, cy);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [zoom, zoomAt]);

  const handleBgClick = useCallback((e: React.MouseEvent) => {
    if (didPanRef.current) return;
    if (e.target === e.currentTarget || (e.target as SVGElement).tagName === "svg") {
      onNodeClick(null);
    }
  }, [onNodeClick]);

  const staticGraphSvg = useMemo(() => (
    <>
      {edges.map((edge, i) => {
        const from = nodes.find((n) => n.id === edge.from);
        const to = nodes.find((n) => n.id === edge.to);
        if (!from || !to) return null;
        const x1 = from.x + NODE_W / 2;
        const y1 = from.y + NODE_H;
        const x2 = to.x + NODE_W / 2;
        const y2 = to.y;
        const midY = (y1 + y2) / 2;
        return (
          <g key={`${edge.from}-${edge.to}-${edge.isError}-${i}`}>
            <line
              x1={x1} y1={y1} x2={x2} y2={y2}
              stroke={edge.isError ? "#ef4444" : "var(--graph-edge-color)"}
              strokeWidth={2}
              strokeDasharray={edge.isError ? "5,3" : undefined}
              markerEnd={edge.isError ? "url(#ah-err)" : "url(#ah)"}
            />
            {edge.isError && (
              <g>
                <rect
                  x={(x1 + x2) / 2 - 26}
                  y={midY - 10}
                  width={52}
                  height={20}
                  rx={4}
                  fill="#fff"
                  stroke="#ef4444"
                  strokeWidth={1}
                />
                <text
                  x={(x1 + x2) / 2}
                  y={midY + 4}
                  textAnchor="middle"
                  fill="#ef4444"
                  fontSize={12}
                  fontWeight={700}
                  fontFamily="monospace"
                >
                  error
                </text>
              </g>
            )}
          </g>
        );
      })}
    </>
  ), [edges, nodes]);

  const staticNodesSvg = useMemo(() => (
    <>
      {nodes.map((node) => {
        const isCurrent = node.id === currentNodeId;
        const isSelected = node.id === selectedNodeId;
        const isLayoutSelected = selectedLayoutNodeIds.includes(node.id);
        const fill = isCurrent ? "#dbeafe" : isLayoutSelected ? "#f3e8ff" : isSelected ? "#f0f9ff" : "#fff";
        const stroke = isCurrent ? "#3b82f6" : isLayoutSelected ? "#a855f7" : isSelected ? "#60a5fa" : "#e0e0e0";
        const strokeW = isCurrent || isSelected || isLayoutSelected ? 2 : 1;
        const textColor = isCurrent ? "#1d4ed8" : "#333";
        return (
          <g
            key={node.id}
            className={`testbed-graph-node ${isSelected ? "selected" : ""} ${isLayoutSelected ? "layout-selected" : ""} ${layoutEditing ? "layout-editing" : ""}`}
            onMouseDown={(event) => onNodeMouseDown(event, node)}
            onClick={(e) => {
              e.stopPropagation();
              if (didDragNodeRef.current) {
                didDragNodeRef.current = false;
                return;
              }
              onNodeClick(isSelected ? null : node.id);
            }}
          >
            <rect x={node.x} y={node.y} width={NODE_W} height={NODE_H} rx={6} fill={fill} stroke={stroke} strokeWidth={strokeW} />
            <text x={node.x + NODE_W / 2} y={node.y + NODE_H / 2 + 4} textAnchor="middle" fill={textColor} fontSize={13} fontWeight={isCurrent || isSelected ? 600 : 400}>
              {node.label}
            </text>
          </g>
        );
      })}
    </>
  ), [nodes, currentNodeId, selectedNodeId, selectedLayoutNodeIds, onNodeClick, onNodeMouseDown]);

  const showMinimap = nodes.length > 0 && minimapVisible;
  const minimapScale = 0.12;

  if (nodes.length === 0) {
    return (
      <div className="testbed-graph-empty">
        <div className="testbed-empty-text">No graph to display</div>
        <div className="testbed-empty-hint">Parse errors in mermaid content</div>
      </div>
    );
  }

  const vbW = containerSize.w / zoom;
  const vbH = containerSize.h / zoom;
  const vbX = -panX / zoom;
  const vbY = -panY / zoom;

  return (
    <div className="testbed-graph-wrapper">
      <div
        className={`testbed-graph-flow ${isPanning ? "panning" : ""} ${spaceHeld ? "space-held" : ""} ${regionSelecting ? "region-selecting" : ""}`}
        ref={containerRef}
        onMouseDown={onGraphMouseDown}
        onMouseMove={onMouseMove}
        onMouseUp={onMouseUp}
        onMouseLeave={onMouseUp}
        onClick={handleBgClick}
      >
        <svg
          width={containerSize.w}
          height={containerSize.h}
          viewBox={`${vbX} ${vbY} ${vbW} ${vbH}`}
        >
          <defs>
            <pattern id="graph-grid" width="24" height="24" patternUnits="userSpaceOnUse">
              <circle cx="1" cy="1" r="1.25" fill="var(--graph-grid-color)" />
            </pattern>
            <marker id="ah" markerWidth="10" markerHeight="7" refX="10" refY="3.5" orient="auto">
              <polygon points="0 0, 10 3.5, 0 7" fill="var(--graph-edge-color)" />
            </marker>
            <marker id="ah-err" markerWidth="10" markerHeight="7" refX="10" refY="3.5" orient="auto">
              <polygon points="0 0, 10 3.5, 0 7" fill="#ef4444" />
            </marker>
          </defs>

          {gridVisible && <rect x={vbX} y={vbY} width={vbW} height={vbH} fill="url(#graph-grid)" />}
          {staticGraphSvg}
          {staticNodesSvg}
          {selectionRect && (
            <rect
              className="testbed-region-selection"
              x={selectionRect.x}
              y={selectionRect.y}
              width={selectionRect.width}
              height={selectionRect.height}
            />
          )}
        </svg>
      </div>

      {showMinimap && (
        <MemoizedMinimap
          nodes={nodes}
          edges={edges}
          bbox={bbox}
          zoom={zoom}
          panX={panX}
          panY={panY}
          containerW={containerSize.w}
          containerH={containerSize.h}
          minimapScale={minimapScale}
          onNavigate={(gx, gy) => {
            setPanX(containerSize.w / 2 - gx * zoom - bbox.minX * zoom);
            setPanY(containerSize.h / 2 - gy * zoom - bbox.minY * zoom);
          }}
        />
      )}

      {allowNodeLayout && layoutKey && (
        <div className="testbed-layout-controls">
          <label title="Drag graph nodes without changing pipeline files">
            <input type="checkbox" checked={layoutEditing} disabled={!layoutWritable} onChange={(event) => {
              setLayoutEditing(event.target.checked);
              if (!event.target.checked) {
                setRegionSelecting(false);
                setSelectedLayoutNodeIds([]);
              }
            }} />
            <span>Edit layout</span>
          </label>
          <label title="Drag a rectangle, then drag one selected node to move the group">
            <input
              type="checkbox"
              checked={regionSelecting}
              disabled={!layoutWritable}
              onChange={(event) => {
                setRegionSelecting(event.target.checked);
                if (event.target.checked) setLayoutEditing(true);
                else setSelectedLayoutNodeIds([]);
              }}
            />
            <span>Region select</span>
          </label>
          <label title="Show an alignment grid behind the graph">
            <input type="checkbox" checked={gridVisible} onChange={(event) => setGridVisible(event.target.checked)} />
            <span>Grid</span>
          </label>
          <button
            type="button"
            disabled={!layoutWritable || Object.keys(savedLayout).length === 0}
            onClick={() => {
              const previousLayout = savedLayout;
              clearLayout(layoutKey);
              setLayoutSaveStatus("Saving…");
              void savePipelineLayout(layoutKey, {})
                .then(() => setLayoutSaveStatus("Auto layout saved to layout file"))
                .catch((error) => {
                  replaceLayout(layoutKey, previousLayout);
                  setLayoutSaveStatus(`Save failed: ${String(error)}`);
                });
            }}
            title="Discard saved positions and align nodes automatically"
          >
            Auto align
          </button>
          <button
            type="button"
            onClick={() => setMinimapVisible(!minimapVisible)}
            aria-pressed={minimapVisible}
            title="Remember minimap visibility for Pipeline graphs"
          >
            {minimapVisible ? "Hide minimap" : "Show minimap"}
          </button>
          {!layoutWritable && <span className="testbed-layout-status">Built-in layout is read-only</span>}
          {layoutWritable && layoutSaveStatus && <span className="testbed-layout-status" title={layoutSaveStatus}>{layoutSaveStatus}</span>}
        </div>
      )}

      <div className="testbed-graph-info">
        <span>Nodes: {nodes.length}</span>
        <span>Edges: {edges.length}</span>
        {selectedNodeId && <span>Selected: {selectedNodeId}</span>}
      </div>
    </div>
  );
}

const MemoizedMinimap = memo(function Minimap({
  nodes, edges, bbox, zoom, panX, panY, containerW, containerH, minimapScale, onNavigate,
}: {
  nodes: GraphNode[];
  edges: GraphEdge[];
  bbox: BBox;
  zoom: number;
  panX: number;
  panY: number;
  containerW: number;
  containerH: number;
  minimapScale: number;
  onNavigate: (gx: number, gy: number) => void;
}) {
  const mmW = 180;
  const mmH = 120;

  const graphW = bbox.width * minimapScale;
  const graphH = bbox.height * minimapScale;
  const offsetX = (mmW - graphW) / 2 - bbox.minX * minimapScale;
  const offsetY = (mmH - graphH) / 2 - bbox.minY * minimapScale;

  const vpX = (-panX / zoom - bbox.minX) * minimapScale + (mmW - graphW) / 2;
  const vpY = (-panY / zoom - bbox.minY) * minimapScale + (mmH - graphH) / 2;
  const vpW = (containerW / zoom) * minimapScale;
  const vpH = (containerH / zoom) * minimapScale;

  const svgRef = useRef<SVGSVGElement>(null);
  const isDraggingRef = useRef(false);

  const miniEdges = useMemo(() => (
    edges.map((edge, i) => {
      const from = nodes.find((n) => n.id === edge.from);
      const to = nodes.find((n) => n.id === edge.to);
      if (!from || !to) return null;
      return (
        <line
          key={`${edge.from}-${edge.to}-${edge.isError}-${i}`}
          x1={from.x * minimapScale + NODE_W / 2 * minimapScale + offsetX}
          y1={from.y * minimapScale + NODE_H * minimapScale + offsetY}
          x2={to.x * minimapScale + NODE_W / 2 * minimapScale + offsetX}
          y2={to.y * minimapScale + offsetY}
          stroke={edge.isError ? "#ef4444" : "#bbb"}
          strokeWidth={0.8}
        />
      );
    })
  ), [edges, nodes, minimapScale, offsetX, offsetY]);

  const miniNodes = useMemo(() => (
    nodes.map((node) => (
      <rect
        key={node.id}
        x={node.x * minimapScale + offsetX}
        y={node.y * minimapScale + offsetY}
        width={NODE_W * minimapScale}
        height={NODE_H * minimapScale}
        rx={2}
        fill="#3b82f6"
        opacity={0.6}
      />
    ))
  ), [nodes, minimapScale, offsetX, offsetY]);

  const clientToGraph = useCallback((clientX: number, clientY: number) => {
    const rect = svgRef.current?.getBoundingClientRect();
    if (!rect) return null;
    const mx = clientX - rect.left;
    const my = clientY - rect.top;
    const gx = (mx - (mmW - graphW) / 2) / minimapScale + bbox.minX;
    const gy = (my - (mmH - graphH) / 2) / minimapScale + bbox.minY;
    return { gx, gy };
  }, [mmW, mmH, graphW, graphH, minimapScale, bbox.minX, bbox.minY]);

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    if (e.button !== 0) return;
    isDraggingRef.current = true;
    const coords = clientToGraph(e.clientX, e.clientY);
    if (coords) onNavigate(coords.gx, coords.gy);
    e.stopPropagation();
  }, [clientToGraph, onNavigate]);

  useEffect(() => {
    const handleMouseMove = (e: MouseEvent) => {
      if (!isDraggingRef.current) return;
      const coords = clientToGraph(e.clientX, e.clientY);
      if (coords) onNavigate(coords.gx, coords.gy);
    };
    const handleMouseUp = () => {
      isDraggingRef.current = false;
    };
    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
    return () => {
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
  }, [clientToGraph, onNavigate]);

  return (
    <div className="testbed-minimap">
      <svg ref={svgRef} width={mmW} height={mmH} onMouseDown={handleMouseDown} style={{ cursor: "pointer" }}>
        <rect width={mmW} height={mmH} fill="#fafafa" stroke="#e0e0e0" rx={4} />
        {miniEdges}
        {miniNodes}
        <rect
          x={vpX}
          y={vpY}
          width={vpW}
          height={vpH}
          fill="rgba(59,130,246,0.08)"
          stroke="#f97316"
          strokeWidth={1.5}
          rx={2}
        />
      </svg>
    </div>
  );
});
