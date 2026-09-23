import { useState } from "react";
import { testRecognition, type Recognition, type MatchResult } from "../../../api/pipeline";
import { useLogStore } from "../../observability/stores/useLogStore";

interface Props {
  recognition: Recognition & { type: "Custom" };
}

export default function CustomRecognizerTester({ recognition }: Props) {
  const [paramStr, setParamStr] = useState(
    JSON.stringify(recognition.param ?? {}, null, 2)
  );
  const [result, setResult] = useState<MatchResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const appendLog = useLogStore((s) => s.appendLog);

  const handleTest = async () => {
    setLoading(true);
    setResult(null);
    setError("");
    try {
      let param: unknown;
      try { param = JSON.parse(paramStr); } catch { param = paramStr; }
      const r: MatchResult = await testRecognition(
        { type: "Custom", recognition: recognition.recognition, param },
        undefined,
        null,
      );
      setResult(r);
      const detail = r.matched
        ? `Custom(${recognition.recognition}): matched (${(r.confidence * 100).toFixed(1)}%)`
        : `Custom(${recognition.recognition}): no match`;
      appendLog({ source: "debug", label: "NodeTest", detail, result: r.matched ? "success" : "info" });
    } catch (e) {
      setError(String(e));
      appendLog({ source: "debug", label: "NodeTest", detail: `Custom: Error: ${e}`, result: "error" });
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="tester-panel">
      <div className="tester-row">
        <span className="tester-label">Recognizer</span>
        <code className="tester-value">{recognition.recognition}</code>
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
        onClick={handleTest}
        disabled={loading}
      >
        {loading ? "Testing..." : "Capture & Test"}
      </button>
      {error && <div className="tester-error">{error}</div>}
      {result && (
        <div className="tester-result">
          <div className="tester-result-row">
            <span className={`tester-badge ${result.matched ? "matched" : "unmatched"}`}>
              {result.matched ? "MATCHED" : "NO MATCH"}
            </span>
            <span className="tester-confidence">
              {result.confidence.toFixed(3)}
            </span>
          </div>
          {result.text && (
            <div className="tester-result-row">
              <span className="tester-label">Text</span>
              <code className="tester-value">{result.text}</code>
            </div>
          )}
          {result.box_region && (
            <div className="tester-result-row">
              <span className="tester-label">Region</span>
              <code className="tester-value">
                ({result.box_region.x}, {result.box_region.y}) {result.box_region.width}×{result.box_region.height}
              </code>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
