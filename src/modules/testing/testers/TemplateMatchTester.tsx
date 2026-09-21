import { useState } from "react";
import { testRecognition, type Recognition, type MatchResult } from "../../../api/pipeline";
import { useLogStore } from "../../capture/stores/useLogStore";

interface Props {
  recognition: Recognition & { type: "TemplateMatch" };
}

export default function TemplateMatchTester({ recognition }: Props) {
  const [threshold, setThreshold] = useState(recognition.threshold ?? 0.8);
  const [result, setResult] = useState<MatchResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const appendLog = useLogStore((s) => s.appendLog);

  const handleTest = async () => {
    setLoading(true);
    setResult(null);
    setError("");
    try {
      const r: MatchResult = await testRecognition(
        { type: "TemplateMatch", template: recognition.template, threshold },
        undefined,
        null,
      );
      setResult(r);
      const detail = r.matched
        ? `TemplateMatch: matched (${(r.confidence * 100).toFixed(1)}%)`
        : "TemplateMatch: no match";
      appendLog({ source: "debug", label: "NodeTest", detail, result: r.matched ? "success" : "info" });
    } catch (e) {
      setError(String(e));
      appendLog({ source: "debug", label: "NodeTest", detail: `TemplateMatch: Error: ${e}`, result: "error" });
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="tester-panel">
      <div className="tester-row">
        <span className="tester-label">Template</span>
        <code className="tester-value">{recognition.template}</code>
      </div>
      <div className="tester-row">
        <span className="tester-label">Threshold</span>
        <input
          type="range"
          min={0}
          max={1}
          step={0.05}
          value={threshold}
          onChange={(e) => setThreshold(parseFloat(e.target.value))}
        />
        <span className="tester-value">{threshold.toFixed(2)}</span>
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
          {result.raw_candidate_count !== undefined && (
            <div className="tester-result-row">
              <span className="tester-label">Candidates</span>
              <span className="tester-value">
                {result.raw_candidate_count} raw → {result.distinct_candidate_count ?? result.candidates.length} distinct
              </span>
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
