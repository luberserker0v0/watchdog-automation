import { useState } from "react";
import { testAction, type Action, type ActionTestContext } from "../../../api/pipeline";

interface Props {
  context: ActionTestContext;
  action: Action & { type: "Custom" };
}

export default function CustomActionTester({ action, context }: Props) {
  const [paramStr, setParamStr] = useState(
    JSON.stringify(action.param ?? {}, null, 2)
  );
  const [result, setResult] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  const handleExecute = async () => {
    setLoading(true);
    setResult("");
    setError("");
    try {
      let param: unknown;
      try { param = JSON.parse(paramStr); } catch { param = paramStr; }
      const msg = await testAction({ type: "Custom", action: action.action, param }, context);
      setResult(msg);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="tester-panel">
      <div className="tester-row">
        <span className="tester-label">Action</span>
        <code className="tester-value">{action.action}</code>
      </div>
      <div className="tester-row">
        <span className="tester-label">Params</span>
        <textarea
          className="tester-textarea"
          value={paramStr}
          onChange={(e) => setParamStr(e.target.value)}
          rows={4}
        />
      </div>
      <button
        className="tester-run-btn"
        onClick={handleExecute}
        disabled={loading}
      >
        {loading ? "Executing..." : "Execute Custom"}
      </button>
      {error && <div className="tester-error">{error}</div>}
      {result && <div className="tester-result"><div className="tester-success">{result}</div></div>}
    </div>
  );
}
