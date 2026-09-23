import { useEffect, useState } from "react";
import { useRecorderStore } from "../stores/useRecorderStore";
import "../styles/recorder.css";

function duration(ms: number) {
  const seconds = Math.floor(ms / 1000);
  return `${String(Math.floor(seconds / 60)).padStart(2, "0")}:${String(seconds % 60).padStart(2, "0")}`;
}

export default function RecordingStatusBar() {
  const status = useRecorderStore((state) => state.status);
  const error = useRecorderStore((state) => state.error);
  const pause = useRecorderStore((state) => state.pause);
  const resume = useRecorderStore((state) => state.resume);
  const stop = useRecorderStore((state) => state.stop);
  const [now, setNow] = useState(Date.now());
  const [receivedAt, setReceivedAt] = useState(Date.now());
  useEffect(() => { setReceivedAt(Date.now()); setNow(Date.now()); }, [status.elapsed_ms, status.session_id, status.state]);
  useEffect(() => {
    if (status.state !== "recording") return;
    const timer = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, [status.state]);
  if (status.state === "idle") return null;
  const liveElapsed = status.state === "recording" ? status.elapsed_ms + Math.max(0, now - receivedAt) : status.elapsed_ms;
  return (
    <div className={`recording-status-bar ${status.state}`} role="status">
      <span className="recording-dot" />
      <strong>{status.state === "paused" ? "Recording paused" : status.state === "stopping" ? "Stopping recording" : "Recording"}</strong>
      {status.target_label && <span className="recording-target">{status.target_label}</span>}
      <span>{duration(liveElapsed)} · {status.event_count} actions</span>
      {(status.reason || error) && <span className="recording-reason">{status.reason || error}</span>}
      <span className="recording-spacer" />
      {status.state === "recording" && <button onClick={() => void pause()}>Pause</button>}
      {status.state === "paused" && <button onClick={() => void resume()}>Resume</button>}
      {status.state !== "stopping" && <button onClick={() => void stop()}>Stop</button>}
    </div>
  );
}
