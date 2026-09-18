import { useEffect, useRef, useState } from "react";
import { usePreferencesStore } from "../../../core/stores/usePreferencesStore";
import { useCaptureFrameStore } from "../../stores/useCaptureFrameStore";
import { useDebugStore } from "../../stores/useDebugStore";
import { useOverlayStore } from "../../stores/useOverlayStore";
import { useStreamStore } from "../../stores/useStreamStore";
import { useLogStore } from "../../stores/useLogStore";
import { testRecognition, type CandidateOrder, type Recognition, type MatchResult } from "../../../../api/pipeline";

type RecogType = "DirectHit" | "TemplateMatch" | "OCR" | "ColorMatch";

const RECOG_TYPES: RecogType[] = ["DirectHit", "TemplateMatch", "OCR", "ColorMatch"];

function buildRecognition(type: RecogType, fields: Record<string, string>): Recognition {
  switch (type) {
    case "DirectHit":
      return { type: "DirectHit" };
    case "TemplateMatch":
      return {
        type: "TemplateMatch",
        template: fields.template || "",
        threshold: fields.threshold ? Number(fields.threshold) : undefined,
      };
    case "OCR":
      return {
        type: "OCR",
        expected: fields.expected || undefined,
        threshold: fields.threshold ? Number(fields.threshold) : undefined,
      };
    case "ColorMatch": {
      const lower = fields.lowerRgb
        ? fields.lowerRgb.split(",").map((s) => Number(s.trim()))
        : [0, 0, 0];
      const upper = fields.upperRgb
        ? fields.upperRgb.split(",").map((s) => Number(s.trim()))
        : [255, 255, 255];
      return { type: "ColorMatch", lower, upper };
    }
  }
}

function parseRoi(fields: Record<string, string>): [number, number, number, number] | undefined {
  if (!fields.roi) return [0, 0, 0, 0];
  const parts = fields.roi.split(",").map((s) => Number(s.trim()));
  if (parts.length !== 4 || parts.some(isNaN)) return undefined;
  return [parts[0], parts[1], parts[2], parts[3]];
}

interface RecognitionTabProps {
  isStreaming?: boolean;
  captureBuf?: number[] | null;
  captureMode?: "screen" | "window";
}

