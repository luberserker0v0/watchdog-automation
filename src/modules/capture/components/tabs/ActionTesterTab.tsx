import { useState } from "react";
import { useDebugStore } from "../../stores/useDebugStore";
import { useLogStore } from "../../stores/useLogStore";
import {
  testAction,
  type Action,
  type ActionTestContext,
} from "../../../../api/pipeline";

type ActionType = "Click" | "LongPress" | "Swipe" | "Scroll" | "InputText" | "ClickKey";

const ACTION_TYPES: ActionType[] = ["Click", "LongPress", "Swipe", "Scroll", "InputText", "ClickKey"];

const KEYS: { label: string; code: number }[] = [
  { label: "Space", code: 0x20 },
  { label: "Enter", code: 0x0D },
  { label: "Escape", code: 0x1B },
  { label: "Tab", code: 0x09 },
  { label: "Backspace", code: 0x08 },
  { label: "Delete", code: 0x2E },
  { label: "ArrowUp", code: 0x26 },
  { label: "ArrowDown", code: 0x28 },
  { label: "ArrowLeft", code: 0x25 },
  { label: "ArrowRight", code: 0x27 },
  { label: "F1", code: 0x70 },
  { label: "F2", code: 0x71 },
  { label: "F3", code: 0x72 },
  { label: "F4", code: 0x73 },
  { label: "F5", code: 0x74 },
  { label: "F6", code: 0x75 },
  { label: "F7", code: 0x76 },
  { label: "F8", code: 0x77 },
  { label: "F9", code: 0x78 },
  { label: "F10", code: 0x79 },
  { label: "F11", code: 0x7A },
  { label: "F12", code: 0x7B },
  { label: "A", code: 0x41 },
  { label: "B", code: 0x42 },
  { label: "C", code: 0x43 },
  { label: "D", code: 0x44 },
  { label: "E", code: 0x45 },
  { label: "F", code: 0x46 },
  { label: "G", code: 0x47 },
  { label: "H", code: 0x48 },
  { label: "I", code: 0x49 },
  { label: "J", code: 0x4A },
  { label: "K", code: 0x4B },
  { label: "L", code: 0x4C },
  { label: "M", code: 0x4D },
  { label: "N", code: 0x4E },
  { label: "O", code: 0x4F },
  { label: "P", code: 0x50 },
  { label: "Q", code: 0x51 },
  { label: "R", code: 0x52 },
  { label: "S", code: 0x53 },
  { label: "T", code: 0x54 },
  { label: "U", code: 0x55 },
  { label: "V", code: 0x56 },
  { label: "W", code: 0x57 },
  { label: "X", code: 0x58 },
  { label: "Y", code: 0x59 },
  { label: "Z", code: 0x5A },
  { label: "0", code: 0x30 },
  { label: "1", code: 0x31 },
  { label: "2", code: 0x32 },
  { label: "3", code: 0x33 },
  { label: "4", code: 0x34 },
  { label: "5", code: 0x35 },
  { label: "6", code: 0x36 },
  { label: "7", code: 0x37 },
  { label: "8", code: 0x38 },
  { label: "9", code: 0x39 },
];

function buildAction(type: ActionType, fields: Record<string, string>): Action {
  const toNum = (v: string | undefined, fallback = 0) =>
    v === "" || v === undefined ? fallback : Number(v);

  switch (type) {
    case "Click":
      return {
        type: "Click",
        target: [toNum(fields.x), toNum(fields.y)],
        target_offset: fields.ox || fields.oy
          ? [toNum(fields.ox), toNum(fields.oy), 0, 0]
          : undefined,
      };
    case "LongPress":
      return {
        type: "LongPress",
        target: [toNum(fields.x), toNum(fields.y)],
        duration: toNum(fields.duration, 1000),
        target_offset: fields.ox || fields.oy
          ? [toNum(fields.ox), toNum(fields.oy), 0, 0]
          : undefined,
      };
    case "Swipe":
      return {
        type: "Swipe",
        begin: [toNum(fields.beginX), toNum(fields.beginY)],
        end: [toNum(fields.endX), toNum(fields.endY)],
        duration: toNum(fields.duration, 200),
      };
    case "Scroll":
      return {
        type: "Scroll",
        target: [toNum(fields.x), toNum(fields.y)],
        dx: toNum(fields.dx),
        dy: toNum(fields.dy),
      };
    case "InputText":
      return { type: "InputText", input_text: fields.text || "" };
    case "ClickKey": {
      const keyCodes = fields.key
        ? fields.key.split(",").map((k) => toNum(k.trim()))
        : [];
      return { type: "ClickKey", key: keyCodes };
    }
  }
}

interface ActionTesterTabProps {
  context?: ActionTestContext;
}

