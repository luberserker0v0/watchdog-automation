import { beforeEach, expect, it } from "vitest";
import { useGraphLayoutStore } from "./useGraphLayoutStore";

beforeEach(() => useGraphLayoutStore.setState({ layouts: {}, minimapVisible: true }));

it("persists rounded node positions per pipeline without modifying another layout", () => {
  const store = useGraphLayoutStore.getState();
  store.setNodePosition("sample/sample.md", "Login", { x: 120.4, y: 80.8 });
  store.setNodePositions("sample/sample.md", {
    Login: { x: 125.2, y: 85.7 },
    Password: { x: 240.8, y: 180.1 },
  });
  store.setNodePosition("other.md", "Start", { x: 10, y: 20 });

  expect(useGraphLayoutStore.getState().layouts).toEqual({
    "sample/sample.md": { Login: { x: 125, y: 86 }, Password: { x: 241, y: 180 } },
    "other.md": { Start: { x: 10, y: 20 } },
  });
  expect(JSON.parse(localStorage.getItem("watchdog:graph-layout")!).state).not.toHaveProperty("layouts");

  useGraphLayoutStore.getState().setMinimapVisible(false);
  expect(JSON.parse(localStorage.getItem("watchdog:graph-layout")!).state.minimapVisible).toBe(false);

  useGraphLayoutStore.getState().clearLayout("sample/sample.md");
  expect(useGraphLayoutStore.getState().layouts).toEqual({
    "other.md": { Start: { x: 10, y: 20 } },
  });

  useGraphLayoutStore.getState().replaceLayout("other.md", { End: { x: 30, y: 40 } });
  expect(useGraphLayoutStore.getState().layouts["other.md"]).toEqual({ End: { x: 30, y: 40 } });
});
