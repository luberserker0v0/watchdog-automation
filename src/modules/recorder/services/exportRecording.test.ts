import { beforeEach, expect, it, vi } from "vitest";

const { writeTextFile } = vi.hoisted(() => ({ writeTextFile: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => true }));
vi.mock("@tauri-apps/plugin-fs", () => ({
  BaseDirectory: { Download: 9 },
  writeTextFile,
}));

import { exportRecording } from "./exportRecording";

beforeEach(() => writeTextFile.mockReset().mockResolvedValue(undefined));

it("writes a timestamped recording only through the Downloads base directory", async () => {
  const filename = await exportRecording([
    { id: 1, offset_ms: 0, source: "mouse", action: { type: "Click", target: [10, 20] } },
  ], 1234);
  expect(filename).toBe("watchdog-action-recording-1234.json");
  expect(writeTextFile).toHaveBeenCalledOnce();
  expect(writeTextFile.mock.calls[0][0]).toBe(filename);
  expect(JSON.parse(writeTextFile.mock.calls[0][1])).toMatchObject({
    version: 2,
    actions: [{ id: 1 }],
    pipeline_actions: [{ type: "Click", target: [10, 20] }],
  });
  expect(writeTextFile.mock.calls[0][2]).toEqual({ baseDir: 9 });
});

it("propagates a scoped filesystem rejection", async () => {
  writeTextFile.mockRejectedValueOnce(new Error("forbidden path"));
  await expect(exportRecording([], 10)).rejects.toThrow("forbidden path");
});
