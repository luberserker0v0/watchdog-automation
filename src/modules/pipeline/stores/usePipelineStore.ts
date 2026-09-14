import { create } from "zustand";
import { useGraphLayoutStore } from "./useGraphLayoutStore";
import type { ParamDef, PipelineGraph, PipelineStartError, StepResult } from "../../../api/pipeline";
import {
  getPipelineStatus,
  loadPipeline,
  normalizePipelineStartError,
  pausePipeline,
  startPipeline,
  stopPipeline,
} from "../../../api/pipeline";
import { useLogStore } from "../../capture/stores/useLogStore";
import { usePreferencesStore } from "../../core/stores/usePreferencesStore";
import { useStreamStore } from "../../capture/stores/useStreamStore";

export type PipelineExecutionStatus = "idle" | "running" | "paused" | "stopping" | "stopped" | "completed" | "failed" | "needs_resize";

type PipelineBatchItem = {
  id: string;
  userParams: Record<string, string | number | boolean>;
};

export type PipelineResizeIssue = Extract<PipelineStartError, { code: "window_size_mismatch" }> & {
  pipelineId: string;
};

export interface PipelineEntry {
  id: string;
  path: string;
  name: string;
  graph: PipelineGraph | null;
  mermaid: string;
  meta: string;
  collectedParams: ParamDef[];
  userParams: Record<string, string | number | boolean>;
  isChecked: boolean;
  status: PipelineExecutionStatus;
  runId: string | null;
  currentNodeId: string | null;
  elapsedMs: number;
  layoutWritable?: boolean;
  error?: string | null;
}

export interface PipelineState {
  pipelineIds: string[];
  pipelinesById: Record<string, PipelineEntry>;
  selectedId: string | null;
  activeId: string | null;
  isExecuting: boolean;
  batchIds: string[];
  resizeIssue: PipelineResizeIssue | null;
  addPipeline: (path: string) => Promise<void>;
  removePipeline: (id: string) => void;
  selectPipeline: (id: string) => void;
  toggleCheck: (id: string) => void;
  updateParams: (id: string, key: string, value: string | number | boolean) => void;
  startExecution: () => Promise<void>;
  stopExecution: () => Promise<void>;
  pauseExecution: () => Promise<void>;
  refreshStatus: () => Promise<void>;
  retryWindowSize: () => Promise<void>;
  cancelWindowSize: () => void;
}

let idCounter = 0;
let executionGeneration = 0;
let stopRequested = false;
let pendingBatch: PipelineBatchItem[] | null = null;
const stepCountById: Record<string, number> = {};
const delay = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));
const genId = () => `pipeline-${++idCounter}`;

function appendStepLog(pipelineName: string, step: StepResult) {
  const timingGranularity = usePreferencesStore.getState().timingGranularity;
  const total = timingGranularity === "off" ? "" : ` · ${step.duration_ms}ms`;
  useLogStore.getState().appendLog({
    source: "pipeline",
    label: pipelineName,
    detail: step.skipped
      ? `${step.node_id} skipped (max_hit reached)${total}`
      : `${step.node_id} ${step.matched ? "matched" : "not matched"}${step.matched ? ` · ${step.actions_fired} actions · ${step.confidence.toFixed(2)}` : ""}${step.error ? ` · ${step.error}` : ""}${total}`,
    result: step.error ? "error" : step.matched ? "success" : "info",
  });
  if (timingGranularity === "detailed" && step.timings_ms) {
    useLogStore.getState().appendLog({
      source: "pipeline",
      label: `${step.node_id} timing`,
      detail: Object.entries(step.timings_ms).map(([phase, ms]) => `${phase.replace(/_/g, " ")} ${ms}ms`).join(" · "),
      result: "info",
    });
  }
}

function collectParams(graph: PipelineGraph): ParamDef[] {
  const params: ParamDef[] = [];
  const seenKeys = new Set<string>();
  for (const node of graph.nodes) {
    for (const param of node.params ?? []) {
      if (!seenKeys.has(param.key)) {
        seenKeys.add(param.key);
        params.push(param);
      }
    }
  }
  return params;
}

function makeDefaultParams(params: ParamDef[]) {
  return Object.fromEntries(
    params
      .filter((param) => param.default !== undefined && param.default !== null)
      .map((param) => [param.key, param.default]),
  ) as Record<string, string | number | boolean>;
}

function updateEntry(
  state: PipelineState,
  id: string,
  update: (entry: PipelineEntry) => PipelineEntry,
) {
  const entry = state.pipelinesById[id];
  if (!entry) return {};
  return { pipelinesById: { ...state.pipelinesById, [id]: update(entry) } };
}

