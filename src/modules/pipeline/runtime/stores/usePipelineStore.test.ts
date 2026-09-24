import { beforeEach, describe, expect, it, vi } from "vitest";
import { useLogStore } from "../../../observability/stores/useLogStore";
import { usePreferencesStore } from "../../../shell/stores/usePreferencesStore";
import { appendPatrolRecovery, appendPatrolReport, usePipelineStore } from "./usePipelineStore";

const pipelineApi = vi.hoisted(() => ({
  loadPipeline: vi.fn(async (path: string) => ({
    graph: {
      version: "1",
      nodes: [{
        id: "start",
        recognition: { type: "DirectHit" },
        params: [{ type: "text", key: "name", label: "Name", default: path }],
      }],
      edges: [],
    },
    mermaid: "start",
    meta: "",
  })),
  startPipeline: vi.fn(async () => "run-1"),
  stopPipeline: vi.fn(async () => undefined),
  pausePipeline: vi.fn(async () => undefined),
  getPipelineStatus: vi.fn(async () => ({
    state: "Completed",
    current_node_id: null,
    elapsed_ms: 300,
    step_results: [],
  })),
  normalizePipelineStartError: vi.fn((error: unknown) => error),
}));
vi.mock("../../../../api/pipeline", () => pipelineApi);

beforeEach(() => {
  vi.clearAllMocks();
  usePipelineStore.setState({
    pipelineIds: [],
    pipelinesById: {},
    selectedId: null,
    activeId: null,
    isExecuting: false,
    batchIds: [],
    resizeIssue: null,
  });
  useLogStore.setState({ entries: [], nextId: 1 });
  usePreferencesStore.setState({ patrolEnabled: false, patrolKHops: 1, patrolThreshold: 0.85, patrolMinMargin: 0.1, patrolRecoveryPolicy: "hint_only", patrolMaxRedirects: 1, llmEndpoint: "", llmModel: "", llmApiKey: "" });
});

