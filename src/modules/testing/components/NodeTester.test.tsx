import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { NodeMetaInfo } from "../../../api/nodeTest";
import { useLogStore } from "../../observability/stores/useLogStore";
import NodeTester, { nodeTestPage } from "./NodeTester";

const api = vi.hoisted(() => ({
  listNodeMetas: vi.fn(),
  openWindow: vi.fn(),
  testNode: vi.fn(),
}));

vi.mock("../../../api/nodeTest", () => api);

const inputNode: NodeMetaInfo = {
  id: "InputAccount",
  file_name: "InputAccount",
  target: { type: "window", title: "Target UI - Login" },
  recognition: { type: "DirectHit" },
  actions: [{ type: "InputText", input_text: "{{account}}" }],
  params: [{ type: "text", key: "account", label: "Account", default: "admin" }],
  enabled: true,
};

describe("NodeTester parameters", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useLogStore.getState().clearLogs();
    api.listNodeMetas.mockResolvedValue([inputNode]);
  });

  it("groups nodes by target page", async () => {
    const dashboardNode: NodeMetaInfo = {
      ...inputNode,
      id: "ClickRefresh",
      file_name: "ClickRefresh",
      target: { type: "window", title: "^Target UI - Dashboard$" },
      actions: [{ type: "Click", target: [10, 10] }],
      params: [],
    };
    api.listNodeMetas.mockResolvedValue([dashboardNode, inputNode]);

    render(<NodeTester />);

    expect(await screen.findByRole("group", { name: "Login" })).toBeInTheDocument();
    expect(screen.getByRole("group", { name: "Dashboard" })).toBeInTheDocument();
    expect(nodeTestPage(inputNode)).toBe("Login");
    expect(nodeTestPage(dashboardNode)).toBe("Dashboard");
  });

  it("renders node-defined fields and sends edited values to the test command", async () => {
    api.testNode.mockResolvedValue({
      success: true,
      matched: true,
      confidence: 1,
      box_region: null,
      actions_fired: 1,
      duration_ms: 5,
      error: null,
      candidates: [],
    });
    const user = userEvent.setup();
    render(<NodeTester />);

    const account = await screen.findByDisplayValue("admin");
    await user.clear(account);
    await user.type(account, "developer");
    await user.click(screen.getByRole("button", { name: "Start This Node" }));

    await waitFor(() => expect(api.testNode).toHaveBeenCalled());
    expect(api.testNode.mock.calls[0][2]).toEqual({ account: "developer" });
  });

  it("reports a completed unmatched recognition as failed", async () => {
    api.testNode.mockResolvedValue({
      success: false,
      matched: false,
      confidence: 0.2,
      box_region: null,
      actions_fired: 0,
      duration_ms: 5,
      error: null,
      candidates: [],
    });
    const user = userEvent.setup();
    render(<NodeTester />);

    await user.click(await screen.findByRole("button", { name: "Start This Node" }));

    await waitFor(() => {
      const entries = useLogStore.getState().entries;
      expect(entries.some((entry) => entry.detail.includes("recognition failed") && entry.result === "error")).toBe(true);
      expect(entries.some((entry) => entry.detail.includes("failed in 5ms") && entry.result === "error")).toBe(true);
    });
  });
});
