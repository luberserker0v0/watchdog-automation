import { Profiler, type ProfilerOnRenderCallback } from "react";
import { fireEvent, render } from "@testing-library/react";
import { expect, it, vi } from "vitest";
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
  Object.defineProperty(split, "getBoundingClientRect", { value: () => ({ left: 0, right: 1000, width: 1000 }) });
  fireEvent.mouseDown(container.querySelector(".divider")!);
  fireEvent.mouseMove(window, { clientX: 700 });
  fireEvent.mouseUp(window);
  expect(commits).toEqual({ left: 0, right: 0 });
  expect((container.querySelector(".panel-left") as HTMLElement).style.width).toBe("calc(100% - 308px)");
  expect((container.querySelector(".panel-right") as HTMLElement).style.width).toBe("300px");
});

it("uses a 48px utility rail or gives the workspace the full width when hidden", () => {
  const view = render(<ResizableSplit left={<span>left</span>} right={<span>right</span>} rightCollapsed />);
  expect(view.container.querySelector(".split-layout")).toHaveClass("right-collapsed");
  expect((view.container.querySelector(".panel-right") as HTMLElement).style.width).toBe("48px");
  expect((view.container.querySelector(".panel-left") as HTMLElement).style.width).toBe("calc(100% - 48px)");

  view.rerender(<ResizableSplit left={<span>left</span>} right={<span>right</span>} rightHidden />);
  expect(view.container.querySelector(".split-layout")).toHaveClass("right-hidden");
  expect((view.container.querySelector(".panel-right") as HTMLElement).style.width).toBe("0px");
  expect((view.container.querySelector(".panel-left") as HTMLElement).style.width).toBe("calc(100% - 0px)");
});

it("reserves at least 420px for the main workspace while expanding the right panel", () => {
  const view = render(<ResizableSplit left={<span>left</span>} right={<span>right</span>} />);
  const split = view.container.querySelector(".split-layout") as HTMLDivElement;
  Object.defineProperty(split, "getBoundingClientRect", { value: () => ({ left: 0, right: 1000, width: 1000 }) });

  fireEvent.mouseDown(view.container.querySelector(".divider")!);
  fireEvent.mouseMove(window, { clientX: 100 });
  fireEvent.mouseUp(window);

  expect((view.container.querySelector(".panel-right") as HTMLElement).style.width).toBe("520px");
  expect((view.container.querySelector(".panel-left") as HTMLElement).style.width).toBe("calc(100% - 528px)");
});

it("reclamps an expanded panel when its container becomes narrow", () => {
  let notify: ((entries: Array<{ contentRect: { width: number } }>) => void) | undefined;
  const disconnect = vi.fn();
  const original = globalThis.ResizeObserver;
  globalThis.ResizeObserver = class {
    constructor(callback: ResizeObserverCallback) {
      notify = callback as unknown as typeof notify;
    }
    observe() {}
    unobserve() {}
    disconnect() { disconnect(); }
  };

  const view = render(<ResizableSplit left={<span>left</span>} right={<span>right</span>} />);
  notify?.([{ contentRect: { width: 700 } }]);
  expect((view.container.querySelector(".panel-right") as HTMLElement).style.width).toBe("272px");
  expect((view.container.querySelector(".panel-left") as HTMLElement).style.width).toBe("calc(100% - 280px)");

  view.unmount();
  expect(disconnect).toHaveBeenCalledOnce();
  globalThis.ResizeObserver = original;
});
