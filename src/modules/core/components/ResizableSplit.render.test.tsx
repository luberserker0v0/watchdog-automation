import { Profiler, type ProfilerOnRenderCallback } from "react";
import { fireEvent, render } from "@testing-library/react";
import { expect, it } from "vitest";
import ResizableSplit from "./ResizableSplit";

it("resizes panels without committing their React subtrees", () => {
  const commits = { left: 0, right: 0 };
  const onRender: ProfilerOnRenderCallback = (id) => { commits[id as "left" | "right"]++; };
  const { container } = render(
    <ResizableSplit
      left={<Profiler id="left" onRender={onRender}><span>left</span></Profiler>}
      right={<Profiler id="right" onRender={onRender}><span>right</span></Profiler>}
    />,
  );
  commits.left = 0;
  commits.right = 0;
  const split = container.querySelector(".split-layout") as HTMLDivElement;
  Object.defineProperty(split, "getBoundingClientRect", { value: () => ({ left: 0, width: 1000 }) });
  fireEvent.mouseDown(container.querySelector(".divider")!);
  fireEvent.mouseMove(window, { clientX: 700 });
  fireEvent.mouseUp(window);
  expect(commits).toEqual({ left: 0, right: 0 });
  expect((container.querySelector(".panel-left") as HTMLElement).style.width).toBe("70%");
});
