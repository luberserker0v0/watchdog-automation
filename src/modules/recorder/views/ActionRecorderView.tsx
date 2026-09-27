import type { Action } from "../../../api/pipeline";
import { useEffect, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { useRecorderStore } from "../stores/useRecorderStore";
import { recordedDelayBefore, toPipelineActions } from "../utils/recordingTiming";
import { exportRecording } from "../services/exportRecording";
import "../styles/recorder.css";

type RecorderIconName = "record" | "stop" | "pause" | "play" | "copy" | "export" | "trash" | "remove";

function RecorderIcon({ name }: { name: RecorderIconName }) {
  const paths: Record<RecorderIconName, ReactNode> = {
    record: <circle cx="12" cy="12" r="6" fill="currentColor" />,
    stop: <rect x="6" y="6" width="12" height="12" rx="2" fill="currentColor" />,
    pause: <><rect x="7" y="5" width="3.5" height="14" rx="1" fill="currentColor" /><rect x="13.5" y="5" width="3.5" height="14" rx="1" fill="currentColor" /></>,
    play: <path d="M8 5.5v13l10-6.5z" fill="currentColor" />,
    copy: <><rect x="8" y="8" width="10" height="11" rx="2" /><path d="M16 8V7a2 2 0 0 0-2-2H7a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h1" /></>,
    export: <><path d="M12 4v11m0 0 4-4m-4 4-4-4" /><path d="M5 17v2h14v-2" /></>,
    trash: <><path d="M5 7h14M9 7V5h6v2m2 0-1 12H8L7 7" /><path d="M10 10v6m4-6v6" /></>,
    remove: <><circle cx="12" cy="12" r="8" /><path d="m9 9 6 6m0-6-6 6" /></>,
  };
  return <svg className="recorder-button-icon" viewBox="0 0 24 24" aria-hidden="true" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">{paths[name]}</svg>;
}

function actionSummary(action: Action): string {
  switch (action.type) {
    case "Wait": return `Wait ${action.duration}ms`;
    case "Click": return `Click at ${JSON.stringify(action.target)}`;
    case "LongPress": return `Long press at ${JSON.stringify(action.target)} for ${action.duration}ms`;
    case "Swipe": return `Swipe ${JSON.stringify(action.begin)} → ${JSON.stringify(action.end)} in ${action.duration}ms`;
    case "Scroll": return `Scroll ${action.dy ?? 0} at ${JSON.stringify(action.target)}`;
    case "ClickKey": return `Press keys ${action.key.join(" + ")}`;
    case "InputText": return action.input_text === "{{recorded_text}}" ? "Typed text (masked)" : `Type “${action.input_text}”`;
    default: return action.type;
  }
}

export default function ActionRecorderView() {
  const { t } = useTranslation();
  const status = useRecorderStore((state) => state.status);
  const actions = useRecorderStore((state) => state.actions);
  const error = useRecorderStore((state) => state.error);
  const hotkeyError = useRecorderStore((state) => state.hotkeyError);
  const toggle = useRecorderStore((state) => state.toggle);
  const togglePause = useRecorderStore((state) => state.togglePause);
  const clear = useRecorderStore((state) => state.clear);
  const removeAction = useRecorderStore((state) => state.removeAction);
  const [pending, setPending] = useState<"toggle" | "pause" | "clear" | null>(null);
  const [feedback, setFeedback] = useState<"copied" | "exported" | null>(null);
  const [exportError, setExportError] = useState<string | null>(null);
  useEffect(() => {
    if (!feedback) return;
    const timer = window.setTimeout(() => setFeedback(null), 1800);
    return () => window.clearTimeout(timer);
  }, [feedback]);
  const run = async (operation: "toggle" | "pause" | "clear", action: () => Promise<void>) => {
    if (pending) return;
    setPending(operation);
    try { await action(); } finally { setPending(null); }
  };
  const copy = async () => {
    await navigator.clipboard.writeText(JSON.stringify(toPipelineActions(actions), null, 2));
    setFeedback("copied");
  };
  const exportDraft = async () => {
    setExportError(null);
    try {
      await exportRecording(actions);
      setFeedback("exported");
    } catch (error) {
      setExportError(t("recorder.exportError", { error: error instanceof Error ? error.message : String(error) }));
    }
  };
  const isActive = status.state !== "idle";
  const isStopping = status.state === "stopping";
  return (
    <div className="recorder-view">
      <header className="recorder-header">
        <div className="recorder-heading"><div className={`recorder-heading-mark ${isActive ? "active" : ""}`}><RecorderIcon name="record" /></div><div><h2>{t("recorder.title")}</h2><p>{t("recorder.subtitle")}</p></div>{actions.length > 0 && <span className="recorder-action-count">{t("recorder.action", { count: actions.length })}</span>}</div>
        <div className="recorder-toolbar" aria-label="Recorder controls">
          <div className="recorder-control-group recorder-session-controls">
            <button className={`recorder-control ${isActive ? "danger" : "primary"}`} disabled={isStopping || pending !== null} onClick={() => void run("toggle", toggle)} title={isActive ? "Stop and finalize this recording" : "Start recording the selected capture target"}>
              <RecorderIcon name={isActive ? "stop" : "record"} /><span>{pending === "toggle" ? (isActive ? t("recorder.stopping") : t("recorder.starting")) : isActive ? t("common.stop") : t("recorder.start")}</span>
            </button>
            {(status.state === "recording" || status.state === "paused") && <button className="recorder-control secondary" disabled={pending !== null} onClick={() => void run("pause", togglePause)} title={status.state === "paused" ? "Continue this recording" : "Pause without ending this recording"}>
              <RecorderIcon name={status.state === "paused" ? "play" : "pause"} /><span>{pending === "pause" ? t("recorder.updating") : status.state === "paused" ? t("common.resume") : t("common.pause")}</span>
            </button>}
          </div>
          <div className="recorder-control-group recorder-draft-controls">
            <button className={`recorder-control quiet ${feedback === "copied" ? "confirmed" : ""}`} disabled={!actions.length} onClick={() => void copy()} title="Copy only the pipeline action array"><RecorderIcon name="copy" /><span>{feedback === "copied" ? t("recorder.copied") : t("recorder.copy")}</span></button>
            <button className={`recorder-control quiet ${feedback === "exported" ? "confirmed" : ""}`} disabled={!actions.length} onClick={() => void exportDraft()} title={t("recorder.exportNote")}><RecorderIcon name="export" /><span>{feedback === "exported" ? t("recorder.exported") : t("recorder.export")}</span></button>
            <button className="recorder-control quiet destructive" disabled={status.state !== "idle" || !actions.length || pending !== null} onClick={() => void run("clear", clear)} title={status.state !== "idle" ? "Stop recording before clearing the draft" : "Clear all recorded actions"}><RecorderIcon name="trash" /><span>{pending === "clear" ? "Clearing…" : "Clear"}</span></button>
          </div>
        </div>
      </header>
      <div className="recorder-export-note">{t("recorder.exportNote")}</div>
      {exportError && <div className="recorder-alert">{exportError}</div>}
      {(error || hotkeyError) && <div className="recorder-alert">{error || hotkeyError}</div>}
      {!actions.length ? (
        <div className="recorder-empty">{t("recorder.empty")}</div>
      ) : (
        <div className="recorder-timeline">
          {actions.map((item, index) => (
            <article className="recorded-action" key={item.id}>
              <div className="recorded-action-time">+{(item.offset_ms / 1000).toFixed(2)}s{index > 0 && <> · wait {(recordedDelayBefore(actions, index) / 1000).toFixed(2)}s</>}</div>
              <div className="recorded-action-content"><strong>{actionSummary(item.action)}</strong><button className="recorded-action-remove" onClick={() => removeAction(item.id)} title={t("common.remove")}><RecorderIcon name="remove" /><span>{t("common.remove")}</span></button><div className="recorded-action-source">{item.source}</div>{item.warning && <p className="recorded-action-warning">{item.warning}</p>}</div>
            </article>
          ))}
        </div>
      )}
    </div>
  );
}
