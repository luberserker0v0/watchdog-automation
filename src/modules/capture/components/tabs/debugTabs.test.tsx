import { StrictMode } from "react";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import InspectorTab from "./InspectorTab";
import RecognitionTab from "./RecognitionTab";

const windowApi = vi.hoisted(() => ({ getWindowDetails: vi.fn() }));
vi.mock("../../../../api/window", () => windowApi);

describe("capture debug tabs", () => {
  it("renders window details in a document-level dialog", async () => {
    windowApi.getWindowDetails.mockResolvedValue([{
      hwnd: 42,
      process_name: "target.exe",
      class_name: "TargetWindow",
      title: "Target UI",
      width: 500,
      height: 600,
    }]);
    const user = userEvent.setup();
    const { container } = render(
      <StrictMode>
        <InspectorTab />
      </StrictMode>,
    );

    await user.click(screen.getByRole("button", { name: "視窗詳情" }));
    const dialog = await screen.findByRole("dialog", { name: "視窗詳情" });

    expect(dialog).toHaveTextContent("target.exe");
    expect(container.contains(dialog)).toBe(false);
    expect(document.body.contains(dialog)).toBe(true);
    expect(screen.getByRole("button", { name: "視窗詳情" })).toBeEnabled();
  });

  it("keeps the recognition execute button available", () => {
    render(<RecognitionTab isStreaming={false} captureBuf={[1]} captureMode="window" />);
    expect(screen.getByRole("button", { name: "執行辨識" })).toBeVisible();
    expect(screen.getByRole("textbox", { name: "Recognition ROI" })).toHaveValue("0,0,0,0");
  });
});
