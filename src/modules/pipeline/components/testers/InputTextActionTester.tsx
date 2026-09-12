import { useState } from "react";
import { testAction, type Action } from "../../../../api/pipeline";
import { useLogStore } from "../../../capture/stores/useLogStore";

interface Props {
  action: Action & { type: "InputText" };
}

export default function InputTextActionTester({ action }: Props) {
  const [text, setText] = useState(action.input_text);
  const [result, setResult] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const appendLog = useLogStore((s) => s.appendLog);

  const handleExecute = async () => {
    setLoading(true);
    setResult("");
    setError("");
    try {
      const msg = await testAction({ type: "InputText", input_text: text });
      setResult(msg);
      appendLog({ source: "debug", label: "NodeTest", detail: `InputText: ${msg}`, result: "success" });
    } catch (e) {
      setError(String(e));
      appendLog({ source: "debug", label: "NodeTest", detail: `InputText: Error: ${e}`, result: "error" });
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="tester-panel">
      <div className="tester-row">
        <span className="tester-label">Text</span>
        <input
          type="text"
          className="tester-input"
          value={text}
          onChange={(e) => setText(e.target.value)}
          placeholder="Text to type..."
        />
      </div>
      <button
        className="tester-run-btn"
        onClick={handleExecute}
        disabled={loading}
      >
        {loading ? "Executing..." : "Execute InputText"}
      </button>
      {error && <div className="tester-error">{error}</div>}
      {result && <div className="tester-result"><div className="tester-success">{result}</div></div>}
    </div>
  );
}