describe("usePipelineStore", () => {
  it("normalizes added pipelines and preserves sibling identity", async () => {
    await usePipelineStore.getState().addPipeline("one.yml");
    await usePipelineStore.getState().addPipeline("two.yml");
    const before = usePipelineStore.getState();
    const [firstId, secondId] = before.pipelineIds;
    const second = before.pipelinesById[secondId];

    before.updateParams(firstId, "start", "name", "changed");
    const after = usePipelineStore.getState();

    expect(after.pipelinesById[firstId].userParams["start"].name).toBe("changed");
    expect(after.pipelinesById[secondId]).toBe(second);
  });

  it("updates selection and removes the selected entity consistently", async () => {
    await usePipelineStore.getState().addPipeline("one.yml");
    await usePipelineStore.getState().addPipeline("two.yml");
    const [firstId, secondId] = usePipelineStore.getState().pipelineIds;
    usePipelineStore.getState().selectPipeline(secondId);
    usePipelineStore.getState().removePipeline(secondId);
    expect(usePipelineStore.getState().pipelineIds).toEqual([firstId]);
    expect(usePipelineStore.getState().selectedId).toBe(firstId);
    expect(usePipelineStore.getState().pipelinesById[secondId]).toBeUndefined();
  });

  it("finishes a sequential run with a single non-overlapping polling timer", async () => {
    vi.useFakeTimers();
    await usePipelineStore.getState().addPipeline("one.yml");
    const id = usePipelineStore.getState().pipelineIds[0];
    const execution = usePipelineStore.getState().startExecution();
    await Promise.resolve();
    await vi.advanceTimersByTimeAsync(300);
    await execution;
    expect((pipelineApi.startPipeline.mock.calls[0] as unknown[])[4]).toMatchObject({ enabled: false });
    expect((pipelineApi.startPipeline.mock.calls[0] as unknown[])[5]).toBeUndefined();
    expect(usePipelineStore.getState().isExecuting).toBe(false);
    expect(usePipelineStore.getState().pipelinesById[id].status).toBe("completed");
    vi.useRealTimers();
  });

  it("passes enabled patrol and thresholds to engine startup", async () => {
    vi.useFakeTimers();
    usePreferencesStore.setState({ patrolEnabled: true, patrolCycleSeconds: 30, patrolKHops: 2, patrolThreshold: 0.91, patrolActionEffectThreshold: 0.82, patrolRedirectThreshold: 0.93, patrolMinMargin: 0.12, llmEndpoint: "http://localhost:8000/v1/chat/completions", llmModel: "vision", llmApiKey: "session-key" });
    await usePipelineStore.getState().addPipeline("one.yml");
    const execution = usePipelineStore.getState().startExecution();
    await Promise.resolve();
    await vi.advanceTimersByTimeAsync(300);
    await execution;
    expect(pipelineApi.startPipeline).toHaveBeenCalledWith(
      "one.yml", expect.any(Object), expect.any(Boolean), expect.any(Boolean),
      { enabled: true, cycle_seconds: 30, k_hops: 2, threshold: 0.91, action_effect_threshold: 0.82, redirect_threshold: 0.93, min_margin: 0.12, recovery_policy: "hint_only", max_redirects: 1 },
      { endpoint: "http://localhost:8000/v1/chat/completions", model: "vision", api_key: "session-key" },
    );
    expect(useLogStore.getState().entries).toContainEqual(expect.objectContaining({
      label: "Patrol active",
      tag: "hint",
      detail: expect.stringContaining("advisory only"),
    }));
    vi.useRealTimers();
  });

  it("keeps the same run id and polling session while paused", async () => {
    await usePipelineStore.getState().addPipeline("one.yml");
    const id = usePipelineStore.getState().pipelineIds[0];
    usePipelineStore.setState((state) => ({
      activeId: id,
      isExecuting: true,
      batchIds: [id],
      pipelinesById: {
        ...state.pipelinesById,
        [id]: { ...state.pipelinesById[id], status: "running", runId: "run-1" },
      },
    }));
    pipelineApi.getPipelineStatus.mockResolvedValueOnce({
      state: "paused",
      current_node_id: null,
      elapsed_ms: 500,
      step_results: [],
    });

    await usePipelineStore.getState().pauseExecution();

    expect(pipelineApi.pausePipeline).toHaveBeenCalledWith("run-1");
    expect(usePipelineStore.getState().activeId).toBe(id);
    expect(usePipelineStore.getState().isExecuting).toBe(true);
    expect(usePipelineStore.getState().pipelinesById[id]).toMatchObject({
      status: "paused",
      runId: "run-1",
    });
  });

  it("applies only newer backend status events and appends a step once", async () => {
    await usePipelineStore.getState().addPipeline("one.yml");
    const id = usePipelineStore.getState().pipelineIds[0];
    usePipelineStore.setState((state) => ({
      pipelinesById: {
        ...state.pipelinesById,
        [id]: { ...state.pipelinesById[id], status: "running", runId: "run-events" },
      },
    }));
    const event = {
      run_id: "run-events", revision: 2, state: "running", current_node_id: "start", step_count: 1,
      elapsed_ms: 42, error: null,
      appended_step: { node_id: "start", matched: true, confidence: 1, box_region: null, actions_fired: 0, duration_ms: 42, error: null },
    };
    usePipelineStore.getState().applyStatusEvent(event);
    usePipelineStore.getState().applyStatusEvent({ ...event, revision: 1, current_node_id: "stale" });

    const entry = usePipelineStore.getState().pipelinesById[id];
    expect(entry.currentNodeId).toBe("start");
    expect(entry.statusRevision).toBe(2);
    expect(entry.lastStepResults).toHaveLength(1);
    expect(useLogStore.getState().entries.filter((item) => item.label === "one")).toHaveLength(1);
  });

  it("drops a patrol report delivered after terminal status", async () => {
    await usePipelineStore.getState().addPipeline("one.yml");
    const id = usePipelineStore.getState().pipelineIds[0];
    usePipelineStore.setState((state) => ({
      pipelinesById: {
        ...state.pipelinesById,
        [id]: { ...state.pipelinesById[id], status: "running", runId: "run-terminal" },
      },
    }));
    usePipelineStore.getState().applyStatusEvent({
      run_id: "run-terminal", revision: 1, state: "completed", current_node_id: null,
      step_count: 0, elapsed_ms: 50, error: null, cleanup_complete: false,
    });

    appendPatrolReport({
      run_id: "run-terminal", trigger: "cycle", expected_node_id: "Done",
      matched_node_id: "Done", target: "screen", session: 1, frame: 9,
      outcome: "aligned", reason: null,
    });

    expect(useLogStore.getState().entries).toHaveLength(0);
  });

  it("formats action-effect patrol reports separately from state matches", () => {
    appendPatrolReport({
      run_id: "run-action", trigger: "cycle", task: "action_effect", action_attempt_id: 7,
      expected_node_id: "ClickLogin", matched_node_id: null, target: "window", session: 2,
      frame: 15, outcome: "unchanged", reason: "Login form remains visible",
    });

    const entries = useLogStore.getState().entries;
    expect(entries[entries.length - 1]).toMatchObject({
      result: "info",
      label: "Patrol · Action check",
      detail: "ClickLogin · no visible change",
    });
  });

  it("shows a declared action error state without raw capture details", () => {
    appendPatrolReport({
      run_id: "run-error", trigger: "failure", task: "action_effect", action_attempt_id: 4,
      expected_node_id: "ClickLogin", matched_node_id: "LoginError", target: "Window { hwnd: 1 }",
      session: 1, frame: 51, outcome: "unexpected", reason: "Invalid account or password is visible",
    });

    const entries = useLogStore.getState().entries;
    expect(entries[entries.length - 1]).toMatchObject({
      label: "Patrol · Action check",
      result: "error",
      detail: "Before failure · ClickLogin → LoginError · unexpected state",
    });
  });

  it("formats recovery hints and accepted redirects", () => {
    appendPatrolRecovery({
      run_id: "hint", policy: "hint_only", failure_node: "LoginError",
      target_node: "InputPassword", decision: "suggest", confidence: 0.91,
      reason: "Correct the password", accepted: false, redirect_count: 0, max_redirects: 1,
    });
    appendPatrolRecovery({
      run_id: "redirect", policy: "redirection_only", failure_node: "LoginError",
      target_node: "InputPassword", decision: "redirect", confidence: 0.92,
      reason: "Login form remains visible", accepted: true, redirect_count: 1, max_redirects: 1,
    });
    expect(useLogStore.getState().entries.slice(-2)).toMatchObject([
      { label: "Patrol · Recovery hint", tag: "hint", result: "info", detail: expect.stringContaining("advisory only; routing unchanged") },
      { label: "Patrol · Redirect", tag: "redirect", result: "success", detail: expect.stringContaining("accepted · 1/1") },
    ]);
  });

  it("adds a GUI log entry when pipeline startup fails", async () => {
    await usePipelineStore.getState().addPipeline("broken.yml");
    pipelineApi.startPipeline.mockRejectedValueOnce({
      code: "generic",
      message: "Target window did not appear within 10000ms after launch",
    });

    await usePipelineStore.getState().startExecution();

    const entry = usePipelineStore.getState().pipelinesById[
      usePipelineStore.getState().pipelineIds[0]
    ];
    expect(entry).toMatchObject({ status: "failed", error: expect.stringContaining("Target window") });
    const logs = useLogStore.getState().entries;
    expect(logs[logs.length - 1]).toMatchObject({
      source: "pipeline",
      result: "error",
      detail: expect.stringContaining("Pipeline failed before execution"),
    });
  });

  it("blocks on a size mismatch and retries only the unfinished batch", async () => {
    vi.useFakeTimers();
    await usePipelineStore.getState().addPipeline("one.yml");
    await usePipelineStore.getState().addPipeline("two.yml");
    const [, secondId] = usePipelineStore.getState().pipelineIds;
    pipelineApi.startPipeline
      .mockResolvedValueOnce("run-1")
      .mockRejectedValueOnce({
        code: "window_size_mismatch",
        hwnd: 42,
        title: "Target",
        expected: [1280, 720],
        before: [900, 600],
        actual: [900, 600],
        resize_attempted: true,
        diagnostic: "Window rejected resizing",
      })
      .mockResolvedValueOnce("run-2");

    const firstAttempt = usePipelineStore.getState().startExecution();
    await vi.advanceTimersByTimeAsync(300);
    await firstAttempt;
    expect(usePipelineStore.getState().pipelinesById[secondId].status).toBe("needs_resize");
    expect(usePipelineStore.getState().resizeIssue?.expected).toEqual([1280, 720]);

    const retry = usePipelineStore.getState().retryWindowSize();
    await vi.advanceTimersByTimeAsync(300);
    await retry;
    expect(pipelineApi.startPipeline).toHaveBeenCalledTimes(3);
    expect((pipelineApi.startPipeline.mock.calls[2] as unknown[])[0]).toBe("two.yml");
    expect(usePipelineStore.getState().pipelinesById[secondId].status).toBe("completed");
    vi.useRealTimers();
  });
});
