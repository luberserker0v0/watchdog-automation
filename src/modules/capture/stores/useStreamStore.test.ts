import { beforeEach, describe, expect, it, vi } from "vitest";

const streamApi = vi.hoisted(() => ({
  getStreamStatus: vi.fn(),
  startStream: vi.fn(),
  stopStream: vi.fn(),
}));
const eventApi = vi.hoisted(() => ({
  listeners: new Map<string, (event: { payload: any }) => void>(),
  listen: vi.fn(async (name: string, callback: (event: { payload: any }) => void) => {
    eventApi.listeners.set(name, callback);
    return () => eventApi.listeners.delete(name);
  }),
}));
vi.mock("../../../api/stream", () => streamApi);
vi.mock("@tauri-apps/api/event", () => ({ listen: eventApi.listen }));

import { useCaptureFrameStore } from "./useCaptureFrameStore";
import { useStreamStore } from "./useStreamStore";
import { usePreferencesStore } from "../../core/stores/usePreferencesStore";

describe("stream lifecycle", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    eventApi.listeners.clear();
    useStreamStore.setState({
      isStreaming: true,
      actualFps: 5,
      startTime: Date.now() - 1_000,
      elapsedTime: "00:01",
      frameSize: null,
      clientSize: null,
    });
    useCaptureFrameStore.getState().setStreamFrameUrl("window", "/frame?t=1");
  });

  it("clears streaming state and the stale frame when the backend target closes", () => {
    useStreamStore.getState().handleBackendStopped("window");

    expect(useStreamStore.getState()).toMatchObject({
      isStreaming: false,
      actualFps: 0,
      startTime: null,
      elapsedTime: "00:00",
    });
    expect(useCaptureFrameStore.getState().frames.window.previewUrl).toBeNull();
  });

  it("synchronizes a backend-started pipeline stream and listens for its frames", async () => {
    streamApi.getStreamStatus.mockResolvedValue({
      is_streaming: true,
      fps: 5,
      target_type: "window",
      target_index: 42,
      start_time: 100,
      engine: "gdi",
      capture_area: "client",
      owner: "pipeline",
      owner_id: "run-1",
      session_id: 7,
      frame_width: 1280,
      frame_height: 720,
      client_width: 1280,
      client_height: 720,
    });

    await useStreamStore.getState().refreshStatus();

    expect(useStreamStore.getState().isStreaming).toBe(true);
    expect(useStreamStore.getState().frameSize).toEqual([1280, 720]);
    expect(useStreamStore.getState().clientSize).toEqual([1280, 720]);
    expect(usePreferencesStore.getState()).toMatchObject({ mode: "window", windowCaptureArea: "client" });
    eventApi.listeners.get("stream-frame")?.({
      payload: { session_id: 6, sequence: 1, target_type: "window", target_index: 42, capture_area: "client", frame_width: 640, frame_height: 360, client_width: 640, client_height: 360 },
    });
    expect(useCaptureFrameStore.getState().frames.window.previewUrl).toBe("/frame?t=1");
    expect(useStreamStore.getState().frameSize).toEqual([1280, 720]);
    eventApi.listeners.get("stream-frame")?.({
      payload: { session_id: 7, sequence: 2, target_type: "window", target_index: 42, capture_area: "client", frame_width: 1279, frame_height: 719, client_width: 1280, client_height: 720 },
    });
    expect(useCaptureFrameStore.getState().frames.window.previewUrl).toContain("watchdog.localhost/frame");
    expect(useStreamStore.getState().frameSize).toEqual([1279, 719]);
    expect(useStreamStore.getState().clientSize).toEqual([1280, 720]);
    eventApi.listeners.get("stream-stopped")?.({
      payload: { reason: "replaced", target_type: "window", target_index: 42, session_id: 6 },
    });
    expect(useStreamStore.getState().isStreaming).toBe(true);
    eventApi.listeners.get("stream-stopped")?.({
      payload: { reason: "pipeline_finished", target_type: "window", target_index: 42, session_id: 7 },
    });
    expect(useStreamStore.getState().isStreaming).toBe(false);
    expect(useStreamStore.getState().frameSize).toBeNull();
    expect(useStreamStore.getState().clientSize).toBeNull();
    expect(useCaptureFrameStore.getState().frames.window.previewUrl).toBeNull();
  });
});
