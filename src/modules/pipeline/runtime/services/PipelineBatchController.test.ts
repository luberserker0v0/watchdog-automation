import { expect, it, vi } from "vitest";
import { PipelineBatchController, type PatrolReportEvent } from "./PipelineBatchController";

const listeners = vi.hoisted(() => new Map<string, (event: { payload: unknown }) => void>());
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (name: string, callback: (event: { payload: unknown }) => void) => {
    listeners.set(name, callback);
    return () => listeners.delete(name);
  }),
}));

it("delivers patrol reports independently of pipeline status", async () => {
  const controller = new PipelineBatchController();
  const onStatus = vi.fn();
  const onPatrol = vi.fn();
  expect(await controller.start(onStatus, onPatrol)).toBe(true);

  const report: PatrolReportEvent = {
    run_id: "run-1",
    expected_node_id: "RecognizeTitle",
    matched_node_id: null,
    target: "screen-0",
    session: 1,
    frame: 12,
    outcome: "inconclusive",
    reason: "Low score",
  };
  listeners.get("patrol-report")?.({ payload: report });
  expect(onPatrol).toHaveBeenCalledWith(report);
  expect(onStatus).not.toHaveBeenCalled();

  await controller.stop();
  expect(listeners.size).toBe(0);
});
