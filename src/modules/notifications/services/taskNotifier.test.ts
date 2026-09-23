import { beforeEach, expect, it, vi } from "vitest";

const notification = vi.hoisted(() => ({
  isPermissionGranted: vi.fn(),
  requestPermission: vi.fn(),
  sendNotification: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-notification", () => notification);

import { notifyTask, resetTaskNotifierForTests } from "./taskNotifier";

beforeEach(() => {
  resetTaskNotifierForTests();
  vi.clearAllMocks();
  notification.isPermissionGranted.mockResolvedValue(true);
});

it("sends each task notification exactly once", async () => {
  const item = { id: "pipeline:run-1", type: "pipeline_completed" as const, title: "Done", body: "Sample completed" };
  expect(await notifyTask(item)).toBe(true);
  expect(await notifyTask(item)).toBe(false);
  expect(notification.sendNotification).toHaveBeenCalledTimes(1);
});

it("deduplicates simultaneous delivery attempts", async () => {
  let release!: (value: boolean) => void;
  notification.isPermissionGranted.mockReturnValue(new Promise((resolve) => { release = resolve; }));
  const item = { id: "pipeline:race", type: "pipeline_completed" as const, title: "Done", body: "Sample completed" };
  const first = notifyTask(item);
  const second = notifyTask(item);
  release(true);
  expect(await Promise.all([first, second])).toEqual([true, false]);
  expect(notification.sendNotification).toHaveBeenCalledTimes(1);
});

it("requests permission and fails quietly when denied", async () => {
  notification.isPermissionGranted.mockResolvedValue(false);
  notification.requestPermission.mockResolvedValue("denied");
  expect(await notifyTask({ id: "one", type: "recording_started", title: "Recorder", body: "Started" })).toBe(false);
  expect(notification.sendNotification).not.toHaveBeenCalled();
});
