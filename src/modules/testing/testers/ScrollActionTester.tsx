import { useState } from "react";
import { testAction, type Action, type ActionTestContext } from "../../../api/pipeline";
import { useLogStore } from "../../observability/stores/useLogStore";

interface Props {
  context: ActionTestContext;
  action: Action & { type: "Scroll" };
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

export default function ScrollActionTester({ action, context }: Props) {
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
      appendLog({ source: "debug", label: "NodeTest", detail: `Scroll: ${msg}`, result: "success" });
    } catch (e) {
      setError(String(e));
      appendLog({ source: "debug", label: "NodeTest", detail: `Scroll: Error: ${e}`, result: "error" });
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="tester-panel">
      <div className="tester-row">
        <span className="tester-label">Target</span>
        <code className="tester-value">{formatTarget(action.target)}</code>
      </div>
      <div className="tester-row">
        <span className="tester-label">DX</span>
        <code className="tester-value">{action.dx ?? 0}</code>
      </div>
      <div className="tester-row">
        <span className="tester-label">DY</span>
        <code className="tester-value">{action.dy ?? 0}</code>
      </div>
      <button
        className="tester-run-btn"
        onClick={handleExecute}
        disabled={loading}
      >
        {loading ? "Executing..." : "Execute Scroll"}
      </button>
      {error && <div className="tester-error">{error}</div>}
      {result && <div className="tester-result"><div className="tester-success">{result}</div></div>}
    </div>
  );
}
