import { memo, useState } from "react";
import type { ParamDef, ParamType } from "../../../api/pipeline";

type ParamValue = string | number | boolean;
interface ParamsFormProps {
  title?: string;
  params: ParamDef[];
  values: Record<string, ParamValue>;
  onChange: (key: string, value: ParamValue) => void;
  disabled?: boolean;
}

const ParamField = memo(function ParamField({
  param,
  value,
  onChange,
}: {
  param: ParamDef;
  value: ParamValue;
  onChange: ParamsFormProps["onChange"];
}) {
  const [passwordVisible, setPasswordVisible] = useState(false);
  let control: React.ReactNode;

  switch (param.type as ParamType) {
    case "text":
      control = <input type="text" className="params-input" value={String(value)} placeholder={param.placeholder} onChange={(event) => onChange(param.key, event.target.value)} />;
      break;
    case "password":
      control = (
        <div className="params-pw-wrap">
          <input
            type={passwordVisible ? "text" : "password"}
            className="params-input"
            value={String(value)}
            placeholder={param.placeholder}
            autoComplete="new-password"
            onChange={(event) => onChange(param.key, event.target.value)}
          />
            <button type="button" className="params-pw-toggle" tabIndex={-1} onClick={() => setPasswordVisible((visible) => !visible)} title={passwordVisible ? "Hide" : "Show"}>
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                {passwordVisible
                  ? <><path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94" /><line x1="1" y1="1" x2="23" y2="23" /></>
                  : <><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" /><circle cx="12" cy="12" r="3" /></>}
              </svg>
            </button>
        </div>
      );
      break;
    case "number":
      control = <input type="number" className="params-input" value={value === "" ? "" : Number(value)} placeholder={param.placeholder} onChange={(event) => onChange(param.key, event.target.value === "" ? "" : Number(event.target.value))} />;
      break;
    case "dropdown":
      control = <select className="params-select" value={String(value)} onChange={(event) => onChange(param.key, event.target.value)}>{param.options?.map((option) => <option key={option} value={option}>{option}</option>)}</select>;
      break;
    case "switch":
      control = <label className="params-switch"><input type="checkbox" checked={Boolean(value)} onChange={(event) => onChange(param.key, event.target.checked)} /><span className="params-switch-slider" /></label>;
      break;
    case "color":
      control = <input type="color" className="params-color" value={String(value)} onChange={(event) => onChange(param.key, event.target.value)} />;
      break;
    default:
      return null;
  }

  return <div className="params-field"><label className="params-label">{param.label}</label><div className="params-input-wrap">{control}</div></div>;
});

export default function ParamsForm({ title, params, values, onChange, disabled }: ParamsFormProps) {
  if (!params.length) return null;
  return (
    <div className="params-form">
      <div className="params-form-title">{title ?? "Parameters"}</div>
      <fieldset className="params-form-fields" disabled={disabled} style={{ border: 0, padding: 0, margin: 0 }}>
        {params.map((param) => <ParamField key={param.key} param={param} value={values[param.key] ?? param.default ?? ""} onChange={onChange} />)}
      </fieldset>
    </div>
  );
}
