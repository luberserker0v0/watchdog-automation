import { useState } from "react";
import { testAction, type Action } from "../../../../api/pipeline";
import { useLogStore } from "../../../capture/stores/useLogStore";

interface Props {
  action: Action & { type: "ClickKey" };
}

const KEY_NAMES: Record<number, string> = {
  13: "Enter", 9: "Tab", 27: "Esc", 32: "Space",
  8: "Backspace", 46: "Delete", 37: "←", 38: "↑", 39: "→", 40: "↓",
  112: "F1", 113: "F2", 114: "F3", 115: "F4",
  116: "F5", 117: "F6", 118: "F7", 119: "F8",
  120: "F9", 121: "F10", 122: "F11", 123: "F12",
};

function formatKeys(keys: number[]): string {
  return keys.map((k) => KEY_NAMES[k] ?? `0x${k.toString(16).toUpperCase()}`).join(" + ");
}

export default function ClickKeyActionTester({ action }: Props) {
  const [result, setResult] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const appendLog = useLogStore((s) => s.appendLog);

  const handleExecute = async () => {
    setLoading(true);
    setResult("");
    setError("");
    try {
      const msg = await testAction({ type: "ClickKey", key: action.key });
      setResult(msg);
      appendLog({ source: "debug", label: "NodeTest", detail: `ClickKey: ${msg}`, result: "success" });
    } catch (e) {
      setError(String(e));
      appendLog({ source: "debug", label: "NodeTest", detail: `ClickKey: Error: ${e}`, result: "error" });
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="tester-panel">
      <div className="tester-row">
        <span className="tester-label">Keys</span>
        <code className="tester-value">{formatKeys(action.key)}</code>
      </div>
      <button
        className="tester-run-btn"
        onClick={handleExecute}
        disabled={loading}
      >
        {loading ? "Executing..." : "Execute ClickKey"}
      </button>
      {error && <div className="tester-error">{error}</div>}
      {result && <div className="tester-result"><div className="tester-success">{result}</div></div>}
    </div>
  );
}
