import { useEffect, useRef, useState, useCallback } from "react";
import { useDebugStore } from "../stores/useDebugStore";
import { useOverlayStore } from "../stores/useOverlayStore";
import { getCoords, imgToNatural, type Coords } from "../utils";

interface UseOverlayInteractionOptions {
  onCursorMove?: (x: number, y: number) => void;
  onCursorLeave?: () => void;
}

export function useOverlayInteraction(options: UseOverlayInteractionOptions = {}) {
  const { onCursorMove, onCursorLeave } = options;

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
  const pendingCoordsRef = useRef<Coords | null>(null);
  const pendingNaturalRef = useRef<{ x: number; y: number } | null>(null);
  const [dragStart, setDragStart] = useState<{ x: number; y: number } | null>(null);
  const [dragCurrent, setDragCurrent] = useState<{ x: number; y: number } | null>(null);
  const [lastCoords, setLastCoords] = useState<Coords | null>(null);

  const handleMouseMove = useCallback(
    (e: React.MouseEvent) => {
      const container = containerRef.current;
      const img = imgRef.current;
      if (!container || !img) return;
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
            if (dragStart) setDragCurrent({ x: pending.overlayX, y: pending.overlayY });
          }
          if (natural) onCursorMove?.(natural.x, natural.y);
        });
      }
    },
    [dragStart, onCursorMove, onCursorLeave],
  );

  useEffect(() => () => {
    if (rafRef.current) cancelAnimationFrame(rafRef.current);
  }, []);

  const handleMouseDown = useCallback(
    (e: React.MouseEvent) => {
      const container = containerRef.current;
      const img = imgRef.current;
      if (!container || !img) return;
      const c = getCoords(e, container, img);
      if (!c) return;
      setDragStart({ x: c.overlayX, y: c.overlayY });
      setDragCurrent({ x: c.overlayX, y: c.overlayY });
      setSelection(null);
    },
    [setSelection],
  );

  const handleMouseUp = useCallback(
    (e: React.MouseEvent) => {
      const container = containerRef.current;
      const img = imgRef.current;
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
    [dragStart, lastCoords, setSelection],
  );

  const handleMouseLeave = useCallback(() => {
    onCursorLeave?.();
  }, [onCursorLeave]);

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
