import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, it, vi } from "vitest";
import { usePreferencesStore } from "../../shell/stores/usePreferencesStore";
import { useScreenStore } from "../stores/useScreenStore";
import { useWindowStore } from "../stores/useWindowStore";
import TargetSelect from "./TargetSelect";

const fetchScreens = useScreenStore.getState().fetchScreens;
const fetchWindows = useWindowStore.getState().fetchWindows;

beforeEach(() => {
  usePreferencesStore.setState({ mode: "screen" });
  useScreenStore.setState({ screens: [{ index: 0, name: "Main", width: 1920, height: 1080, is_primary: true }], selectedIndex: null, isLoading: false, fetchScreens });
  useWindowStore.setState({ windows: [{ index: 0, title: "Editor", app_name: "Code", width: 800, height: 600, hwnd: 1 }], selectedIndex: null, isLoading: false, fetchWindows });
});

it("offers the correct reload action for each capture mode", async () => {
  const user = userEvent.setup();
  const reloadScreens = vi.fn().mockResolvedValue(undefined);
  const reloadWindows = vi.fn().mockResolvedValue(undefined);
  useScreenStore.setState({ fetchScreens: reloadScreens });
  useWindowStore.setState({ fetchWindows: reloadWindows });

  const { rerender } = render(<TargetSelect />);
  await user.click(screen.getByRole("button", { name: "Reload screens" }));
  expect(reloadScreens).toHaveBeenCalledOnce();

  act(() => actMode("window"));
  rerender(<TargetSelect />);
  await user.click(screen.getByRole("button", { name: "Reload windows using current filters" }));
  expect(reloadWindows).toHaveBeenCalledOnce();
});

it("mounts only the active target selector and updates its store", async () => {
  const user = userEvent.setup();
  const { getByRole, rerender } = render(<TargetSelect />);
  await user.selectOptions(getByRole("combobox"), "0");
  expect(useScreenStore.getState().selectedIndex).toBe(0);
  act(() => actMode("window"));
  rerender(<TargetSelect />);
  expect(getByRole("option", { name: "Editor - 800x600" })).toBeInTheDocument();
});

function actMode(mode: "screen" | "window") {
  usePreferencesStore.getState().setMode(mode);
}
