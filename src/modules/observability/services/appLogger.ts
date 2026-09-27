import { isTauri } from "@tauri-apps/api/core";
import { error as nativeError, info as nativeInfo, warn as nativeWarn } from "@tauri-apps/plugin-log";

function redactSecrets(value: string): string {
  return value
    .replace(/(authorization\s*[:=]\s*bearer\s+)[^\s,;}]+/gi, "$1[REDACTED]")
    .replace(/((?:llm[_-]?)?api[_-]?key["']?\s*[:=]\s*["']?)[^\s,"'};]+/gi, "$1[REDACTED]");
}

function text(message: string, detail?: unknown): string {
  if (detail === undefined) return redactSecrets(message);
  if (detail instanceof Error) return redactSecrets(`${message} ${detail.message}`);
  return redactSecrets(`${message} ${String(detail)}`);
}

function dispatch(
  native: (message: string) => Promise<void>,
  fallback: (...data: unknown[]) => void,
  message: string,
  detail?: unknown,
) {
  const rendered = text(message, detail);
  if (isTauri()) void native(rendered).catch(() => fallback(rendered));
  else fallback(rendered);
}

export const appLogger = {
  info: (message: string, detail?: unknown) => dispatch(nativeInfo, console.info, message, detail),
  warn: (message: string, detail?: unknown) => dispatch(nativeWarn, console.warn, message, detail),
  error: (message: string, detail?: unknown) => dispatch(nativeError, console.error, message, detail),
};