export default function ActionTesterTab({ context }: ActionTesterTabProps) {
  const actionType = useDebugStore((s) => s.actionType);
  const fields = useDebugStore((s) => s.actionFields);
  const setActionType = useDebugStore((s) => s.setActionType);
  const setActionField = useDebugStore((s) => s.setActionField);
  const clearActionFields = useDebugStore((s) => s.clearActionFields);

  const [result, setResult] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const appendLog = useLogStore((s) => s.appendLog);

  const handleTypeChange = (type: ActionType) => {
    setActionType(type);
    clearActionFields();
    setResult(null);
  };

  const handleExecute = async () => {
    setLoading(true);
    setResult(null);
    try {
      const action = buildAction(actionType, fields);
      const msg = await testAction(action, context);
      setResult(msg);
      appendLog({ source: "debug", label: "動作", detail: `${actionType}: ${msg}`, result: msg.startsWith("Error") ? "error" : "success" });
    } catch (e) {
      setResult(`Error: ${e}`);
      appendLog({ source: "debug", label: "動作", detail: `${actionType}: Error: ${e}`, result: "error" });
    } finally {
      setLoading(false);
    }
  };

  const positionalAction = actionType === "Click"
    || actionType === "LongPress"
    || actionType === "Swipe"
    || actionType === "Scroll";
  const coordinateSpace = !positionalAction
    ? "Not used by this action"
    : context?.type === "screen"
      ? `Screen ${context.index}`
      : context?.type === "window"
        ? `Window ${context.capture_area}`
        : "Capture target unavailable";

  const renderFields = () => {
    switch (actionType) {
      case "Click":
        return (
          <>
            <div className="action-field-row">
              <label>X</label>
              <input type="number" value={fields.x || ""} onChange={(e) => setActionField("x", e.target.value)} />
              <label>Y</label>
              <input type="number" value={fields.y || ""} onChange={(e) => setActionField("y", e.target.value)} />
            </div>
            <div className="action-field-row">
              <label>Offset X</label>
              <input type="number" value={fields.ox || ""} onChange={(e) => setActionField("ox", e.target.value)} />
              <label>Offset Y</label>
              <input type="number" value={fields.oy || ""} onChange={(e) => setActionField("oy", e.target.value)} />
            </div>
          </>
        );
      case "LongPress":
        return (
          <>
            <div className="action-field-row">
              <label>X</label>
              <input type="number" value={fields.x || ""} onChange={(e) => setActionField("x", e.target.value)} />
              <label>Y</label>
              <input type="number" value={fields.y || ""} onChange={(e) => setActionField("y", e.target.value)} />
            </div>
            <div className="action-field-row">
              <label>Duration (ms)</label>
              <input type="number" value={fields.duration || "1000"} onChange={(e) => setActionField("duration", e.target.value)} />
            </div>
          </>
        );
      case "Swipe":
        return (
          <>
            <div className="action-field-row">
              <label>Begin X</label>
              <input type="number" value={fields.beginX || ""} onChange={(e) => setActionField("beginX", e.target.value)} />
              <label>Begin Y</label>
              <input type="number" value={fields.beginY || ""} onChange={(e) => setActionField("beginY", e.target.value)} />
            </div>
            <div className="action-field-row">
              <label>End X</label>
              <input type="number" value={fields.endX || ""} onChange={(e) => setActionField("endX", e.target.value)} />
              <label>End Y</label>
              <input type="number" value={fields.endY || ""} onChange={(e) => setActionField("endY", e.target.value)} />
            </div>
            <div className="action-field-row">
              <label>Duration (ms)</label>
              <input type="number" value={fields.duration || "200"} onChange={(e) => setActionField("duration", e.target.value)} />
            </div>
          </>
        );
      case "Scroll":
        return (
          <>
            <div className="action-field-row">
              <label>X</label>
              <input type="number" value={fields.x || ""} onChange={(e) => setActionField("x", e.target.value)} />
              <label>Y</label>
              <input type="number" value={fields.y || ""} onChange={(e) => setActionField("y", e.target.value)} />
            </div>
            <div className="action-field-row">
              <label>dX</label>
              <input type="number" value={fields.dx || "0"} onChange={(e) => setActionField("dx", e.target.value)} />
              <label>dY</label>
              <input type="number" value={fields.dy || "0"} onChange={(e) => setActionField("dy", e.target.value)} />
            </div>
          </>
        );
      case "InputText":
        return (
          <div className="action-field-row">
            <label>Text</label>
            <input type="text" value={fields.text || ""} onChange={(e) => setActionField("text", e.target.value)} className="action-text-input" />
          </div>
        );
      case "ClickKey":
        return (
          <div className="action-field-col">
            <label>Key</label>
            <select value={fields.key || ""} onChange={(e) => setActionField("key", e.target.value)}>
              <option value="">Select key...</option>
              {KEYS.map((k) => (
                <option key={k.code} value={String(k.code)}>
                  {k.label}
                </option>
              ))}
            </select>
          </div>
        );
    }
  };

  return (
    <div className="tab-content action-tester-tab">
      <div className="action-field-row">
        <label>動作類型</label>
        <select value={actionType} onChange={(e) => handleTypeChange(e.target.value as ActionType)}>
          {ACTION_TYPES.map((t) => (
            <option key={t} value={t}>{t}</option>
          ))}
        </select>
      </div>
      <div className="action-field-row">
        <label>Coordinate space</label>
        <span>{coordinateSpace}</span>
      </div>
      {renderFields()}
      <button
        className="action-execute-btn"
        onClick={handleExecute}
        disabled={loading || (positionalAction && !context)}
        title={positionalAction && !context ? "Capture a target image before testing this action" : undefined}
      >
        {loading ? "執行中..." : "執行"}
      </button>
      {result && (
        <div className={`action-result ${result.startsWith("Error") ? "error" : "success"}`}>
          {result}
        </div>
      )}
    </div>
  );
}
