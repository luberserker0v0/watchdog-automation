import { useEffect, useState } from "react";
import { useRecorderStore } from "../stores/useRecorderStore";
import { useTranslation } from "react-i18next";
import "../styles/recorder.css";

function duration(ms: number) {
  const seconds = Math.floor(ms / 1000);
  return `${String(Math.floor(seconds / 60)).padStart(2, "0")}:${String(seconds % 60).padStart(2, "0")}`;
}

export default function RecordingStatusBar() {
  const { t } = useTranslation();
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
      <strong>{status.state === "paused" ? t("recordingStatus.paused") : status.state === "stopping" ? t("recordingStatus.stopping") : t("recordingStatus.recording")}</strong>
      {status.target_label && <span className="recording-target">{status.target_label}</span>}
      <span>{duration(liveElapsed)} · {t("recordingStatus.actions", { count: status.event_count })}</span>
      {(status.reason || error) && <span className="recording-reason">{status.reason || error}</span>}
      <span className="recording-spacer" />
      {status.state === "recording" && <button onClick={() => void pause()}>{t("common.pause")}</button>}
      {status.state === "paused" && <button onClick={() => void resume()}>{t("common.resume")}</button>}
      {status.state !== "stopping" && <button onClick={() => void stop()}>{t("common.stop")}</button>}
    </div>
  );
}
