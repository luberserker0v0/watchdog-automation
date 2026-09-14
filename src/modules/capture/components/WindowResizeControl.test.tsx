import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const windowApi = vi.hoisted(() => ({
  resizeWindowClient: vi.fn(),
  getWindows: vi.fn(),
  captureWindowByHwnd: vi.fn(),
}));
vi.mock("../../../api/window", () => windowApi);

import { useStreamStore } from "../stores/useStreamStore";
import { useWindowStore } from "../stores/useWindowStore";
import WindowResizeControl from "./WindowResizeControl";

describe("WindowResizeControl", () => {
  const fetchWindows = vi.fn(async () => undefined);

  beforeEach(() => {
    vi.clearAllMocks();
    useStreamStore.setState({ captureOwner: "none", frameSize: [1296, 759], clientSize: [1200, 680] });
    useWindowStore.setState({
      selectedIndex: 0,
      windows: [{ index: 0, title: "Target", app_name: "Target", width: 1296, height: 759, hwnd: 42 }],
      fetchWindows,
    });
  });

  afterEach(cleanup);

  it("uses only the manually entered client size", async () => {
    windowApi.resizeWindowClient.mockResolvedValue({
      hwnd: 42,
      expected: [1000, 700],
      actual: [1000, 700],
      resized: true,
    });
    render(<WindowResizeControl />);

    fireEvent.change(screen.getByLabelText("Client width"), { target: { value: "1000" } });
    fireEvent.change(screen.getByLabelText("Client height"), { target: { value: "700" } });
    fireEvent.click(screen.getByRole("button", { name: "Resize" }));

    await waitFor(() => expect(screen.getByText("Resized to 1000×700")).toBeInTheDocument());
    expect(windowApi.resizeWindowClient).toHaveBeenCalledWith(42, [1000, 700]);
    expect(useStreamStore.getState().clientSize).toEqual([1000, 700]);
  });

  it("disables resize while a pipeline owns capture", () => {
    useStreamStore.setState({ captureOwner: "pipeline" });
    render(<WindowResizeControl />);
    fireEvent.change(screen.getByLabelText("Client width"), { target: { value: "1000" } });
    fireEvent.change(screen.getByLabelText("Client height"), { target: { value: "700" } });

    expect(screen.getByRole("button", { name: "Resize" })).toBeDisabled();
    expect(screen.getByText("Pipeline owns capture")).toBeInTheDocument();
  });
});
