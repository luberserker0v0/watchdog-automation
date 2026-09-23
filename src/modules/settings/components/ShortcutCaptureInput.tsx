import { useEffect, useState } from "react";
import { releaseRecorderHotkeys } from "../../recorder/services/recorderHotkeys";
import { useRecorderStore } from "../../recorder/stores/useRecorderStore";
import { useTranslation } from "react-i18next";

interface ShortcutCaptureInputProps {
  id: string;
  label: string;
  value: string;
  disabled?: boolean;
  onCapture: (shortcut: string) => string | null;
}

const MODIFIER_KEYS = new Set(["Control", "Shift", "Alt", "Meta"]);

export function formatShortcut(event: Pick<KeyboardEvent, "key" | "code" | "ctrlKey" | "metaKey" | "altKey" | "shiftKey">): string | null {
  if (MODIFIER_KEYS.has(event.key)) return null;
  let key: string | null = null;
  if (/^Key[A-Z]$/.test(event.code)) key = event.code.slice(3);
  else if (/^Digit[0-9]$/.test(event.code)) key = event.code.slice(5);
  else if (/^F(?:[1-9]|1[0-9]|2[0-4])$/.test(event.code)) key = event.code;
  else if (/^Numpad[0-9]$/.test(event.code)) key = event.code;
  else key = ({
    Space: "Space", Enter: "Enter", Tab: "Tab", Backspace: "Backspace",
    Delete: "Delete", Insert: "Insert", Home: "Home", End: "End",
    PageUp: "PageUp", PageDown: "PageDown", ArrowUp: "ArrowUp",
    ArrowDown: "ArrowDown", ArrowLeft: "ArrowLeft", ArrowRight: "ArrowRight",
  } as Record<string, string>)[event.code] ?? null;
  if (!key) return null;
  const modifiers: string[] = [];
  if (event.ctrlKey) modifiers.push("CommandOrControl");
  if (event.metaKey) modifiers.push("Super");
  if (event.altKey) modifiers.push("Alt");
  if (event.shiftKey) modifiers.push("Shift");
  if (!modifiers.length && !/^F\d+$/.test(key)) return null;
  return [...modifiers, key].join("+");
}

export default function ShortcutCaptureInput({ id, label, value, disabled = false, onCapture }: ShortcutCaptureInputProps) {
  const { t } = useTranslation();
  const [capturing, setCapturing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const setShortcutCaptureActive = useRecorderStore((state) => state.setShortcutCaptureActive);

  const finish = () => {
    setCapturing(false);
    setShortcutCaptureActive(false);
  };

  useEffect(() => {
    if (!capturing) return;
    const onKeyDown = (event: KeyboardEvent) => {
      event.preventDefault();
      event.stopPropagation();
      if (event.repeat || MODIFIER_KEYS.has(event.key)) return;
      if (event.key === "Escape") {
        setError(null);
        finish();
        return;
      }
      const shortcut = formatShortcut(event);
      if (!shortcut) {
        setError(t("settings.shortcutInvalid"));
        return;
      }
      const validation = onCapture(shortcut);
      if (validation) {
        setError(validation);
        return;
      }
      setError(null);
      finish();
    };
    window.addEventListener("keydown", onKeyDown, true);
    return () => window.removeEventListener("keydown", onKeyDown, true);
  }, [capturing, onCapture, setShortcutCaptureActive, t]);

  useEffect(() => () => setShortcutCaptureActive(false), [setShortcutCaptureActive]);

  return (
    <div className="shortcut-capture">
      <input id={id} className={`settings-item-control shortcut-capture-value ${capturing ? "capturing" : ""}`}
        aria-label={label} readOnly value={capturing ? t("settings.shortcutPrompt") : value} />
      <button className="settings-action" type="button" disabled={disabled || capturing}
        aria-label={t("settings.shortcutRecordLabel", { label })}
        onClick={async () => {
          setError(null);
          setShortcutCaptureActive(true);
          await releaseRecorderHotkeys();
          setCapturing(true);
        }}>
        {capturing ? t("settings.shortcutListening") : t("settings.shortcutRecord")}
      </button>
      {error && <span className="settings-item-error" role="alert">{error}</span>}
    </div>
  );
}
