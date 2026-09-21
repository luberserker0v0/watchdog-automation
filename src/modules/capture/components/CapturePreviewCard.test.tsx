import { render } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { usePreferencesStore } from "../../shell/stores/usePreferencesStore";
import { useUiStore } from "../../shell/stores/useUiStore";
import { useCaptureFrameStore } from "../stores/useCaptureFrameStore";
import CapturePreviewCard from "./CapturePreviewCard";

vi.mock("./TargetInfo", () => ({ default: () => <div>Target status</div> }));
vi.mock("./ExpandButton", () => ({ default: ({ label }: { label?: string }) => <button>{label}</button> }));

beforeEach(() => {
  usePreferencesStore.setState({ mode: "screen" });
  useUiStore.setState({ previewOpen: false });
  useCaptureFrameStore.setState({
    frames: {
      screen: { previewUrl: "/latest-frame.png", captureBuf: [1], objectUrl: false },
      window: { previewUrl: null, captureBuf: null, objectUrl: false },
    },
  });
});

describe("CapturePreviewCard", () => {
  it("keeps the latest thumbnail visible while the detached Preview is open", () => {
    useUiStore.setState({ previewOpen: true });
    const view = render(<CapturePreviewCard />);

    expect(view.getByRole("img", { name: "Latest capture" })).toHaveAttribute("src", "/latest-frame.png");
    expect(view.getByRole("button", { name: "Focus Preview" })).toBeVisible();
  });
});
