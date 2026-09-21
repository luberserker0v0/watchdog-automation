import { useCallback, useEffect, useRef, type ReactNode } from "react";

interface ResizableSplitProps {
  left: ReactNode;
  right: ReactNode;
  rightCollapsed?: boolean;
  rightHidden?: boolean;
  collapsedWidth?: number;
  defaultRightWidth?: number;
}

export default function ResizableSplit({
  left,
  right,
  rightCollapsed = false,
  rightHidden = false,
  collapsedWidth = 48,
  defaultRightWidth = 320,
}: ResizableSplitProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const leftRef = useRef<HTMLDivElement>(null);
  const rightRef = useRef<HTMLDivElement>(null);
  const dragging = useRef(false);
  const expandedRightWidth = useRef(defaultRightWidth);

  const applyExpandedWidth = useCallback((containerWidth: number, requestedWidth: number) => {
    if (!containerWidth) return;
    const minimumRight = Math.min(260, Math.max(0, containerWidth - 8));
    const minimumLeft = Math.min(420, Math.max(0, containerWidth - minimumRight - 8));
    const maximumRight = Math.max(minimumRight, Math.min(520, containerWidth - minimumLeft - 8));
    const rightWidth = Math.max(minimumRight, Math.min(maximumRight, requestedWidth));
    if (leftRef.current) leftRef.current.style.width = `calc(100% - ${rightWidth + 8}px)`;
    if (rightRef.current) rightRef.current.style.width = `${rightWidth}px`;
  }, []);

  const stop = useCallback(() => {
    dragging.current = false;
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
  }, []);
  const move = useCallback((event: MouseEvent) => {
    if (!dragging.current || !containerRef.current || rightCollapsed || rightHidden) return;
    const rect = containerRef.current.getBoundingClientRect();
    if (!rect.width) return;
    expandedRightWidth.current = Math.max(260, Math.min(520, rect.right - event.clientX));
    applyExpandedWidth(rect.width, expandedRightWidth.current);
  }, [applyExpandedWidth, rightCollapsed, rightHidden]);

  useEffect(() => {
    const rightWidth = rightHidden ? 0 : rightCollapsed ? collapsedWidth : expandedRightWidth.current;
    const dividerWidth = rightHidden || rightCollapsed ? 0 : 8;
    if (leftRef.current) leftRef.current.style.width = `calc(100% - ${rightWidth + dividerWidth}px)`;
    if (rightRef.current) rightRef.current.style.width = `${rightWidth}px`;
  }, [collapsedWidth, rightCollapsed, rightHidden]);

  useEffect(() => {
    if (rightCollapsed || rightHidden || !containerRef.current || typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(([entry]) => {
      applyExpandedWidth(entry.contentRect.width, expandedRightWidth.current);
    });
    observer.observe(containerRef.current);
    return () => observer.disconnect();
  }, [applyExpandedWidth, rightCollapsed, rightHidden]);

  useEffect(() => {
    window.addEventListener("mousemove", move);
    window.addEventListener("mouseup", stop);
    return () => {
      window.removeEventListener("mousemove", move);
      window.removeEventListener("mouseup", stop);
      stop();
    };
  }, [move, stop]);

  return (
    <div className={`split-layout ${rightCollapsed ? "right-collapsed" : ""} ${rightHidden ? "right-hidden" : ""}`} ref={containerRef}>
      <div className="panel-left" ref={leftRef} style={{ width: `calc(100% - ${rightHidden ? 0 : rightCollapsed ? collapsedWidth : defaultRightWidth + 8}px)` }}>{left}</div>
      <div className="divider" aria-hidden={rightCollapsed || rightHidden} onMouseDown={() => {
        if (rightCollapsed || rightHidden) return;
        dragging.current = true;
        document.body.style.cursor = "col-resize";
        document.body.style.userSelect = "none";
      }}><div className="divider-grip" /></div>
      <div className="panel-right" ref={rightRef} style={{ width: rightHidden ? 0 : rightCollapsed ? collapsedWidth : defaultRightWidth }}>{right}</div>
    </div>
  );
}
