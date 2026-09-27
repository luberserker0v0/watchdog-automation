import { beforeEach, expect, it, vi } from "vitest";

const { nativeInfo } = vi.hoisted(() => ({ nativeInfo: vi.fn(() => Promise.resolve()) }));
vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => true }));
vi.mock("@tauri-apps/plugin-log", () => ({
  info: nativeInfo,
  warn: vi.fn(() => Promise.resolve()),
  error: vi.fn(() => Promise.resolve()),
}));

beforeEach(() => {
  nativeInfo.mockClear();
  vi.resetModules();
});

it("forwards frontend diagnostics to the native Log plugin", async () => {
  const { appLogger } = await import("./appLogger");
  appLogger.info("Capture started", 3);
  expect(nativeInfo).toHaveBeenCalledWith("Capture started 3");
});

it("redacts API credentials before native logging", async () => {
  const { appLogger } = await import("./appLogger");
  appLogger.info("request Authorization: Bearer private-token llmApiKey=other-secret");
  expect(nativeInfo).toHaveBeenCalledWith(
    "request Authorization: Bearer [REDACTED] llmApiKey=[REDACTED]",
  );
});
