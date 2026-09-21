import { beforeEach, expect, it } from "vitest";
import { createNodeEmulatorDraft } from "../../testing/emulator/nodeEmulatorModel";
import { useDebugStore } from "./useDebugStore";

beforeEach(() => {
  useDebugStore.setState({
    activeToolId: "inspector",
    nodeEmulatorDraft: createNodeEmulatorDraft(),
    nodeEmulatorInitialized: false,
  });
});

it("persists the active tool and normalized emulator draft but not overlay state", () => {
  useDebugStore.getState().setActiveToolId("node-emulator");
  useDebugStore.getState().setNodeEmulatorDraft({ ...createNodeEmulatorDraft(), id: "SavedNode" });
  const persisted = JSON.parse(localStorage.getItem("watchdog:debug")!);
  expect(persisted.state).toMatchObject({ activeToolId: "node-emulator", nodeEmulatorDraft: { id: "SavedNode" }, nodeEmulatorInitialized: true });
  expect(persisted.state).not.toHaveProperty("showRoi");
});

it("initializes from capture context once and only refreshes on an explicit command", () => {
  const first = { type: "screen" as const, index: 2 };
  const second = { type: "window" as const, title: "^Target$", capture_area: "client" as const };
  useDebugStore.getState().initializeNodeEmulatorTarget(first);
  useDebugStore.getState().initializeNodeEmulatorTarget(second);
  expect(useDebugStore.getState().nodeEmulatorDraft.target).toEqual(first);
  useDebugStore.getState().useNodeEmulatorTarget(second);
  expect(useDebugStore.getState().nodeEmulatorDraft.target).toEqual(second);
});

it("hydrates partial older emulator drafts with current nested defaults", async () => {
  localStorage.setItem("watchdog:debug", JSON.stringify({ state: { activeToolId: "node-emulator", nodeEmulatorDraft: { id: "OldDraft", recognition: { type: "OCR", expected: "Login" } }, nodeEmulatorInitialized: true }, version: 0 }));
  await useDebugStore.persist.rehydrate();
  expect(useDebugStore.getState().nodeEmulatorDraft).toMatchObject({ id: "OldDraft", recognition: { type: "OCR", expected: "Login", candidateOrder: "native", candidateIndex: "0" } });
  expect(useDebugStore.getState().nodeEmulatorDraft.actions.length).toBeGreaterThan(0);
});
