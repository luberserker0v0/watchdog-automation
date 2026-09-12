import { useState } from "react";
import { testRecognition, type Recognition, type MatchResult } from "../../../../api/pipeline";
import { useLogStore } from "../../../capture/stores/useLogStore";

interface Props {
  recognition: Recognition & { type: "ColorMatch" };
}

function rgbToHex(r: number, g: number, b: number): string {
  return `#${((1 << 24) + (r << 16) + (g << 8) + b).toString(16).slice(1)}`;
}

function hexToRgb(hex: string): [number, number, number] {
  const h = hex.replace("#", "");
  return [
    parseInt(h.substring(0, 2), 16),
    parseInt(h.substring(2, 4), 16),
    parseInt(h.substring(4, 6), 16),
  ];
}

export default function ColorMatchTester({ recognition }: Props) {
  const [lower, setLower] = useState(recognition.lower);
  const [upper, setUpper] = useState(recognition.upper);
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
        { type: "ColorMatch", lower, upper },
        undefined,
        null,
      );
      setResult(r);
      const detail = r.matched
        ? `ColorMatch: matched (${(r.confidence * 100).toFixed(1)}%)`
        : "ColorMatch: no match";
      appendLog({ source: "debug", label: "NodeTest", detail, result: r.matched ? "success" : "info" });
    } catch (e) {
      setError(String(e));
      appendLog({ source: "debug", label: "NodeTest", detail: `ColorMatch: Error: ${e}`, result: "error" });
    } finally {
      setLoading(false);
    }
  };

  const lowerHex = rgbToHex(lower[0] ?? 0, lower[1] ?? 0, lower[2] ?? 0);
  const upperHex = rgbToHex(upper[0] ?? 255, upper[1] ?? 255, upper[2] ?? 255);

  return (
    <div className="tester-panel">
      <div className="tester-row">
        <span className="tester-label">Lower</span>
        <input
          type="color"
          value={lowerHex}
          onChange={(e) => setLower(hexToRgb(e.target.value))}
        />
        <code className="tester-value">{lowerHex}</code>
      </div>
      <div className="tester-row">
        <span className="tester-label">Upper</span>
        <input
          type="color"
          value={upperHex}
          onChange={(e) => setUpper(hexToRgb(e.target.value))}
        />
        <code className="tester-value">{upperHex}</code>
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
