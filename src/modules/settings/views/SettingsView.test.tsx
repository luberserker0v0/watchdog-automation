import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, it } from "vitest";
import { usePreferencesStore } from "../../shell/stores/usePreferencesStore";
import SettingsView from "./SettingsView";

beforeEach(() => {
  usePreferencesStore.setState({ patrolEnabled: false, patrolCycleSeconds: 20 });
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
  await user.clear(cycle);
  await user.type(cycle, "30");
  await user.tab();
  expect(usePreferencesStore.getState().patrolCycleSeconds).toBe(30);
  expect(usePreferencesStore.getState().patrolEnabled).toBe(true);
  await user.click(screen.getByLabelText("啟用巡檢"));
  expect(usePreferencesStore.getState().patrolEnabled).toBe(false);
});
