import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import { usePreferencesStore } from "../../core/stores/usePreferencesStore";
import { useUiStore } from "../../core/stores/useUiStore";
import { useStreamStore } from "../stores/useStreamStore";
import CaptureView from "./CaptureView";

vi.mock("../components/ModeSelector", () => ({ default: () => <div>Mode control</div> }));
vi.mock("../components/TargetSelect", () => ({ default: () => <div>Target control</div> }));
vi.mock("../components/StreamButton", () => ({ default: () => <button>Stream control</button> }));
vi.mock("../components/CaptureButton", () => ({ default: () => <button>Capture control</button> }));
vi.mock("../components/CapturePreviewCard", () => ({ default: () => <div>Bounded thumbnail</div> }));
vi.mock("../components/ExpandButton", () => ({ default: () => <button>Preview</button> }));
vi.mock("../components/DesktopFilter", () => ({ default: () => <div>Desktop filter</div> }));
vi.mock("../components/WindowStateFilter", () => ({ default: () => <div>Window filter</div> }));
vi.mock("../components/WindowResizeControl", () => ({ default: () => <div>Resize control</div> }));
vi.mock("../components/CaptureAreaSelector", () => ({ default: () => <div>Area control</div> }));

beforeEach(() => {
  usePreferencesStore.setState({ mode: "window" });
  useUiStore.setState({ capturePanelOpen: false });
  useStreamStore.setState({ isStreaming: false, captureOwner: "none" });
});

it("starts as a compact rail and expands into only capture controls", () => {
  render(<CaptureView />);
  expect(screen.getByRole("complementary", { name: "Capture controls" })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Preview" })).toBeInTheDocument();

  fireEvent.click(screen.getByRole("button", { name: "Expand capture panel" }));
  expect(screen.getByText("Mode control")).toBeInTheDocument();
  expect(screen.getByText("Target control")).toBeInTheDocument();
  expect(screen.getByText("Bounded thumbnail")).toBeInTheDocument();
  expect(screen.queryByText(/Recognition|Node Emulator|Coordinate/)).not.toBeInTheDocument();
});
