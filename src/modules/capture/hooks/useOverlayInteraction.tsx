import { useEffect, useLayoutEffect, useRef, useState, useCallback } from "react";
import { useDebugStore } from "../stores/useDebugStore";
import { useOverlayStore } from "../stores/useOverlayStore";
import { getCoords, imgToNatural, type Coords } from "../utils";

interface UseOverlayInteractionOptions {
  onCursorMove?: (x: number, y: number) => void;
  onCursorLeave?: () => void;
  resetKey?: string | number;
}

interface PreviewTransform {
  zoom: number;
  panX: number;
  panY: number;
}

export type PreviewInteractionMode = "drag" | "select" | "normal";

function InteractionModeIcon({ mode, active }: { mode: PreviewInteractionMode; active: boolean }) {
  const fill = active ? "currentColor" : "none";
  if (mode === "drag") {
    return <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M8 11V6a1.5 1.5 0 0 1 3 0v4-6a1.5 1.5 0 0 1 3 0v6-5a1.5 1.5 0 0 1 3 0v6-3a1.5 1.5 0 0 1 3 0v6c0 4-2.5 7-7 7h-1c-2.5 0-4-1-5.5-3L3 13.5A1.7 1.7 0 0 1 5.6 11L8 13" fill={fill} stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" /></svg>;
  }
  if (mode === "select") {
    return <svg viewBox="0 0 24 24" aria-hidden="true"><rect x="4" y="5" width="16" height="14" rx="1" fill={fill} stroke="currentColor" strokeWidth="1.5" strokeDasharray={active ? undefined : "3 2"} /></svg>;
  }
  return <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 3l13 9-6 1.5L9 20 5 3z" fill={fill} stroke="currentColor" strokeWidth="1.5" strokeLinejoin="round" /></svg>;
}

const MIN_ZOOM = 1;
const MAX_ZOOM = 8;