export const usePipelineStore = create<PipelineState>((set, get) => ({
  pipelineIds: [],
  pipelinesById: {},
  selectedId: null,
  activeId: null,
  isExecuting: false,
  batchIds: [],
  resizeIssue: null,

  addPipeline: async (path) => {
    const result = await loadPipeline(path);
    useGraphLayoutStore.getState().replaceLayout(path, result.layout?.nodes ?? {});
    const params = collectParams(result.graph);
    const id = genId();
    const name = path.split(/[/\\]/).pop()?.replace(/\.[^.]+$/, "") || "Unknown";
    const entry: PipelineEntry = {
      id,
      path,
      name,
      graph: result.graph,
      mermaid: result.mermaid,
      meta: result.meta,
      collectedParams: params,
      userParams: makeDefaultParams(params),
      isChecked: true,
      status: "idle",
      runId: null,
      currentNodeId: null,
      elapsedMs: 0,
      layoutWritable: result.layout_writable,
      error: null,
    };
    set((state) => ({
      pipelineIds: [...state.pipelineIds, id],
      pipelinesById: { ...state.pipelinesById, [id]: entry },
      selectedId: state.selectedId ?? id,
    }));
  },

  removePipeline: (id) => set((state) => {
    if (!state.pipelinesById[id] || state.batchIds.includes(id)) return state;
    const pipelineIds = state.pipelineIds.filter((pipelineId) => pipelineId !== id);
    const pipelinesById = { ...state.pipelinesById };
    delete pipelinesById[id];
    return {
      pipelineIds,
      pipelinesById,
      selectedId: state.selectedId === id ? (pipelineIds[0] ?? null) : state.selectedId,
    };
  }),

  selectPipeline: (id) => set({ selectedId: id }),
  toggleCheck: (id) => set((state) => state.batchIds.includes(id) ? state : updateEntry(state, id, (entry) => ({
    ...entry,
    isChecked: !entry.isChecked,
  }))),
  updateParams: (id, key, value) => set((state) => state.batchIds.includes(id) ? state : updateEntry(state, id, (entry) => ({
    ...entry,
    userParams: { ...entry.userParams, [key]: value },
  }))),

  startExecution: async () => {
    if (get().isExecuting) return;
    stopRequested = false;
    const generation = ++executionGeneration;
    const snapshot = get();
    const batch = pendingBatch ?? snapshot.pipelineIds.filter((id) => {
      const entry = snapshot.pipelinesById[id];
      return entry?.isChecked && entry.status !== "running";
    }).map((id) => ({
      id,
      userParams: { ...snapshot.pipelinesById[id].userParams },
    }));
    if (batch.length === 0) return;
    pendingBatch = null;
    set({ isExecuting: true, batchIds: batch.map((item) => item.id), resizeIssue: null });

    for (const [batchIndex, batchItem] of batch.entries()) {
      const { id } = batchItem;
      if (generation !== executionGeneration || !get().isExecuting) break;
      const entry = get().pipelinesById[id];
      if (!entry) continue;
      try {
        const preferences = usePreferencesStore.getState();
        const runId = await startPipeline(
          entry.path,
          batchItem.userParams,
          preferences.saveDebugImages,
          preferences.timingGranularity === "detailed",
        );
        await useStreamStore.getState().refreshStatus();
        if (generation !== executionGeneration) {
          await stopPipeline(runId).catch(() => undefined);
          break;
        }
        set((state) => ({
          activeId: id,
          ...updateEntry(state, id, (current) => ({
            ...current,
            status: "running",
            runId,
            currentNodeId: null,
            elapsedMs: 0,
            error: null,
          })),
        }));
        stepCountById[id] = 0;

        useLogStore.getState().appendLog({
          source: "pipeline",
          label: entry.name,
          detail: `Pipeline started (runId: ${runId})`,
          result: "info",
        });

        let statusFailures = 0;
        let statusStopSent = false;
        while (generation === executionGeneration) {
          await delay(300);
          if (generation !== executionGeneration) break;
          let status: Awaited<ReturnType<typeof getPipelineStatus>>;
          try {
            status = await getPipelineStatus(runId);
            statusFailures = 0;
          } catch (error) {
            statusFailures++;
            if (statusFailures < 20) continue;
            const message = `Unable to read backend pipeline status: ${error instanceof Error ? error.message : String(error)}`;
            if (!statusStopSent) {
              statusStopSent = true;
              await stopPipeline(runId).catch(() => undefined);
              set((state) => updateEntry(state, id, (current) => ({
                ...current,
                status: "stopping",
                error: message,
              })));
            }
            await delay(1_000);
            continue;
          }
          const executionStatus = status.state.toLowerCase() as PipelineExecutionStatus;
          const terminal = ["completed", "failed", "stopped"].includes(executionStatus);
          const prevCount = stepCountById[id] ?? 0;
          const newSteps = status.step_results.slice(prevCount);
          stepCountById[id] = status.step_results.length;
          for (const step of newSteps) appendStepLog(entry.name, step);
          set((state) => updateEntry(state, id, (current) => ({
            ...current,
            status: executionStatus,
            runId: terminal ? null : current.runId,
            currentNodeId: terminal ? null : status.current_node_id,
            elapsedMs: status.elapsed_ms,
            error: status.error ?? null,
          })));
          if (terminal) break;
        }

        const finalEntry = get().pipelinesById[id];
        const finalStatus = finalEntry?.status ?? "unknown";
        useLogStore.getState().appendLog({
          source: "pipeline",
          label: entry.name,
          detail: `Pipeline ${finalStatus}${finalEntry?.error ? ` · ${finalEntry.error}` : ""}`,
          result: finalStatus === "completed" ? "success" : "error",
        });
        delete stepCountById[id];
        if (stopRequested) break;
      } catch (error) {
        const startError = normalizePipelineStartError(error);
        if (startError.code === "window_size_mismatch" && generation === executionGeneration) {
          pendingBatch = batch.slice(batchIndex);
          const issue: PipelineResizeIssue = { ...startError, pipelineId: id };
          set((state) => ({
            isExecuting: false,
            activeId: id,
            batchIds: pendingBatch!.map((item) => item.id),
            resizeIssue: issue,
            ...updateEntry(state, id, (current) => ({
              ...current,
              status: "needs_resize",
              runId: null,
              currentNodeId: null,
              error: startError.diagnostic,
            })),
          }));
          useLogStore.getState().appendLog({
            source: "pipeline",
            label: entry.name,
            detail: `Window content must be ${startError.expected[0]}×${startError.expected[1]}; current size is ${startError.before[0]}×${startError.before[1]}. Resize the target application and retry.`,
            result: "error",
          });
          return;
        }
        const errorMessage = startError.code === "generic" ? startError.message : startError.diagnostic;
        console.error(`Pipeline ${entry.name} failed:`, errorMessage);
        if (generation === executionGeneration) {
          set((state) => updateEntry(state, id, (current) => ({
            ...current,
            status: "failed",
            runId: null,
            currentNodeId: null,
            error: errorMessage,
          })));
        }
      }
    }
    if (generation === executionGeneration) {
      pendingBatch = null;
      set({ isExecuting: false, activeId: null, batchIds: [], resizeIssue: null });
    }
  },

  stopExecution: async () => {
    stopRequested = true;
    const running = Object.values(get().pipelinesById).filter(
      (entry) => entry.runId && (entry.status === "running" || entry.status === "paused"),
    );
    if (running.length === 0) {
      if (Object.values(get().pipelinesById).some((entry) => entry.status === "stopping")) return;
      executionGeneration++;
      pendingBatch = null;
      set({ isExecuting: false, activeId: null, batchIds: [], resizeIssue: null });
      return;
    }
    set((state) => ({
      pipelinesById: Object.fromEntries(state.pipelineIds.map((id) => {
        const entry = state.pipelinesById[id];
        return [id, running.some((item) => item.id === id) ? { ...entry, status: "stopping" as const } : entry];
      })),
    }));
    await Promise.all(running.map((entry) => stopPipeline(entry.runId!).catch((error) => {
      console.error("Failed to stop pipeline:", error);
    })));
  },

  pauseExecution: async () => {
    const snapshot = get();
    const active = snapshot.activeId ? snapshot.pipelinesById[snapshot.activeId] : undefined;
    if (active?.runId) {
      try {
        await pausePipeline(active.runId);
        const status = await getPipelineStatus(active.runId);
        set((state) => updateEntry(state, active.id, (entry) => ({
          ...entry,
          status: status.state.toLowerCase() as PipelineExecutionStatus,
          currentNodeId: status.current_node_id,
          elapsedMs: status.elapsed_ms,
          error: status.error ?? null,
        })));
      } catch (error) {
        console.error("Failed to toggle pipeline pause:", error);
      }
    }
  },

  refreshStatus: async () => {
    const running = Object.values(get().pipelinesById).filter(
      (entry) => entry.runId && (entry.status === "running" || entry.status === "paused"),
    );
    await Promise.all(running.map(async (entry) => {
      try {
        const status = await getPipelineStatus(entry.runId!);
        const executionStatus = status.state.toLowerCase() as PipelineExecutionStatus;
        const terminal = ["completed", "failed", "stopped"].includes(executionStatus);
        const prevCount = stepCountById[entry.id] ?? 0;
        const newSteps = status.step_results.slice(prevCount);
        stepCountById[entry.id] = status.step_results.length;
        for (const step of newSteps) appendStepLog(entry.name, step);
        set((state) => updateEntry(state, entry.id, (current) => ({
          ...current,
          status: executionStatus,
          runId: terminal ? null : current.runId,
          currentNodeId: terminal ? null : status.current_node_id,
          elapsedMs: status.elapsed_ms,
          error: status.error ?? null,
        })));
      } catch {
        // Retry transient status failures on the next refresh.
      }
    }));
  },

  retryWindowSize: async () => {
    const issue = get().resizeIssue;
    if (!issue || !pendingBatch) return;
    set((state) => ({
      resizeIssue: null,
      ...updateEntry(state, issue.pipelineId, (entry) => ({ ...entry, status: "idle", error: null })),
    }));
    await get().startExecution();
  },

  cancelWindowSize: () => {
    const issue = get().resizeIssue;
    executionGeneration++;
    pendingBatch = null;
    set((state) => ({
      isExecuting: false,
      activeId: null,
      batchIds: [],
      resizeIssue: null,
      ...(issue ? updateEntry(state, issue.pipelineId, (entry) => ({ ...entry, status: "idle", error: null })) : {}),
    }));
  },
}));

export const selectDisplayedPipelineId = (state: PipelineState) => state.activeId ?? state.selectedId;
