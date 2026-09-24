import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { loadPatrolChatImage, usePatrolChatStore, type PatrolChatContent, type PatrolChatEvent } from "../stores/usePatrolChatStore";
import "./patrol-chat-room.css";

function ImagePreview({ part }: { part: Extract<PatrolChatContent, { type: "image" }> }) {
  const { t } = useTranslation();
  const [url, setUrl] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [large, setLarge] = useState(false);
  useEffect(() => {
    let active = true;
    loadPatrolChatImage(part.image_id).then((value) => {
      if (active) setUrl(value);
    }).catch(() => {
      if (active) setError(t("patrol.imageUnavailable"));
    });
    return () => { active = false; };
  }, [part.image_id, t]);
  return <div className="patrol-chat-image">
    <strong>{part.label}</strong>
    {error && <span role="status">{error}</span>}
    {!error && !url && <span>{t("patrol.imageLoading")}</span>}
    {url && <button type="button" onClick={() => setLarge(!large)} aria-label={`${t(large ? "patrol.shrink" : "patrol.enlarge")} ${part.label}`}>
      <img src={url} alt={part.label} className={large ? "large" : ""} />
    </button>}
  </div>;
}

function MessageContent({ content }: { content: PatrolChatContent[] }) {
  return <div className="patrol-chat-content">{content.map((part, index) => {
    if (part.type === "image") return <ImagePreview key={`${part.image_id}-${index}`} part={part} />;
    if (part.type === "score") return <div className="patrol-chat-score" key={`${part.node_id}-${index}`}>
      <span>{part.node_id}</span><meter min={0} max={1} value={part.score} />
      <strong>{(part.score * 100).toFixed(0)}%</strong>
    </div>;
    if (part.type === "field") return <div className="patrol-chat-field" key={index}><span>{part.label}</span><strong>{part.value}</strong></div>;
    return <section className="patrol-chat-text" key={index}><strong>{part.label}</strong><p>{part.text}</p></section>;
  })}</div>;
}

function Message({ message }: { message: PatrolChatEvent }) {
  const { t } = useTranslation();
  const [expanded, setExpanded] = useState(false);
  const time = new Date(message.occurred_at_ms).toLocaleTimeString();
  const detailText = message.content.map((part) => part.type === "field" ? part.value : part.type === "text" ? part.text : "").join(" ");
  const summary = message.summary.includes("circuit") || detailText.includes("circuit opened")
    ? t("patrol.circuitOpen")
    : message.summary.includes("deadline") || detailText.includes("deadline expired")
      ? t("patrol.deadlineExpired")
      : detailText.includes("Low-confidence")
        ? t("patrol.lowConfidence")
        : detailText.includes("no newer frame")
          ? t("patrol.noNewerFrame")
        : detailText.includes("(unstable)")
          ? t("patrol.unstable")
          : t(`patrol.kind.${message.kind}`, { defaultValue: message.summary });
  return <article className={`patrol-chat-message ${message.sender}`} aria-label={`${message.sender} ${message.kind}`}>
    <div className="patrol-chat-message-head">
      <strong>{t(message.sender === "vlm" ? "patrol.vlm" : "patrol.patrol")}</strong>
      <span>{message.tier && `${message.tier} · `}{time}</span>
    </div>
    <p className="patrol-chat-summary">{summary}</p>
    <details onToggle={(event) => setExpanded(event.currentTarget.open)}>
      <summary>{t(expanded ? "patrol.hideDetails" : "patrol.showDetails")}</summary>
      {expanded && <>
        <div className="patrol-chat-context">{t("patrol.node")} {message.node_id} · {t("patrol.frame")} {message.frame} · {t("patrol.session")} {message.session}<br />{message.target}</div>
        <MessageContent content={message.content} />
      </>}
    </details>
  </article>;
}

export default function PatrolChatRoom() {
  const { t } = useTranslation();
  const messages = usePatrolChatStore((state) => state.messages);
  const runStatuses = usePatrolChatStore((state) => state.runStatuses);
  const selectedRunId = usePatrolChatStore((state) => state.selectedRunId);
  const selectRun = usePatrolChatStore((state) => state.selectRun);
  const autoScroll = usePatrolChatStore((state) => state.autoScroll);
  const setAutoScroll = usePatrolChatStore((state) => state.setAutoScroll);
  const clear = usePatrolChatStore((state) => state.clear);
  const scrollRef = useRef<HTMLDivElement>(null);
  const runs = useMemo(() => [...new Set(messages.map((message) => message.run_id))].reverse(), [messages]);
  const activeRun = selectedRunId === "latest" ? runs[0] : selectedRunId;
  const visible = useMemo(() => messages.filter((message) => activeRun === "all" || message.run_id === activeRun).sort((a, b) => a.id - b.id), [messages, activeRun]);

  useEffect(() => {
    if (autoScroll && scrollRef.current) scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
  }, [visible.length, autoScroll, activeRun]);

  return <section className="patrol-chat-room" aria-label={t("patrol.title")}>
    <header className="patrol-chat-toolbar">
      <div><h2>{t("patrol.title")}</h2><p>{t("patrol.subtitle")}{activeRun !== "all" && activeRun && ` · ${activeRun}: ${runStatuses[activeRun] ?? t("patrol.recorded")}`}</p></div>
      <div className="patrol-chat-controls">
        <label>{t("patrol.run")} <select aria-label={t("patrol.run")} value={selectedRunId} onChange={(event) => selectRun(event.target.value)}>
          <option value="latest">{t("patrol.latest")}</option><option value="all">{t("patrol.all")}</option>
          {runs.map((run) => <option key={run} value={run}>{run} · {runStatuses[run] ?? t("patrol.recorded")}</option>)}
        </select></label>
        <label><input type="checkbox" checked={autoScroll} onChange={(event) => setAutoScroll(event.target.checked)} /> {t("patrol.autoScroll")}</label>
        <button type="button" onClick={() => void clear()}>{t("patrol.clear")}</button>
      </div>
    </header>
    <div className="patrol-chat-timeline" ref={scrollRef}>
      {visible.length === 0 && <div className="patrol-chat-empty">{t("patrol.empty")}</div>}
      {visible.map((message, index) => <div key={message.id}>
        {(index === 0 || visible[index - 1].sample_id !== message.sample_id) && <div className="patrol-chat-sample">{t("patrol.runGroup", { run: message.run_id, node: message.node_id, frame: message.frame })}</div>}
        <Message message={message} />
      </div>)}
    </div>
  </section>;
}
