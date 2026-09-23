import { useState } from "react";
import { testAction, type Action, type ActionTestContext } from "../../../api/pipeline";
import { useLogStore } from "../../observability/stores/useLogStore";

export default function WaitActionTester({ action, context }: {
  action: Action & { type: "Wait" };
  context: ActionTestContext;
}) {
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState("");
  const [error, setError] = useState("");
  const appendLog = useLogStore((state) => state.appendLog);

  const execute = async () => {
    setLoading(true);
    setResult("");
    setError("");
    try {
      const message = await testAction(action, context);
      setResult(message);
      appendLog({ source: "debug", label: "NodeTest", detail: `Wait: ${message}`, result: "success" });
    } catch (caught) {
      const message = String(caught);
      setError(message);
      appendLog({ source: "debug", label: "NodeTest", detail: `Wait: Error: ${message}`, result: "error" });
    } finally {
      setLoading(false);
    }
  };

  return <div className="tester-panel">
    <div className="tester-row"><span className="tester-label">Duration</span><code className="tester-value">{action.duration} ms</code></div>
    <button className="tester-run-btn" onClick={() => void execute()} disabled={loading}>{loading ? "Waiting..." : "Test Wait"}</button>
    {error && <div className="tester-error">{error}</div>}
    {result && <div className="tester-result"><div className="tester-success">{result}</div></div>}
  </div>;
}
