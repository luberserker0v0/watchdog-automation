import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, it, vi } from "vitest";
import { usePreferencesStore } from "../../shell/stores/usePreferencesStore";
import SettingsView from "./SettingsView";

const { openRelatedFolder } = vi.hoisted(() => ({
  openRelatedFolder: vi.fn().mockResolvedValue(undefined),
}));
vi.mock("../../../api/settings", () => ({
  listRelatedFolders: vi.fn().mockResolvedValue([
    { key: "logs", name: "logs", purpose: "Application and execution logs", path: "C:\\Users\\tester\\.watch_dog\\logs" },
  ]),
  openRelatedFolder,
}));

beforeEach(() => {
  usePreferencesStore.setState({ patrolEnabled: false, patrolCycleSeconds: 20, patrolRecoveryPolicy: "hint_only", patrolMaxRedirects: 1, taskNotificationsEnabled: false, recorderNotificationsEnabled: false, recorderStartStopHotkey: "CommandOrControl+Alt+F9", recorderPauseHotkey: "CommandOrControl+Alt+F10", recorderCaptureText: false });
});

it("enables patrol with one switch and exposes search settings", async () => {
  const user = userEvent.setup();
  render(<SettingsView />);
  const hops = screen.getByLabelText("Neighbor search distance K") as HTMLInputElement;
  const cycle = screen.getByLabelText("Patrol interval (seconds)") as HTMLInputElement;
  expect(cycle.value).toBe("20");
  expect(cycle.disabled).toBe(true);
  expect(hops.disabled).toBe(true);
  expect(screen.queryByLabelText("巡檢模式")).toBeNull();
  await user.click(screen.getByLabelText("Enable Patrol"));
  expect(hops.disabled).toBe(false);
  await user.selectOptions(screen.getByLabelText("Failure recovery"), "redirection_only");
  expect(screen.getByLabelText("Maximum redirects per run")).toBeTruthy();
  expect(usePreferencesStore.getState().patrolRecoveryPolicy).toBe("redirection_only");
  await user.clear(cycle);
  await user.type(cycle, "30");
  await user.tab();
  expect(usePreferencesStore.getState().patrolCycleSeconds).toBe(30);
  expect(usePreferencesStore.getState().patrolEnabled).toBe(true);
  await user.click(screen.getByLabelText("Enable Patrol"));
  expect(usePreferencesStore.getState().patrolEnabled).toBe(false);
});

it("shows notification and recorder controls and rejects duplicate shortcuts", async () => {
  const user = userEvent.setup();
  render(<SettingsView />);
  expect(screen.getByLabelText("Pipeline completion")).toBeTruthy();
  expect(screen.getByLabelText("Capture typed text")).not.toBeChecked();
  expect(screen.getByText(/Development notifications may show PowerShell branding/)).toBeTruthy();
  await user.click(screen.getByRole("button", { name: "Record Pause / Resume shortcut" }));
  expect(screen.getByLabelText("Pause / Resume shortcut")).toHaveValue("Press shortcut… (Esc to cancel)");
  fireEvent.keyDown(window, { key: "F9", code: "F9", ctrlKey: true, altKey: true });
  expect(await screen.findByText("Shortcuts must be different.")).toBeTruthy();
  expect(usePreferencesStore.getState().recorderPauseHotkey).toBe("CommandOrControl+Alt+F10");
  fireEvent.keyDown(window, { key: "F8", code: "F8", ctrlKey: true, altKey: true });
  expect(usePreferencesStore.getState().recorderPauseHotkey).toBe("CommandOrControl+Alt+F8");
});

it("shows related folder purposes and opens an allowlisted folder", async () => {
  const user = userEvent.setup();
  render(<SettingsView />);
  expect(await screen.findByText("Application and execution logs")).toBeTruthy();
  expect(screen.getByText("C:\\Users\\tester\\.watch_dog\\logs")).toBeTruthy();
  await user.click(screen.getByRole("button", { name: "Open logs folder" }));
  expect(openRelatedFolder).toHaveBeenCalledWith("logs");
});

it("switches the GUI to Traditional Chinese and persists the choice", async () => {
  const user = userEvent.setup();
  render(<SettingsView />);
  await user.selectOptions(screen.getByLabelText("Language"), "zh-TW");
  expect(await screen.findByRole("heading", { name: "設定" })).toBeTruthy();
  expect(screen.getByLabelText("開始 / 停止快捷鍵")).toBeTruthy();
  expect(screen.getByLabelText("暫停 / 繼續快捷鍵")).toBeTruthy();
  expect(screen.getByText(/可列印按鍵會成為遮罩佔位符/)).toBeTruthy();
  expect(screen.getByText("Windows · macOS/Linux 規劃中")).toBeTruthy();
  expect(localStorage.getItem("watchdog:language")).toBe("zh-TW");
});
