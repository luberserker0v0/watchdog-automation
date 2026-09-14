import { Profiler, type ProfilerOnRenderCallback } from "react";
import { act, render } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import PipelineCard from "./PipelineCard";
import { type PipelineEntry, usePipelineStore } from "../stores/usePipelineStore";

const entry = (id: string): PipelineEntry => ({
  id,
  path: `${id}.yml`,
  name: id,
  graph: null,
  mermaid: id,
  meta: "",
  collectedParams: [],
  userParams: {},
  isChecked: true,
  status: "idle",
  runId: null,
  currentNodeId: null,
  elapsedMs: 0,
});

beforeEach(() => {
  const a = entry("a");
  const b = entry("b");
  usePipelineStore.setState({
    pipelineIds: ["a", "b"],
    pipelinesById: { a, b },
    selectedId: "a",
    activeId: null,
    isExecuting: false,
    batchIds: [],
    resizeIssue: null,
  });
});

describe("PipelineCard render isolation", () => {
  it("does not commit an inactive sibling when one entity changes", () => {
    const commits = { a: 0, b: 0 };
    const onRender: ProfilerOnRenderCallback = (id) => { commits[id as "a" | "b"]++; };
    render(<>
      <Profiler id="a" onRender={onRender}><PipelineCard id="a" onPreview={() => undefined} /></Profiler>
      <Profiler id="b" onRender={onRender}><PipelineCard id="b" onPreview={() => undefined} /></Profiler>
    </>);
    commits.a = 0;
    commits.b = 0;

    act(() => usePipelineStore.getState().updateParams("a", "value", "new"));

    expect(commits.a).toBe(1);
    expect(commits.b).toBe(0);
  });

  it("does not commit cards for elapsed-only polling ticks", () => {
    usePipelineStore.setState((state) => ({
      pipelinesById: { ...state.pipelinesById, a: { ...state.pipelinesById.a, status: "running" } },
    }));
    const commits = { a: 0, b: 0 };
    const onRender: ProfilerOnRenderCallback = (id) => { commits[id as "a" | "b"]++; };
    render(<>
      <Profiler id="a" onRender={onRender}><PipelineCard id="a" onPreview={() => undefined} /></Profiler>
      <Profiler id="b" onRender={onRender}><PipelineCard id="b" onPreview={() => undefined} /></Profiler>
    </>);
    commits.a = 0;
    commits.b = 0;
    act(() => usePipelineStore.setState((state) => ({
      pipelinesById: { ...state.pipelinesById, a: { ...state.pipelinesById.a, elapsedMs: 600 } },
    })));
    expect(commits).toEqual({ a: 0, b: 0 });
  });

  it("commits only the previous and next card when selection changes", () => {
    const commits = { a: 0, b: 0 };
    const onRender: ProfilerOnRenderCallback = (id) => { commits[id as "a" | "b"]++; };
    render(<>
      <Profiler id="a" onRender={onRender}><PipelineCard id="a" onPreview={() => undefined} /></Profiler>
      <Profiler id="b" onRender={onRender}><PipelineCard id="b" onPreview={() => undefined} /></Profiler>
    </>);
    commits.a = 0;
    commits.b = 0;
    act(() => usePipelineStore.getState().selectPipeline("b"));
    expect(commits).toEqual({ a: 1, b: 1 });
  });

  it("keeps card interactions ID-scoped", async () => {
    const preview = vi.fn();
    const user = userEvent.setup();
    usePipelineStore.setState((state) => ({
      pipelinesById: {
        ...state.pipelinesById,
        b: {
          ...state.pipelinesById.b,
          collectedParams: [{ type: "text", key: "name", label: "Name" }],
        },
      },
    }));
    const { getByRole, getByTitle } = render(<PipelineCard id="b" onPreview={preview} />);
    await user.click(getByTitle("Preview graph"));
    expect(preview).toHaveBeenCalledWith("b");
    await user.click(getByRole("button", { name: "▾" }));
    await user.type(getByRole("textbox"), "Alice");
    expect(usePipelineStore.getState().pipelinesById.b.userParams.name).toBe("Alice");
    await user.click(getByRole("checkbox"));
    expect(usePipelineStore.getState().pipelinesById.b.isChecked).toBe(false);
  });

  it("shows actionable expected and current sizes for a blocked preflight", async () => {
    const user = userEvent.setup();
    usePipelineStore.setState((state) => ({
      activeId: "a",
      batchIds: ["a"],
      resizeIssue: {
        pipelineId: "a",
        code: "window_size_mismatch",
        hwnd: 42,
        title: "Target",
        expected: [1280, 720],
        before: [900, 600],
        actual: [900, 600],
        resize_attempted: true,
        diagnostic: "Window rejected resizing",
      },
      pipelinesById: {
        ...state.pipelinesById,
        a: { ...state.pipelinesById.a, status: "needs_resize" },
      },
    }));
    const { getByRole, getByText } = render(<PipelineCard id="a" onPreview={() => undefined} />);
    expect(getByText(/Expected content size 1280 × 720/)).toBeTruthy();
    expect(getByText(/current size 900 × 600/)).toBeTruthy();
    await user.click(getByRole("button", { name: "Cancel" }));
    expect(usePipelineStore.getState().resizeIssue).toBeNull();
    expect(usePipelineStore.getState().pipelinesById.a.status).toBe("idle");
  });
});
