import { create } from "zustand";
import type { ParamDef, PipelineGraph, StepResult } from "../../../api/pipeline";
import {
  getPipelineStatus,
  loadPipeline,
  pausePipeline,
  startPipeline,
  stopPipeline,
} from "../../../api/pipeline";
import { useLogStore } from "../../capture/stores/useLogStore";
import { usePreferencesStore } from "../../core/stores/usePreferencesStore";

export type PipelineExecutionStatus = "idle" | "running" | "paused" | "completed" | "failed";

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
}

export interface PipelineState {
  pipelineIds: string[];
  pipelinesById: Record<string, PipelineEntry>;
  selectedId: string | null;
  activeId: string | null;
  isExecuting: boolean;
  addPipeline: (path: string) => Promise<void>;
  removePipeline: (id: string) => void;
  selectPipeline: (id: string) => void;
  toggleCheck: (id: string) => void;
  updateParams: (id: string, key: string, value: string | number | boolean) => void;
  startExecution: () => Promise<void>;
  stopExecution: () => Promise<void>;
  pauseExecution: () => Promise<void>;
  refreshStatus: () => Promise<void>;
}

let idCounter = 0;
let executionGeneration = 0;
const stepCountById: Record<string, number> = {};
const delay = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));
const genId = () => `pipeline-${++idCounter}`;

function appendStepLog(pipelineName: string, step: StepResult) {
  const timingGranularity = usePreferencesStore.getState().timingGranularity;
  const total = timingGranularity === "off" ? "" : ` · ${step.duration_ms}ms`;
  useLogStore.getState().appendLog({
    source: "pipeline",
    label: pipelineName,
    detail: `${step.node_id} ${step.matched ? "matched" : "not matched"}${step.matched ? ` · ${step.actions_fired} actions · ${step.confidence.toFixed(2)}` : ""}${step.error ? ` · ${step.error}` : ""}${total}`,
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

  addPipeline: async (path) => {
    const result = await loadPipeline(path);
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
    };
    set((state) => ({
      pipelineIds: [...state.pipelineIds, id],
      pipelinesById: { ...state.pipelinesById, [id]: entry },
      selectedId: state.selectedId ?? id,
    }));
  },

  removePipeline: (id) => set((state) => {
    if (!state.pipelinesById[id]) return state;
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
  toggleCheck: (id) => set((state) => updateEntry(state, id, (entry) => ({
    ...entry,
    isChecked: !entry.isChecked,
  }))),
  updateParams: (id, key, value) => set((state) => updateEntry(state, id, (entry) => ({
    ...entry,
    userParams: { ...entry.userParams, [key]: value },
  }))),

  startExecution: async () => {
    const generation = ++executionGeneration;
    const snapshot = get();
    const ids = snapshot.pipelineIds.filter((id) => {
      const entry = snapshot.pipelinesById[id];
      return entry?.isChecked && entry.status !== "running";
    });
    if (ids.length === 0) return;
    set({ isExecuting: true });

    for (const id of ids) {
      if (generation !== executionGeneration || !get().isExecuting) break;
      const entry = get().pipelinesById[id];
      if (!entry) continue;
      try {
        const preferences = usePreferencesStore.getState();
        const runId = await startPipeline(
          entry.path,
          entry.userParams,
          preferences.saveDebugImages,
          preferences.timingGranularity === "detailed",
        );
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
          })),
        }));
        stepCountById[id] = 0;

        useLogStore.getState().appendLog({
          source: "pipeline",
          label: entry.name,
          detail: `Pipeline started (runId: ${runId})`,
          result: "info",
        });

        while (generation === executionGeneration) {
          await delay(300);
          if (generation !== executionGeneration) break;
          const status = await getPipelineStatus(runId);
          const executionStatus = status.state.toLowerCase() as PipelineExecutionStatus;
          const prevCount = stepCountById[id] ?? 0;
          const newSteps = status.step_results.slice(prevCount);
          stepCountById[id] = status.step_results.length;
          for (const step of newSteps) appendStepLog(entry.name, step);
          set((state) => updateEntry(state, id, (current) => ({
            ...current,
            status: executionStatus,
            currentNodeId: status.current_node_id,
            elapsedMs: status.elapsed_ms,
          })));
          if (executionStatus === "completed" || executionStatus === "failed") break;
        }

        const finalStatus = get().pipelinesById[id]?.status ?? "unknown";
        useLogStore.getState().appendLog({
          source: "pipeline",
          label: entry.name,
          detail: `Pipeline ${finalStatus}`,
          result: finalStatus === "completed" ? "success" : "error",
        });
        delete stepCountById[id];
      } catch (error) {
        console.error(`Pipeline ${entry.name} failed:`, error);
        if (generation === executionGeneration) {
          set((state) => updateEntry(state, id, (current) => ({ ...current, status: "failed" })));
        }
      }
    }
    if (generation === executionGeneration) set({ isExecuting: false, activeId: null });
  },

  stopExecution: async () => {
    executionGeneration++;
    const running = Object.values(get().pipelinesById).filter(
      (entry) => entry.runId && (entry.status === "running" || entry.status === "paused"),
    );
    await Promise.all(running.map((entry) => stopPipeline(entry.runId!).catch((error) => {
      console.error("Failed to stop pipeline:", error);
    })));
    set((state) => ({
      isExecuting: false,
      activeId: null,
      pipelinesById: Object.fromEntries(state.pipelineIds.map((id) => {
        const entry = state.pipelinesById[id];
        const unchanged = entry.status === "idle" && entry.runId === null && entry.currentNodeId === null;
        return [id, unchanged ? entry : { ...entry, status: "idle", runId: null, currentNodeId: null }];
      })),
    }));
  },

  pauseExecution: async () => {
    executionGeneration++;
    const snapshot = get();
    const active = snapshot.activeId ? snapshot.pipelinesById[snapshot.activeId] : undefined;
    if (active?.runId) {
      await pausePipeline(active.runId).catch((error) => console.error("Failed to pause pipeline:", error));
    }
    set((state) => ({
      isExecuting: false,
      activeId: null,
      pipelinesById: Object.fromEntries(state.pipelineIds.map((id) => {
        const entry = state.pipelinesById[id];
        return [id, entry.status === "running"
          ? { ...entry, status: "paused", runId: null, currentNodeId: null }
          : entry];
      })),
    }));
  },

  refreshStatus: async () => {
    const running = Object.values(get().pipelinesById).filter(
      (entry) => entry.runId && (entry.status === "running" || entry.status === "paused"),
    );
    await Promise.all(running.map(async (entry) => {
      try {
        const status = await getPipelineStatus(entry.runId!);
        const prevCount = stepCountById[entry.id] ?? 0;
        const newSteps = status.step_results.slice(prevCount);
        stepCountById[entry.id] = status.step_results.length;
        for (const step of newSteps) appendStepLog(entry.name, step);
        set((state) => updateEntry(state, entry.id, (current) => ({
          ...current,
          status: status.state.toLowerCase() as PipelineExecutionStatus,
          currentNodeId: status.current_node_id,
          elapsedMs: status.elapsed_ms,
        })));
      } catch {
        // Retry transient status failures on the next refresh.
      }
    }));
  },
}));

export const selectDisplayedPipelineId = (state: PipelineState) => state.activeId ?? state.selectedId;
