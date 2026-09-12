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

export interface MermaidGraphProps {
  mermaid: string;
  currentNodeId: string | null;
  selectedNodeId: string | null;
  onNodeClick: (id: string | null) => void;
}

export default function MermaidGraph({
  mermaid,
  currentNodeId,
  selectedNodeId,
  onNodeClick,
}: MermaidGraphProps) {
  const [zoom, setZoom] = useState(1.0);
  const [panX, setPanX] = useState(0);
  const [panY, setPanY] = useState(0);
  const [isPanning, setIsPanning] = useState(false);
  const [spaceHeld, setSpaceHeld] = useState(false);
  const [containerSize, setContainerSize] = useState({ w: 600, h: 400 });

  const containerRef = useRef<HTMLDivElement>(null);
  const panStartRef = useRef({ x: 0, y: 0, panX: 0, panY: 0 });
  const didPanRef = useRef(false);
  const mouseDownOnNodeRef = useRef(false);
  const panFrameRef = useRef(0);
  const pendingPanRef = useRef({ x: 0, y: 0 });

  const graph = useMemo(() => {
    if (!mermaid) return { nodes: [], edges: [] };
    const { nodes, edges } = parseMermaidGraph(mermaid);
    return { nodes: layoutGraph(nodes, edges), edges };
  }, [mermaid]);

  const { nodes, edges } = graph;
  const bbox = useMemo(() => computeBBox(nodes), [nodes]);

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

  const onWheel = useCallback((e: React.WheelEvent) => {
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
  }, [isPanning]);

  const onMouseUp = useCallback(() => {
    setIsPanning(false);
  }, []);

  useEffect(() => () => {
    if (panFrameRef.current) cancelAnimationFrame(panFrameRef.current);
  }, []);

  const onNodeMouseDown = useCallback(() => {
    mouseDownOnNodeRef.current = true;
  }, []);

  const onBgMouseDown = useCallback(() => {
    mouseDownOnNodeRef.current = false;
  }, []);

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
              stroke={edge.isError ? "#ef4444" : "#666"}
              strokeWidth={edge.isError ? 2 : 1.5}
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
        const fill = isCurrent ? "#dbeafe" : isSelected ? "#f0f9ff" : "#fff";
        const stroke = isCurrent ? "#3b82f6" : isSelected ? "#60a5fa" : "#e0e0e0";
        const strokeW = isCurrent || isSelected ? 2 : 1;
        const textColor = isCurrent ? "#1d4ed8" : "#333";
        return (
          <g
            key={node.id}
            className={`testbed-graph-node ${isSelected ? "selected" : ""}`}
            onMouseDown={onNodeMouseDown}
            onClick={(e) => { e.stopPropagation(); onNodeClick(isSelected ? null : node.id); }}
          >
            <rect x={node.x} y={node.y} width={NODE_W} height={NODE_H} rx={6} fill={fill} stroke={stroke} strokeWidth={strokeW} />
            <text x={node.x + NODE_W / 2} y={node.y + NODE_H / 2 + 4} textAnchor="middle" fill={textColor} fontSize={13} fontWeight={isCurrent || isSelected ? 600 : 400}>
              {node.label}
            </text>
          </g>
        );
      })}
    </>
  ), [nodes, currentNodeId, selectedNodeId, onNodeClick, onNodeMouseDown]);

  const showMinimap = nodes.length > 0;
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
        className={`testbed-graph-flow ${isPanning ? "panning" : ""} ${spaceHeld ? "space-held" : ""}`}
        ref={containerRef}
        onWheel={onWheel}
        onMouseDown={(e) => { onBgMouseDown(); onMouseDown(e); }}
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
            <marker id="ah" markerWidth="10" markerHeight="7" refX="10" refY="3.5" orient="auto">
              <polygon points="0 0, 10 3.5, 0 7" fill="#666" />
            </marker>
            <marker id="ah-err" markerWidth="10" markerHeight="7" refX="10" refY="3.5" orient="auto">
              <polygon points="0 0, 10 3.5, 0 7" fill="#ef4444" />
            </marker>
          </defs>

          {staticGraphSvg}
          {staticNodesSvg}
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
