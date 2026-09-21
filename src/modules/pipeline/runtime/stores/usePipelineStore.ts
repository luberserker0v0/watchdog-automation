import { create } from "zustand";
import { useGraphLayoutStore } from "../../authoring/stores/useGraphLayoutStore";
import type { ParamDef, PipelineGraph, PipelineStartError, StepResult } from "../../../../api/pipeline";
import {
  getPipelineStatus,
  loadPipeline,
  normalizePipelineStartError,
  pausePipeline,
  startPipeline,
  stopPipeline,
} from "../../../../api/pipeline";
import { useLogStore } from "../../../capture/stores/useLogStore";
import { usePreferencesStore } from "../../../shell/stores/usePreferencesStore";
import { useStreamStore } from "../../../capture/stores/useStreamStore";
import { PipelineBatchController, type PipelineStatusEvent, type PatrolReportEvent } from "../services/PipelineBatchController";
import { ensurePatrolChatListener } from "./usePatrolChatStore";

export type PipelineExecutionStatus = "idle" | "running" | "paused" | "stopping" | "stopped" | "completed" | "failed" | "needs_resize";

type PipelineBatchItem = {
  id: string;
  userParams: Record<string, Record<string, string | number | boolean>>;
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
  paramsByNode: Record<string, ParamDef[]>;
  userParams: Record<string, Record<string, string | number | boolean>>;
  isChecked: boolean;
  status: PipelineExecutionStatus;
  runId: string | null;
  currentNodeId: string | null;
  elapsedMs: number;
  layoutWritable?: boolean;
  error?: string | null;
  cleanupError?: string | null;
  attemptCount?: number;
  hitCounts?: Record<string, number>;
  lastStepResults?: StepResult[];
  stepCount?: number;
  nodeStartTime?: number;
  nodeElapsedMs?: number;
  childrenParentId?: string | null;
  childrenStartTime?: number | null;
  childrenChildId?: string | null;
  childrenTimeoutMs?: number | null;
  statusRevision?: number;
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
  updateParams: (id: string, nodeId: string, key: string, value: string | number | boolean) => void;
  startExecution: () => Promise<void>;
  stopExecution: () => Promise<void>;
  pauseExecution: () => Promise<void>;
  refreshStatus: () => Promise<void>;
  retryWindowSize: () => Promise<void>;
  cancelWindowSize: () => void;
  applyStatusEvent: (event: PipelineStatusEvent) => void;
}

let idCounter = 0;
let executionGeneration = 0;
let stopRequested = false;
let pendingBatch: PipelineBatchItem[] | null = null;
const stepCountById: Record<string, number> = {};
const terminalPatrolRunIds = new Set<string>();
const delay = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));
const genId = () => `pipeline-${++idCounter}`;
const batchController = new PipelineBatchController();

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

export function appendPatrolReport(report: PatrolReportEvent) {
  // Tauri events with different names can reach JavaScript on adjacent event
  // loop turns. Suppress a patrol report once this run's terminal status has
  // already been observed, even if the backend emitted the report first.
  if (terminalPatrolRunIds.has(report.run_id)) return;
  const capture = `${report.trigger === "failure" ? "before failure · " : ""}frame ${report.frame} · session ${report.session} · ${report.target}`;
  const detail = report.outcome === "aligned"
    ? `${capture}: ${report.expected_node_id} aligned`
    : report.outcome === "possible_mismatch"
      ? `${capture}: expected ${report.expected_node_id}; possible match ${report.matched_node_id}`
      : `${capture}: ${report.expected_node_id} inconclusive${report.reason ? ` · ${report.reason}` : ""}`;
  useLogStore.getState().appendLog({
    source: "pipeline",
    label: `Patrol · ${report.run_id}`,
    detail,
    result: report.outcome === "aligned" ? "success" : "info",
  });
}

function collectParamsByNode(graph: PipelineGraph): Record<string, ParamDef[]> {
  const result: Record<string, ParamDef[]> = {};
  for (const node of graph.nodes) {
    if (node.params?.length) {
      result[node.id] = node.params;
    }
  }
  return result;
}

