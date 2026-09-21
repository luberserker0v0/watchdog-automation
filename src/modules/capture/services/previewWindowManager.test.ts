import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { usePreferencesStore } from "../../shell/stores/usePreferencesStore";
import { useUiStore } from "../../shell/stores/useUiStore";

const mocks = vi.hoisted(() => {
  const eventHandlers = new Map<string, (event: { payload: unknown }) => void>();
  const resizedHandlers: Array<(event: { payload: { width: number; height: number } }) => void> = [];
  const windows: MockWindow[] = [];
  const onceHandlers = new Map<MockWindow, Map<string, (event: unknown) => void>>();
  const control = { failNextRegistration: false };

  class MockWindow {
    static getByLabel = vi.fn(async () => null as MockWindow | null);
    label: string;
    options: Record<string, unknown>;
    emit = vi.fn().mockResolvedValue(undefined);
    show = vi.fn().mockResolvedValue(undefined);
    setFocus = vi.fn().mockResolvedValue(undefined);
    scaleFactor = vi.fn().mockResolvedValue(1.5);
    once = vi.fn(async (name: string, handler: (event: unknown) => void) => {
      if (control.failNextRegistration) {
        control.failNextRegistration = false;
        throw new Error("registration failed");
      }
      const handlers = onceHandlers.get(this) ?? new Map();
      handlers.set(name, handler);
      onceHandlers.set(this, handlers);
      return vi.fn(() => handlers.delete(name));
    });
    onResized = vi.fn(async (handler: (event: { payload: { width: number; height: number } }) => void) => {
      resizedHandlers.push(handler);
      return vi.fn();
    });

    constructor(label: string, options: Record<string, unknown>) {
      this.label = label;
      this.options = options;
      windows.push(this);
    }
  }

  return { eventHandlers, resizedHandlers, windows, onceHandlers, control, MockWindow };
});

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (name: string, handler: (event: { payload: unknown }) => void) => {
    mocks.eventHandlers.set(name, handler);
    return vi.fn(() => mocks.eventHandlers.delete(name));
  }),
}));

vi.mock("@tauri-apps/api/webviewWindow", () => ({ WebviewWindow: mocks.MockWindow }));

import { logicalPreviewSize, openOrFocusPreview } from "./previewWindowManager";

beforeEach(() => {
  vi.useFakeTimers();
  mocks.windows.length = 0;
  mocks.resizedHandlers.length = 0;
  mocks.eventHandlers.clear();
  mocks.onceHandlers.clear();
  mocks.control.failNextRegistration = false;
  mocks.MockWindow.getByLabel.mockResolvedValue(null);
  usePreferencesStore.setState({ mode: "screen", previewWindowSize: [1000, 700] });
  useUiStore.setState({ previewOpen: false, capturePanelOpen: true });
});

afterEach(() => vi.useRealTimers());

it("converts physical resize events to bounded logical dimensions", () => {
  expect(logicalPreviewSize({ width: 1800, height: 1200 }, 1.5)).toEqual([1200, 800]);
  expect(logicalPreviewSize({ width: 200, height: 100 }, 2)).toEqual([600, 400]);
});

it("creates one stable Preview, focuses repeats, and persists logical resize", async () => {
  await openOrFocusPreview();
  expect(mocks.windows).toHaveLength(1);
  expect(mocks.windows[0].label).toBe("capture-preview");
  expect(mocks.windows[0].options).toMatchObject({ width: 1000, height: 700, minWidth: 600, minHeight: 400, center: true });
  expect(useUiStore.getState()).toMatchObject({ previewOpen: true, capturePanelOpen: false });

  await openOrFocusPreview();
  expect(mocks.windows).toHaveLength(1);
  expect(mocks.windows[0].show).toHaveBeenCalledOnce();
  expect(mocks.windows[0].setFocus).toHaveBeenCalledOnce();

  mocks.resizedHandlers[0]({ payload: { width: 1800, height: 1200 } });
  await vi.advanceTimersByTimeAsync(181);
  expect(usePreferencesStore.getState().previewWindowSize).toEqual([1200, 800]);

  mocks.onceHandlers.get(mocks.windows[0])?.get("tauri://destroyed")?.({});
  expect(useUiStore.getState().previewOpen).toBe(false);
  expect(mocks.eventHandlers.has("preview-ready")).toBe(false);

  await openOrFocusPreview();
  expect(mocks.windows).toHaveLength(2);
  mocks.onceHandlers.get(mocks.windows[1])?.get("tauri://destroyed")?.({});
});

it("recovers cleanly when Preview listener registration fails", async () => {
  mocks.control.failNextRegistration = true;
  await expect(openOrFocusPreview()).rejects.toThrow("registration failed");
  expect(useUiStore.getState().previewOpen).toBe(false);
  expect(mocks.eventHandlers.has("preview-ready")).toBe(false);

  await expect(openOrFocusPreview()).resolves.toBeUndefined();
  expect(mocks.windows).toHaveLength(2);
});
