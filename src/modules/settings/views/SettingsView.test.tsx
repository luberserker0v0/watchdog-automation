import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, it } from "vitest";
import { usePreferencesStore } from "../../shell/stores/usePreferencesStore";
import SettingsView from "./SettingsView";

beforeEach(() => {
  usePreferencesStore.setState({ patrolEnabled: false, patrolCycleSeconds: 20, patrolRecoveryPolicy: "hint_only", patrolMaxRedirects: 1, taskNotificationsEnabled: false, recorderNotificationsEnabled: false, recorderStartStopHotkey: "CommandOrControl+Alt+F9", recorderPauseHotkey: "CommandOrControl+Alt+F10", recorderCaptureText: false });
});

it("enables patrol with one switch and exposes search settings", async () => {
  const user = userEvent.setup();
  render(<SettingsView />);
  const hops = screen.getByLabelText("鄰近搜尋距離 K") as HTMLInputElement;
  const cycle = screen.getByLabelText("巡檢週期（秒）") as HTMLInputElement;
  expect(cycle.value).toBe("20");
  expect(cycle.disabled).toBe(true);
  expect(hops.disabled).toBe(true);
  expect(screen.queryByLabelText("巡檢模式")).toBeNull();
  await user.click(screen.getByLabelText("啟用巡檢"));
  expect(hops.disabled).toBe(false);
  await user.selectOptions(screen.getByLabelText("失敗復原方式"), "redirection_only");
  expect(screen.getByLabelText("每次執行最多重新導向次數")).toBeTruthy();
  expect(usePreferencesStore.getState().patrolRecoveryPolicy).toBe("redirection_only");
  await user.clear(cycle);
  await user.type(cycle, "30");
  await user.tab();
  expect(usePreferencesStore.getState().patrolCycleSeconds).toBe(30);
  expect(usePreferencesStore.getState().patrolEnabled).toBe(true);
  await user.click(screen.getByLabelText("啟用巡檢"));
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