function makeDefaultParamsByNode(paramsByNode: Record<string, ParamDef[]>): Record<string, Record<string, string | number | boolean>> {
  const result: Record<string, Record<string, string | number | boolean>> = {};
  for (const [nodeId, params] of Object.entries(paramsByNode)) {
    result[nodeId] = Object.fromEntries(
      params
        .filter((param) => param.default !== undefined && param.default !== null)
        .map((param) => [param.key, param.default]),
    ) as Record<string, string | number | boolean>;
  }
  return result;
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
    const paramsByNode = collectParamsByNode(result.graph);
    const id = genId();
    const name = path.split(/[/\\]/).pop()?.replace(/\.[^.]+$/, "") || "Unknown";
    const entry: PipelineEntry = {
      id,
      path,
      name,
      graph: result.graph,
      mermaid: result.mermaid,
      meta: result.meta,
      paramsByNode,
      userParams: makeDefaultParamsByNode(paramsByNode),
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
  updateParams: (id, nodeId, key, value) => set((state) => state.batchIds.includes(id) ? state : updateEntry(state, id, (entry) => ({
    ...entry,
    userParams: {
      ...entry.userParams,
      [nodeId]: { ...entry.userParams[nodeId], [key]: value },
    },
  }))),

  applyStatusEvent: (event) => set((state) => {
    const id = state.pipelineIds.find((pipelineId) => state.pipelinesById[pipelineId]?.runId === event.run_id);
    if (!id) return state;
    const entry = state.pipelinesById[id];
    if ((entry.statusRevision ?? -1) >= event.revision) return state;
    const executionStatus = event.state.toLowerCase() as PipelineExecutionStatus;
    if (["completed", "failed", "stopped"].includes(executionStatus)) {
      terminalPatrolRunIds.add(event.run_id);
    }
    const terminal = ["completed", "failed", "stopped"].includes(executionStatus) && event.cleanup_complete !== false;
    const priorSteps = stepCountById[id] ?? entry.stepCount ?? 0;
    const currentSteps = event.step_count;
    stepCountById[id] = Math.max(priorSteps, currentSteps);
    if (event.appended_step && currentSteps > priorSteps) appendStepLog(entry.name, event.appended_step);
    return updateEntry(state, id, (current) => ({
      ...current,
      status: executionStatus,
      runId: terminal ? null : current.runId,
      currentNodeId: terminal ? null : event.current_node_id,
      elapsedMs: event.elapsed_ms,
      error: event.error ?? null,
      cleanupError: event.cleanup_error ?? null,
      attemptCount: event.attempt_count,
      hitCounts: event.hit_counts ?? {},
      lastStepResults: event.appended_step ? [...(current.lastStepResults ?? []), event.appended_step].slice(-10) : current.lastStepResults,
      stepCount: currentSteps,
      childrenParentId: terminal ? null : (event.children_parent_id ?? null),
      childrenStartTime: terminal ? null : (event.children_start_time_ms != null ? Date.now() - event.children_start_time_ms : null),
      childrenChildId: terminal ? null : (event.children_child_id ?? null),
      childrenTimeoutMs: terminal ? null : (event.children_timeout_ms ?? null),
      statusRevision: event.revision,
    }));
  }),

  startExecution: async () => {
    if (get().isExecuting) return;
    await ensurePatrolChatListener();
    const hasStatusEvents = await batchController.start(
      (event) => get().applyStatusEvent(event),
      appendPatrolReport,
    );
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
          {
            enabled: preferences.patrolEnabled,
            cycle_seconds: preferences.patrolCycleSeconds,
            k_hops: preferences.patrolKHops,
            threshold: preferences.patrolThreshold,
            min_margin: preferences.patrolMinMargin,
          },
          preferences.patrolEnabled && preferences.llmEndpoint && preferences.llmModel ? {
            endpoint: preferences.llmEndpoint,
            model: preferences.llmModel,
            api_key: preferences.llmApiKey || undefined,
          } : undefined,
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
            cleanupError: null,
            childrenParentId: null,
            childrenStartTime: null,
            childrenChildId: null,
            childrenTimeoutMs: null,
            statusRevision: undefined,
          })),
        }));
        stepCountById[id] = 0;
        terminalPatrolRunIds.delete(runId);

        useLogStore.getState().appendLog({
          source: "pipeline",
          label: entry.name,
          detail: `Pipeline started (runId: ${runId})`,
          result: "info",
        });

        let statusFailures = 0;
        let statusStopSent = false;
        while (generation === executionGeneration) {
          // Events normally deliver progress first. The command status path is
          // a low-frequency recovery mechanism; tests/browser mode retain the
          // legacy cadence because no native event bridge exists there.
          await delay(hasStatusEvents ? 1_000 : 300);
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
          if (["completed", "failed", "stopped"].includes(executionStatus)) {
            terminalPatrolRunIds.add(runId);
          }
          const terminal = ["completed", "failed", "stopped"].includes(executionStatus) && status.cleanup_complete !== false;
          const prevCount = stepCountById[id] ?? 0;
          const newSteps = status.step_results.slice(prevCount);
          stepCountById[id] = status.step_results.length;
          for (const step of newSteps) appendStepLog(entry.name, step);
          const isRunning = executionStatus === "running";
          set((state) => updateEntry(state, id, (current) => {
            const polledRevision = status.revision ?? 0;
            const revisionStale = polledRevision <= (current.statusRevision ?? -1);

            const nodeId = terminal ? null : status.current_node_id;
            const nodeChanged = nodeId && nodeId !== current.currentNodeId;

            let nodeStartTime = current.nodeStartTime;
            let nodeElapsedMs = current.nodeElapsedMs ?? 0;

            // Use backend-provided children state instead of heuristic
            const childrenStartTimeMs = status.children_start_time_ms ?? null;
            const childrenParentId = revisionStale
              ? current.childrenParentId ?? null
              : (status.children_parent_id ?? null);
            // Convert backend start time (ms since children trying started) to absolute timestamp
            const childrenStartTime = revisionStale
              ? current.childrenStartTime ?? null
              : (childrenStartTimeMs !== null ? Date.now() - childrenStartTimeMs : null);

            if (nodeChanged) {
              nodeStartTime = Date.now();
              nodeElapsedMs = 0;
            } else if (isRunning && !childrenParentId && nodeStartTime) {
              nodeElapsedMs += 300;
            } else if (isRunning && childrenParentId && childrenStartTimeMs !== null) {
              // While children are tried, use backend-provided elapsed time
              nodeElapsedMs = childrenStartTimeMs;
            }

            return {
              ...current,
              status: executionStatus,
              runId: terminal ? null : current.runId,
              currentNodeId: nodeId,
              elapsedMs: status.elapsed_ms,
              error: status.error ?? null,
              cleanupError: status.cleanup_error ?? null,
              hitCounts: status.hit_counts ?? {},
              attemptCount: status.attempt_count,
              lastStepResults: status.step_results.slice(-10),
              stepCount: status.step_results.length,
              nodeStartTime,
              nodeElapsedMs,
              childrenParentId,
              childrenStartTime,
              childrenChildId: revisionStale
                ? current.childrenChildId ?? null
                : (status.children_child_id ?? null),
              childrenTimeoutMs: revisionStale
                ? current.childrenTimeoutMs ?? null
                : (status.children_timeout_ms ?? null),
              statusRevision: status.revision,
            };
          }));
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
        useLogStore.getState().appendLog({
          source: "pipeline",
          label: entry.name,
          detail: `Pipeline failed before execution · ${errorMessage}`,
          result: "error",
        });
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
          hitCounts: status.hit_counts ?? {},
          attemptCount: status.attempt_count,
          lastStepResults: status.step_results.slice(-10),
          stepCount: status.step_results.length,
          childrenParentId: status.children_parent_id ?? null,
          childrenStartTime: status.children_start_time_ms !== null && status.children_start_time_ms !== undefined
            ? Date.now() - status.children_start_time_ms
            : null,
          childrenChildId: status.children_child_id ?? null,
          childrenTimeoutMs: status.children_timeout_ms ?? null,
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
        const terminal = ["completed", "failed", "stopped"].includes(executionStatus) && status.cleanup_complete !== false;
        const prevCount = stepCountById[entry.id] ?? 0;
        const newSteps = status.step_results.slice(prevCount);
        stepCountById[entry.id] = status.step_results.length;
        for (const step of newSteps) appendStepLog(entry.name, step);
        set((state) => updateEntry(state, entry.id, (current) => {
          const polledRevision = status.revision ?? 0;
          const revisionStale = polledRevision <= (current.statusRevision ?? -1);

          return {
            ...current,
            status: executionStatus,
            runId: terminal ? null : current.runId,
            currentNodeId: terminal ? null : status.current_node_id,
            elapsedMs: status.elapsed_ms,
            error: status.error ?? null,
            cleanupError: status.cleanup_error ?? null,
            attemptCount: status.attempt_count,
            hitCounts: status.hit_counts ?? {},
            lastStepResults: status.step_results.slice(-10),
            stepCount: status.step_results.length,
            childrenParentId: revisionStale
              ? current.childrenParentId ?? null
              : (status.children_parent_id ?? null),
            childrenStartTime: revisionStale
              ? current.childrenStartTime ?? null
              : (status.children_start_time_ms !== null && status.children_start_time_ms !== undefined
                ? Date.now() - status.children_start_time_ms
                : null),
            childrenChildId: revisionStale
              ? current.childrenChildId ?? null
              : (status.children_child_id ?? null),
            childrenTimeoutMs: revisionStale
              ? current.childrenTimeoutMs ?? null
              : (status.children_timeout_ms ?? null),
            statusRevision: status.revision,
          };
        }));
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
