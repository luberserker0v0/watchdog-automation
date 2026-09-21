import { useEffect, useMemo, useRef, useState } from "react";
import { loadPatrolChatImage, usePatrolChatStore, type PatrolChatContent, type PatrolChatEvent } from "../stores/usePatrolChatStore";
import "./patrol-chat-room.css";

function ImagePreview({ part }: { part: Extract<PatrolChatContent, { type: "image" }> }) {
  const [url, setUrl] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [large, setLarge] = useState(false);
  useEffect(() => {
    let active = true;
    loadPatrolChatImage(part.image_id).then((value) => {
      if (active) setUrl(value);
    }).catch(() => {
      if (active) setError("Image preview is no longer available in this session.");
    });
    return () => { active = false; };
  }, [part.image_id]);
  return <div className="patrol-chat-image">
    <strong>{part.label}</strong>
    {error && <span role="status">{error}</span>}
    {!error && !url && <span>Loading image…</span>}
    {url && <button type="button" onClick={() => setLarge(!large)} aria-label={`${large ? "Shrink" : "Enlarge"} ${part.label}`}>
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
  const [expanded, setExpanded] = useState(false);
  const time = new Date(message.occurred_at_ms).toLocaleTimeString();
  return <article className={`patrol-chat-message ${message.sender}`} aria-label={`${message.sender} ${message.kind}`}>
    <div className="patrol-chat-message-head">
      <strong>{message.sender === "vlm" ? "VLM" : "Patrol"}</strong>
      <span>{message.tier && `${message.tier} · `}{time}</span>
    </div>
    <p className="patrol-chat-summary">{message.summary}</p>
    <details onToggle={(event) => setExpanded(event.currentTarget.open)}>
      <summary>{expanded ? "Hide details" : "Show details"}</summary>
      {expanded && <>
        <div className="patrol-chat-context">Node {message.node_id} · frame {message.frame} · session {message.session}<br />{message.target}</div>
        <MessageContent content={message.content} />
      </>}
    </details>
  </article>;
}

export default function PatrolChatRoom() {
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

  return <section className="patrol-chat-room" aria-label="Patrol Chat Room">
    <header className="patrol-chat-toolbar">
      <div><h2>Patrol Chat Room</h2><p>Automatic Patrol and VLM activity · read only{activeRun !== "all" && activeRun && ` · ${activeRun}: ${runStatuses[activeRun] ?? "Recorded"}`}</p></div>
      <div className="patrol-chat-controls">
        <label>Run <select aria-label="Run" value={selectedRunId} onChange={(event) => selectRun(event.target.value)}>
          <option value="latest">Latest run</option><option value="all">All runs</option>
          {runs.map((run) => <option key={run} value={run}>{run} · {runStatuses[run] ?? "Recorded"}</option>)}
        </select></label>
        <label><input type="checkbox" checked={autoScroll} onChange={(event) => setAutoScroll(event.target.checked)} /> Auto-scroll</label>
        <button type="button" onClick={() => void clear()}>Clear session</button>
      </div>
    </header>
    <div className="patrol-chat-timeline" ref={scrollRef}>
      {visible.length === 0 && <div className="patrol-chat-empty">No Patrol exchange yet. Enable Patrol in Settings and start a pipeline.</div>}
      {visible.map((message, index) => <div key={message.id}>
        {(index === 0 || visible[index - 1].sample_id !== message.sample_id) && <div className="patrol-chat-sample">Run {message.run_id} · {message.node_id} · frame {message.frame}</div>}
        <Message message={message} />
      </div>)}
    </div>
  </section>;
}
