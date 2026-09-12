import { useCallback, useEffect, useRef, type ReactNode } from "react";

export default function ResizableSplit({ left, right }: { left: ReactNode; right: ReactNode }) {
  const containerRef = useRef<HTMLDivElement>(null);
  const leftRef = useRef<HTMLDivElement>(null);
  const rightRef = useRef<HTMLDivElement>(null);
  const dragging = useRef(false);

  const stop = useCallback(() => {
    dragging.current = false;
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
  }, []);
  const move = useCallback((event: MouseEvent) => {
    if (!dragging.current || !containerRef.current) return;
    const rect = containerRef.current.getBoundingClientRect();
    if (!rect.width) return;
    const leftRatio = Math.max(0.2, Math.min(0.8, (event.clientX - rect.left) / rect.width));
    if (leftRef.current) leftRef.current.style.width = `${leftRatio * 100}%`;
    if (rightRef.current) rightRef.current.style.width = `${(1 - leftRatio) * 100}%`;
  }, []);

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
    <div className="split-layout" ref={containerRef}>
      <div className="panel-left" ref={leftRef} style={{ width: "50%" }}>{left}</div>
      <div className="divider" onMouseDown={() => {
        dragging.current = true;
        document.body.style.cursor = "col-resize";
        document.body.style.userSelect = "none";
      }}><div className="divider-grip" /></div>
      <div className="panel-right" ref={rightRef} style={{ width: "50%" }}>{right}</div>
    </div>
  );
}
