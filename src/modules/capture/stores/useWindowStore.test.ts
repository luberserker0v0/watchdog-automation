import { beforeEach, expect, it, vi } from "vitest";
import { getWindows } from "../../../api/window";
import { useDesktopStore } from "./useDesktopStore";
import { useWindowStore } from "./useWindowStore";

vi.mock("../../../api/window", () => ({
  getWindows: vi.fn().mockResolvedValue([]),
  captureWindowByHwnd: vi.fn(),
}));

beforeEach(() => {
  vi.clearAllMocks();
  useDesktopStore.setState({ selectedDesktopId: null });
  useWindowStore.setState({ windowStateFilter: "Foreground" });
});

it("passes the selected desktop only to window enumeration", async () => {
  useDesktopStore.getState().setSelectedDesktop("desktop-2");
  await useWindowStore.getState().fetchWindows();
  expect(getWindows).toHaveBeenCalledWith("Foreground", "desktop-2");

  useDesktopStore.getState().setSelectedDesktop(null);
  await useWindowStore.getState().fetchWindows();
  expect(getWindows).toHaveBeenLastCalledWith("Foreground", undefined);
});