export function useOverlayInteraction(options: UseOverlayInteractionOptions = {}) {
  const { onCursorMove, onCursorLeave, resetKey } = options;

  const selection = useOverlayStore((s) => s.selection);
  const roi = useOverlayStore((s) => s.roi);
  const resultRoi = useOverlayStore((s) => s.resultRoi);
  const showSelection = useDebugStore((s) => s.showSelection);
  const showRoi = useDebugStore((s) => s.showRoi);
  const showResultRoi = useDebugStore((s) => s.showResultRoi);
  const setShowSelection = useDebugStore((s) => s.setShowSelection);
  const setShowRoi = useDebugStore((s) => s.setShowRoi);
  const setShowResultRoi = useDebugStore((s) => s.setShowResultRoi);
  const setSelection = useOverlayStore((s) => s.setSelection);
  const clearAll = useOverlayStore((s) => s.clearAll);

  const imgRef = useRef<HTMLImageElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const rafRef = useRef<number>(0);
  const zoomRafRef = useRef<number>(0);
  const transformRef = useRef<PreviewTransform>({ zoom: 1, panX: 0, panY: 0 });
  const pendingTransformRef = useRef<PreviewTransform | null>(null);
  const lastPointerRef = useRef<{ clientX: number; clientY: number } | null>(null);
  const pendingCoordsRef = useRef<Coords | null>(null);
  const pendingNaturalRef = useRef<{ x: number; y: number } | null>(null);
  const panStartRef = useRef<{ clientX: number; clientY: number; panX: number; panY: number } | null>(null);
  const [dragStart, setDragStart] = useState<{ x: number; y: number } | null>(null);
  const [dragCurrent, setDragCurrent] = useState<{ x: number; y: number } | null>(null);
  const [lastCoords, setLastCoords] = useState<Coords | null>(null);
  const [imageTransform, setImageTransform] = useState<PreviewTransform>(transformRef.current);
  const [interactionMode, setInteractionMode] = useState<PreviewInteractionMode>("select");
  const [isPanning, setIsPanning] = useState(false);

  const scheduleTransform = useCallback((next: PreviewTransform) => {
    pendingTransformRef.current = next;
    if (zoomRafRef.current) return;
    zoomRafRef.current = requestAnimationFrame(() => {
      zoomRafRef.current = 0;
      const pending = pendingTransformRef.current;
      if (!pending) return;
      pendingTransformRef.current = null;
      transformRef.current = pending;
      setImageTransform(pending);
    });
  }, []);

  const resetView = useCallback(() => {
    if (zoomRafRef.current) {
      cancelAnimationFrame(zoomRafRef.current);
      zoomRafRef.current = 0;
    }
    const reset = { zoom: 1, panX: 0, panY: 0 };
    pendingTransformRef.current = null;
    transformRef.current = reset;
    panStartRef.current = null;
    setIsPanning(false);
    setImageTransform(reset);
  }, []);

  const handleMouseMove = useCallback(
    (e: React.MouseEvent) => {
      const container = containerRef.current;
      const img = imgRef.current;
      if (!container || !img) return;
      lastPointerRef.current = { clientX: e.clientX, clientY: e.clientY };
      if (interactionMode === "drag" && panStartRef.current) {
        const start = panStartRef.current;
        scheduleTransform({
          zoom: transformRef.current.zoom,
          panX: start.panX + e.clientX - start.clientX,
          panY: start.panY + e.clientY - start.clientY,
        });
        return;
      }
      const c = getCoords(e, container, img);
      if (!c) {
        onCursorLeave?.();
        return;
      }
      const n = imgToNatural(c.imgX, c.imgY, c);
      pendingCoordsRef.current = c;
      pendingNaturalRef.current = n;
      if (!rafRef.current) {
        rafRef.current = requestAnimationFrame(() => {
          rafRef.current = 0;
          const pending = pendingCoordsRef.current;
          const natural = pendingNaturalRef.current;
          if (pending) {
            setLastCoords(pending);
            if (interactionMode === "select" && dragStart) setDragCurrent({ x: pending.overlayX, y: pending.overlayY });
          }
          if (natural) onCursorMove?.(natural.x, natural.y);
        });
      }
    },
    [dragStart, interactionMode, onCursorMove, onCursorLeave, scheduleTransform],
  );

  useEffect(() => () => {
    if (rafRef.current) cancelAnimationFrame(rafRef.current);
    if (zoomRafRef.current) cancelAnimationFrame(zoomRafRef.current);
  }, []);

  useEffect(() => {
    resetView();
    setLastCoords(null);
    panStartRef.current = null;
    setIsPanning(false);
  }, [resetKey, resetView]);

  useLayoutEffect(() => {
    const container = containerRef.current;
    const img = imgRef.current;
    const pointer = lastPointerRef.current;
    if (!container || !img || !pointer) return;
    const coords = getCoords(pointer, container, img);
    if (coords) setLastCoords(coords);
  }, [imageTransform]);

  const handleWheel = useCallback((e: WheelEvent) => {
    const container = containerRef.current;
    const img = imgRef.current;
    if (!container || !img) return;
    e.preventDefault();
    e.stopPropagation();
    lastPointerRef.current = { clientX: e.clientX, clientY: e.clientY };

    const current = pendingTransformRef.current ?? transformRef.current;
    const factor = Math.exp(-e.deltaY * 0.0015);
    const zoom = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, current.zoom * factor));
    if (zoom === current.zoom) return;
    let panX = 0;
    let panY = 0;
    if (zoom > MIN_ZOOM) {
      const rect = container.getBoundingClientRect();
      const cursorX = e.clientX - rect.left - rect.width / 2;
      const cursorY = e.clientY - rect.top - rect.height / 2;
      const imageX = (cursorX - current.panX) / current.zoom;
      const imageY = (cursorY - current.panY) / current.zoom;
      panX = cursorX - imageX * zoom;
      panY = cursorY - imageY * zoom;
    }
    scheduleTransform({ zoom, panX, panY });
  }, [scheduleTransform]);

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;
    container.addEventListener("wheel", handleWheel, { passive: false });
    return () => container.removeEventListener("wheel", handleWheel);
  }, [handleWheel]);

  const handleMouseDown = useCallback(
    (e: React.MouseEvent) => {
      const container = containerRef.current;
      const img = imgRef.current;
      if (!container || !img) return;
      if (interactionMode === "normal") return;
      if (interactionMode === "drag") {
        panStartRef.current = {
          clientX: e.clientX,
          clientY: e.clientY,
          panX: transformRef.current.panX,
          panY: transformRef.current.panY,
        };
        setIsPanning(true);
        return;
      }
      const c = getCoords(e, container, img);
      if (!c) return;
      setDragStart({ x: c.overlayX, y: c.overlayY });
      setDragCurrent({ x: c.overlayX, y: c.overlayY });
      setSelection(null);
    },
    [interactionMode, setSelection],
  );

  const handleMouseUp = useCallback(
    (e: React.MouseEvent) => {
      const container = containerRef.current;
      const img = imgRef.current;
      if (interactionMode === "drag") {
        panStartRef.current = null;
        setIsPanning(false);
        return;
      }
      if (interactionMode === "normal") return;
      if (!container || !img || !dragStart || !lastCoords) return;
      const c = getCoords(e, container, img);
      if (!c) {
        setDragStart(null);
        setDragCurrent(null);
        return;
      }
      const startImg = {
        x: dragStart.x - (c.overlayX - c.imgX),
        y: dragStart.y - (c.overlayY - c.imgY),
      };
      const endImg = { x: c.imgX, y: c.imgY };
      const startN = imgToNatural(startImg.x, startImg.y, c);
      const endN = imgToNatural(endImg.x, endImg.y, c);
      const x = Math.min(startN.x, endN.x);
      const y = Math.min(startN.y, endN.y);
      const w = Math.abs(endN.x - startN.x);
      const h = Math.abs(endN.y - startN.y);
      if (w > 1 || h > 1) setSelection({ x, y, w, h });
      setDragStart(null);
      setDragCurrent(null);
    },
    [dragStart, interactionMode, lastCoords, setSelection],
  );

  const handleMouseLeave = useCallback(() => {
    panStartRef.current = null;
    setIsPanning(false);
    onCursorLeave?.();
  }, [onCursorLeave]);

  const changeInteractionMode = useCallback((mode: PreviewInteractionMode) => {
    panStartRef.current = null;
    setIsPanning(false);
    setDragStart(null);
    setDragCurrent(null);
    setInteractionMode(mode);
  }, []);

  const renderDragRect = () => {
    if (!dragStart || !dragCurrent || !lastCoords) return null;
    const x = Math.min(dragStart.x, dragCurrent.x);
    const y = Math.min(dragStart.y, dragCurrent.y);
    const w = Math.abs(dragCurrent.x - dragStart.x);
    const h = Math.abs(dragCurrent.y - dragStart.y);
    return <div className="debug-rect-overlay" style={{ left: x, top: y, width: w, height: h }} />;
  };

  const renderSelectionRect = () => {
    if (!showSelection || !selection || !lastCoords || dragStart) return null;
    const { displayW, displayH, naturalW, naturalH, overlayX, overlayY, imgX, imgY } = lastCoords;
    const totalOffsetX = overlayX - imgX;
    const totalOffsetY = overlayY - imgY;
    const x = (selection.x / naturalW) * displayW + totalOffsetX;
    const y = (selection.y / naturalH) * displayH + totalOffsetY;
    const w = (selection.w / naturalW) * displayW;
    const h = (selection.h / naturalH) * displayH;
    return <div className="debug-rect-overlay finalized" style={{ left: x, top: y, width: w, height: h }} />;
  };

  const renderRoiRect = () => {
    if (!showRoi || !roi || !lastCoords) return null;
    const { displayW, displayH, naturalW, naturalH, overlayX, overlayY, imgX, imgY } = lastCoords;
    const totalOffsetX = overlayX - imgX;
    const totalOffsetY = overlayY - imgY;
    const isFullFrame = roi.x === 0 && roi.y === 0 && roi.w === 0 && roi.h === 0;
    const resolved = isFullFrame ? { x: 0, y: 0, w: naturalW, h: naturalH } : roi;
    const x = (resolved.x / naturalW) * displayW + totalOffsetX;
    const y = (resolved.y / naturalH) * displayH + totalOffsetY;
    const w = (resolved.w / naturalW) * displayW;
    const h = (resolved.h / naturalH) * displayH;
    if (w < 1 || h < 1) return null;
    return (
      <div className="debug-rect-overlay roi" style={{ left: x, top: y, width: w, height: h }}>
        <span className="debug-roi-label">ROI</span>
      </div>
    );
  };

  const renderResultRoiRect = () => {
    if (!showResultRoi || !resultRoi || !lastCoords) return null;
    const { displayW, displayH, naturalW, naturalH, overlayX, overlayY, imgX, imgY } = lastCoords;
    const totalOffsetX = overlayX - imgX;
    const totalOffsetY = overlayY - imgY;
    const x = (resultRoi.x / naturalW) * displayW + totalOffsetX;
    const y = (resultRoi.y / naturalH) * displayH + totalOffsetY;
    const w = (resultRoi.w / naturalW) * displayW;
    const h = (resultRoi.h / naturalH) * displayH;
    if (w < 1 || h < 1) return null;
    return (
      <div className="debug-rect-overlay result-roi" style={{ left: x, top: y, width: w, height: h }}>
        <span className="debug-result-roi-label">Result</span>
      </div>
    );
  };

  const renderSidePanel = (showPreview: boolean) => (
    <div className="debug-side-panel left">
      {showPreview && (
        <>
          <div className="debug-interaction-modes" role="group" aria-label="Preview interaction mode">
            {(["drag", "select", "normal"] as const).map((mode) => {
              const active = interactionMode === mode;
              const label = mode === "drag" ? "Drag mode" : mode === "select" ? "Select mode" : "Normal mode";
              return <button key={mode} type="button" className={active ? "active" : ""} aria-label={label} aria-pressed={active} title={label} onClick={() => changeInteractionMode(mode)}><InteractionModeIcon mode={mode} active={active} /></button>;
            })}
          </div>
          <label className="debug-overlay-check">
            <input
              type="checkbox"
              checked={showSelection}
              onChange={(e) => setShowSelection(e.target.checked)}
            />
            <span className="debug-overlay-dot selection" />
            <span>選取</span>
          </label>
          <label className="debug-overlay-check">
            <input
              type="checkbox"
              checked={showRoi}
              onChange={(e) => setShowRoi(e.target.checked)}
            />
            <span className="debug-overlay-dot roi" />
            <span>ROI</span>
          </label>
          <label className="debug-overlay-check">
            <input
              type="checkbox"
              checked={showResultRoi}
              onChange={(e) => setShowResultRoi(e.target.checked)}
            />
            <span className="debug-overlay-dot result" />
            <span>Result</span>
          </label>
          <button className="debug-overlay-clear-btn" onClick={clearAll}>
            清除全部
          </button>
          <div className="debug-zoom-controls">
            <span className="debug-zoom-value" title="Use the mouse wheel over the image to zoom">
              {Math.round(imageTransform.zoom * 100)}%
            </span>
            <button
              className="debug-fit-btn"
              type="button"
              onClick={resetView}
              disabled={imageTransform.zoom === 1 && imageTransform.panX === 0 && imageTransform.panY === 0}
              aria-label="Fit image"
              title="Reset position and zoom"
            >
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="M9 4H4v5M15 4h5v5M4 15v5h5M20 15v5h-5" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" />
              </svg>
              <span>Fit</span>
            </button>
          </div>
        </>
      )}
    </div>
  );

  return {
    imgRef,
    containerRef,
    handleMouseMove,
    handleMouseDown,
    handleMouseUp,
    handleMouseLeave,
    imageStyle: {
      transform: `translate(${imageTransform.panX}px, ${imageTransform.panY}px) scale(${imageTransform.zoom})`,
      transformOrigin: "center center",
    },
    zoom: imageTransform.zoom,
    interactionMode,
    isPanning,
    renderDragRect,
    renderSelectionRect,
    renderRoiRect,
    renderResultRoiRect,
    renderSidePanel,
    dragStart,
    lastCoords,
    selection,
  };
}