export default function RecognitionTab(props: RecognitionTabProps) {
  const storeMode = usePreferencesStore((s) => s.mode);
  const mode = props.captureMode ?? storeMode;
  const storeCaptureBuf = useCaptureFrameStore((s) => s.frames[mode].captureBuf);
  const storeIsStreaming = useStreamStore((s) => s.isStreaming);
  const setRoi = useOverlayStore((s) => s.setRoi);
  const setResultRoi = useOverlayStore((s) => s.setResultRoi);
  const clearRecognition = useOverlayStore((s) => s.clearRecognition);

  const recogType = useDebugStore((s) => s.recogType);
  const fields = useDebugStore((s) => s.recogFields);
  const setRecogType = useDebugStore((s) => s.setRecogType);
  const setRecogField = useDebugStore((s) => s.setRecogField);
  const clearRecogFields = useDebugStore((s) => s.clearRecogFields);

  const isStreaming = props.isStreaming ?? storeIsStreaming;
  const captureBuf = props.captureBuf !== undefined
    ? props.captureBuf
    : storeCaptureBuf;

  const [result, setResult] = useState<MatchResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const appendLog = useLogStore((s) => s.appendLog);
  const executionGeneration = useRef(0);

  useEffect(() => () => {
    executionGeneration.current += 1;
    clearRecognition();
  }, [clearRecognition]);

  const handleTypeChange = (type: RecogType) => {
    setRecogType(type);
    clearRecogFields();
    setResult(null);
    setError(null);
    executionGeneration.current += 1;
    clearRecognition();
  };

  const syncRoiToStore = (value: string) => {
    setRecogField("roi", value);
    executionGeneration.current += 1;
    setResult(null);
    setError(null);
    clearRecognition();
  };

  const handleRun = async () => {
    if (!isStreaming && !captureBuf) {
      setError("請先截圖或開始串流");
      return;
    }
    const generation = ++executionGeneration.current;
    setLoading(true);
    setResult(null);
    setResultRoi(null);
    setError(null);
    try {
      const recognition = buildRecognition(recogType, fields);
      const roi = parseRoi(fields);
      if (roi) setRoi({ x: roi[0], y: roi[1], w: roi[2], h: roi[3] });
      const buf = isStreaming ? null : captureBuf;
      const candidateSelection = recogType === "OCR" || recogType === "TemplateMatch"
        ? {
            order: (fields.candidateOrder || "native") as CandidateOrder,
            index: Math.max(0, Number(fields.candidateIndex) || 0),
          }
        : undefined;
      const res = await testRecognition(recognition, roi, buf, candidateSelection);
      if (generation !== executionGeneration.current) return;
      setResult(res);
      const detail = res.matched
        ? `${recogType}: Yes (${(res.confidence * 100).toFixed(1)}%)${res.text ? ` "${res.text}"` : ""}`
        : `${recogType}: No`;
      appendLog({ source: "debug", label: "辨識", detail, result: res.matched ? "success" : "info" });
      if (res.box_region) {
        setResultRoi({ x: res.box_region.x, y: res.box_region.y, w: res.box_region.width, h: res.box_region.height });
      }
    } catch (e) {
      if (generation !== executionGeneration.current) return;
      setError(String(e));
      appendLog({ source: "debug", label: "辨識", detail: `${recogType}: Error: ${e}`, result: "error" });
    } finally {
      if (generation === executionGeneration.current) setLoading(false);
    }
  };

  const renderFields = () => {
    switch (recogType) {
      case "DirectHit":
        return <div className="recog-hint">DirectHit always succeeds — no config needed.</div>;
      case "TemplateMatch":
        return (
          <>
            <div className="action-field-col">
              <label>Template</label>
              <input type="text" value={fields.template || ""} onChange={(e) => setRecogField("template", e.target.value)} placeholder="e.g. Common/Confirm.png" />
            </div>
            <div className="action-field-row">
              <label>Threshold</label>
              <input type="number" step="0.01" min="0" max="1" value={fields.threshold || "0.8"} onChange={(e) => setRecogField("threshold", e.target.value)} />
            </div>
          </>
        );
      case "OCR":
        return (
          <>
            <div className="action-field-col">
              <label>Expected (regex)</label>
              <input type="text" value={fields.expected || ""} onChange={(e) => setRecogField("expected", e.target.value)} placeholder="Substring: Notifications · Exact: ^Notifications$" />
              <div className="recog-hint">
                Unanchored text searches by substring. Use ^...$ for an exact block match; .* is supported but usually unnecessary.
              </div>
            </div>
            <div className="action-field-row">
              <label>Threshold</label>
              <input type="number" step="0.01" min="0" max="1" value={fields.threshold || "0.6"} onChange={(e) => setRecogField("threshold", e.target.value)} />
            </div>
          </>
        );
      case "ColorMatch":
        return (
          <>
            <div className="action-field-col">
              <label>Lower RGB (r,g,b)</label>
              <input type="text" value={fields.lowerRgb || "0,0,0"} onChange={(e) => setRecogField("lowerRgb", e.target.value)} />
            </div>
            <div className="action-field-col">
              <label>Upper RGB (r,g,b)</label>
              <input type="text" value={fields.upperRgb || "255,255,255"} onChange={(e) => setRecogField("upperRgb", e.target.value)} />
            </div>
          </>
        );
    }
  };

  return (
    <div className="tab-content recognition-tab">
      <div className="action-field-row">
        <label>辨識類型</label>
        <select value={recogType} onChange={(e) => handleTypeChange(e.target.value as RecogType)}>
          {RECOG_TYPES.map((t) => (
            <option key={t} value={t}>{t}</option>
          ))}
        </select>
      </div>
      {renderFields()}
      {(recogType === "OCR" || recogType === "TemplateMatch") && (
        <>
          <div className="action-field-row">
            <label>Candidate order</label>
            <select value={fields.candidateOrder || "native"} onChange={(e) => setRecogField("candidateOrder", e.target.value)}>
              <option value="native">Native</option>
              <option value="top_to_bottom">Top to bottom</option>
              <option value="bottom_to_top">Bottom to top</option>
              <option value="left_to_right">Left to right</option>
              <option value="right_to_left">Right to left</option>
              <option value="confidence_desc">Confidence</option>
            </select>
          </div>
          <div className="action-field-row">
            <label>Candidate index</label>
            <input type="number" min="0" step="1" value={fields.candidateIndex || "0"} onChange={(e) => setRecogField("candidateIndex", e.target.value)} />
          </div>
        </>
      )}
      <div className="action-field-col">
        <label>ROI (x,y,w,h) — 0,0,0,0 means full capture</label>
        <input
          aria-label="Recognition ROI"
          type="text"
          value={fields.roi ?? "0,0,0,0"}
          onChange={(e) => syncRoiToStore(e.target.value)}
          placeholder="0,0,0,0"
        />
      </div>
      <button className="action-execute-btn" onClick={handleRun} disabled={loading || (!isStreaming && !captureBuf)}>
        {loading ? "辨識中..." : "執行辨識"}
      </button>
      {!isStreaming && !captureBuf && <div className="recog-hint">請先截圖或開始串流</div>}
      {error && <div className="action-result error">{error}</div>}
      {result?.selection_error && <div className="action-result error">{result.selection_error}</div>}
      {result && (
        <div className="recog-result">
          <div className="recog-result-row">
            <span className="debug-label">Matched</span>
            <span className={`recog-matched ${result.matched ? "yes" : "no"}`}>
              {result.matched ? "Yes" : "No"}
            </span>
          </div>
          <div className="recog-result-row">
            <span className="debug-label">Confidence</span>
            <span className="debug-value">{(result.confidence * 100).toFixed(1)}%</span>
          </div>
          {result.raw_candidate_count !== undefined && (
            <div className="recog-result-row">
              <span className="debug-label">Candidates</span>
              <span className="debug-value">
                {result.raw_candidate_count} raw → {result.distinct_candidate_count ?? result.candidates.length} distinct
              </span>
            </div>
          )}
          {result.box_region && (
            <div className="recog-result-row">
              <span className="debug-label">Region</span>
              <span className="debug-value">
                ({result.box_region.x}, {result.box_region.y}, {result.box_region.width}, {result.box_region.height})
              </span>
            </div>
          )}
          {result.text && (
            <div className="recog-text-container">
              <span className="debug-label">Detected</span>
              <div className="recog-text">{result.text}</div>
            </div>
          )}
          {result.candidates?.map((candidate) => (
            <div className="recog-result-row" key={`${candidate.rank}-${candidate.region.x}-${candidate.region.y}`}>
              <span className="debug-label">#{candidate.rank}{candidate.selected ? " ✓" : ""}</span>
              <span className="debug-value">
                {candidate.text ?? "candidate"} · {(candidate.confidence * 100).toFixed(1)}% · ({candidate.region.x}, {candidate.region.y}, {candidate.region.width}, {candidate.region.height})
              </span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
