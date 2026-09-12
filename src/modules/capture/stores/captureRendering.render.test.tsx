import { Profiler, type ProfilerOnRenderCallback } from "react";
import { act, render } from "@testing-library/react";
import { beforeEach, expect, it } from "vitest";
import { useCaptureFrameStore } from "./useCaptureFrameStore";
import { useScreenStore } from "./useScreenStore";
import { useOverlayStore } from "./useOverlayStore";
import { useDebugStore } from "./useDebugStore";

function FrameConsumer() {
  const url = useCaptureFrameStore((state) => state.frames.screen.previewUrl);
  return <span>{url}</span>;
}

function TargetConsumer() {
  const selected = useScreenStore((state) => state.selectedIndex);
  return <span>{selected}</span>;
}

beforeEach(() => {
  useCaptureFrameStore.setState({
    frames: {
      screen: { previewUrl: null, captureBuf: null, objectUrl: false },
      window: { previewUrl: null, captureBuf: null, objectUrl: false },
    },
  });
  useScreenStore.setState({ selectedIndex: 0 });
});

it("frame ticks do not commit target metadata consumers", () => {
  const commits = { frame: 0, target: 0 };
  const onRender: ProfilerOnRenderCallback = (id) => { commits[id as "frame" | "target"]++; };
  render(<>
    <Profiler id="frame" onRender={onRender}><FrameConsumer /></Profiler>
    <Profiler id="target" onRender={onRender}><TargetConsumer /></Profiler>
  </>);
  commits.frame = 0;
  commits.target = 0;
  act(() => useCaptureFrameStore.getState().setStreamFrameUrl("screen", "/frame?t=1"));
  expect(commits).toEqual({ frame: 1, target: 0 });
});

it("cursor traffic does not commit persisted debug-preference consumers", () => {
  const commits = { cursor: 0, preferences: 0 };
  const onRender: ProfilerOnRenderCallback = (id) => { commits[id as "cursor" | "preferences"]++; };
  function Cursor() {
    const x = useOverlayStore((state) => state.cursorX);
    return <span>{x}</span>;
  }
  function Preferences() {
    const type = useDebugStore((state) => state.actionType);
    return <span>{type}</span>;
  }
  render(<>
    <Profiler id="cursor" onRender={onRender}><Cursor /></Profiler>
    <Profiler id="preferences" onRender={onRender}><Preferences /></Profiler>
  </>);
  commits.cursor = 0;
  commits.preferences = 0;
  act(() => useOverlayStore.getState().setCursor(10, 20));
  expect(commits).toEqual({ cursor: 1, preferences: 0 });
});
