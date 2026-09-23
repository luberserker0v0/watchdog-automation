import { useState } from "react";
import { testRecognition, type CandidateOrder, type Recognition, type MatchResult } from "../../../api/pipeline";
import { useLogStore } from "../../observability/stores/useLogStore";

interface Props {
  recognition: Recognition & { type: "OCR" };
}

export default function OcrTester({ recognition }: Props) {
  const [expected, setExpected] = useState(recognition.expected ?? "");
  const [useRegex, setUseRegex] = useState(false);
  const [threshold, setThreshold] = useState(recognition.threshold ?? 0.6);
  const [order, setOrder] = useState<CandidateOrder>("native");
  const [candidateIndex, setCandidateIndex] = useState(0);
  const [result, setResult] = useState<MatchResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const appendLog = useLogStore((s) => s.appendLog);

  const handleTest = async () => {
    setLoading(true);
    setResult(null);
    setError("");
    try {
      const pattern = useRegex ? expected : expected;
      const r: MatchResult = await testRecognition(
        { type: "OCR", expected: pattern || undefined, threshold },
        undefined,
        null,
        { order, index: candidateIndex },
      );
      setResult(r);
      const detail = r.matched
        ? `OCR: matched (${(r.confidence * 100).toFixed(1)}%)${r.text ? ` "${r.text}"` : ""}`
        : "OCR: no match";
      appendLog({ source: "debug", label: "NodeTest", detail, result: r.matched ? "success" : "info" });
    } catch (e) {
      setError(String(e));
      appendLog({ source: "debug", label: "NodeTest", detail: `OCR: Error: ${e}`, result: "error" });
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="tester-panel">
      <div className="tester-row">
        <span className="tester-label">Expected Text</span>
        <input
          type="text"
          className="tester-input"
          value={expected}
          onChange={(e) => setExpected(e.target.value)}
          placeholder="Text to match..."
        />
      </div>
      <div className="tester-row">
        <span className="tester-label">Candidate order</span>
        <select className="tester-input" value={order} onChange={(e) => setOrder(e.target.value as CandidateOrder)}>
          <option value="native">Native</option>
          <option value="top_to_bottom">Top to bottom</option>
          <option value="bottom_to_top">Bottom to top</option>
          <option value="left_to_right">Left to right</option>
          <option value="right_to_left">Right to left</option>
          <option value="confidence_desc">Confidence</option>
        </select>
      </div>
      <div className="tester-row">
        <span className="tester-label">Candidate index</span>
        <input className="tester-input" type="number" min={0} step={1} value={candidateIndex} onChange={(e) => setCandidateIndex(Math.max(0, Number(e.target.value) || 0))} />
      </div>
      <div className="tester-row">
        <label className="tester-checkbox">
          <input
            type="checkbox"
            checked={useRegex}
            onChange={(e) => setUseRegex(e.target.checked)}
          />
          Regex pattern
        </label>
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
          {result.text && (
            <div className="tester-result-row">
              <span className="tester-label">Detected</span>
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
          {result.selection_error && <div className="tester-error">{result.selection_error}</div>}
          {result.candidates?.map((candidate) => (
            <div className="tester-result-row" key={`${candidate.rank}-${candidate.region.x}-${candidate.region.y}`}>
              <span className={`tester-badge ${candidate.selected ? "matched" : ""}`}>#{candidate.rank}</span>
              <code className="tester-value">
                {candidate.text ?? "candidate"} · {(candidate.confidence * 100).toFixed(1)}% · ({candidate.region.x}, {candidate.region.y}) {candidate.region.width}×{candidate.region.height}
              </code>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
