import { useEffect, useState } from "react";
import { resizeWindowClient } from "../../../api/window";
import { useStreamStore } from "../stores/useStreamStore";
import { useWindowStore } from "../stores/useWindowStore";

export default function WindowResizeControl() {
  const selectedIndex = useWindowStore((state) => state.selectedIndex);
  const windows = useWindowStore((state) => state.windows);
  const fetchWindows = useWindowStore((state) => state.fetchWindows);
  const captureOwner = useStreamStore((state) => state.captureOwner);
  const selectedWindow = selectedIndex === null ? undefined : windows[selectedIndex];
  const hwnd = selectedWindow?.hwnd;
  const [width, setWidth] = useState("");
  const [height, setHeight] = useState("");
  const [status, setStatus] = useState("");
  const [isResizing, setIsResizing] = useState(false);

  useEffect(() => {
    setWidth("");
    setHeight("");
    setStatus("");
  }, [hwnd]);

  const parsedWidth = Number(width);
  const parsedHeight = Number(height);
  const validSize = Number.isInteger(parsedWidth) && parsedWidth > 0
    && Number.isInteger(parsedHeight) && parsedHeight > 0;
  const disabled = !hwnd || !validSize || isResizing || captureOwner === "pipeline";

  const handleResize = async () => {
    if (!hwnd || !validSize) return;
    setIsResizing(true);
    setStatus("Resizing…");
    try {
      const result = await resizeWindowClient(hwnd, [parsedWidth, parsedHeight]);
      const stream = useStreamStore.getState();
      stream.setCaptureSizes(stream.frameSize, result.actual);
      setWidth(String(result.actual[0]));
      setHeight(String(result.actual[1]));
      setStatus(`Resized to ${result.actual[0]}×${result.actual[1]}`);
      await fetchWindows();
    } catch (error) {
      setStatus(`Resize failed: ${String(error)}`);
    } finally {
      setIsResizing(false);
    }
  };

  return (
    <div className="window-resize-control">
      <span className="window-resize-label">Client size</span>
      <input
        aria-label="Client width"
        type="number"
        min="1"
        step="1"
        value={width}
        placeholder="Width"
        onChange={(event) => setWidth(event.target.value)}
      />
      <span aria-hidden="true">×</span>
      <input
        aria-label="Client height"
        type="number"
        min="1"
        step="1"
        value={height}
        placeholder="Height"
        onChange={(event) => setHeight(event.target.value)}
      />
      <button type="button" onClick={handleResize} disabled={disabled}>
        {isResizing ? "Resizing…" : "Resize"}
      </button>
      {captureOwner === "pipeline" && <span className="window-resize-status">Pipeline owns capture</span>}
      {captureOwner !== "pipeline" && status && <span className="window-resize-status" title={status}>{status}</span>}
    </div>
  );
}
