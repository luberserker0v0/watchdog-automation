import { useState } from "react";
import { testAction, type Action, type ActionTestContext } from "../../../api/pipeline";
import { useLogStore } from "../../observability/stores/useLogStore";

interface Props {
  context: ActionTestContext;
  action: Action & { type: "Swipe" };
}

function formatTarget(target: unknown): string {
  if (target === true) return "Recognized region center";
  if (target === false) return "None";
  if (Array.isArray(target)) {
    if (target.length === 2) return `(${target[0]}, ${target[1]})`;
    if (target.length === 4) return `(${target[0]}, ${target[1]}) ${target[2]}×${target[3]}`;
  }
  if (typeof target === "string") return `Node: ${target}`;
  return String(target);
}

export default function SwipeActionTester({ action, context }: Props) {
  const [result, setResult] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const appendLog = useLogStore((s) => s.appendLog);

  const handleExecute = async () => {
    setLoading(true);
    setResult("");
    setError("");
    try {
      const msg = await testAction(action, context);
      setResult(msg);
      appendLog({ source: "debug", label: "NodeTest", detail: `Swipe: ${msg}`, result: "success" });
    } catch (e) {
      setError(String(e));
      appendLog({ source: "debug", label: "NodeTest", detail: `Swipe: Error: ${e}`, result: "error" });
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="tester-panel">
      <div className="tester-row">
        <span className="tester-label">Begin</span>
        <code className="tester-value">{formatTarget(action.begin)}</code>
      </div>
      {action.begin_offset && (
        <div className="tester-row">
          <span className="tester-label">Begin Offset</span>
          <code className="tester-value">[{action.begin_offset.join(", ")}]</code>
        </div>
      )}
      <div className="tester-row">
        <span className="tester-label">End</span>
        <code className="tester-value">{formatTarget(action.end)}</code>
      </div>
      {action.end_offset && (
        <div className="tester-row">
          <span className="tester-label">End Offset</span>
          <code className="tester-value">[{action.end_offset.join(", ")}]</code>
        </div>
      )}
      <div className="tester-row">
        <span className="tester-label">Duration</span>
        <code className="tester-value">{action.duration ?? 200}ms</code>
      </div>
      <button
        className="tester-run-btn"
        onClick={handleExecute}
        disabled={loading}
      >
        {loading ? "Executing..." : "Execute Swipe"}
      </button>
      {error && <div className="tester-error">{error}</div>}
      {result && <div className="tester-result"><div className="tester-success">{result}</div></div>}
    </div>
  );
}
