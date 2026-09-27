import { create } from "zustand";
import { persist } from "zustand/middleware";
import { watchdogJsonStorage } from "../../shell/services/persistence";
import type { PipelineTarget } from "../../../api/pipeline";
import {
  createNodeEmulatorDraft,
  defaultActionDraft,
  defaultParamDraft,
  type NodeEmulatorDraft,
} from "../../testing/emulator/nodeEmulatorModel";

type ActionType = "Click" | "LongPress" | "Swipe" | "Scroll" | "InputText" | "ClickKey";
type RecogType = "DirectHit" | "TemplateMatch" | "OCR" | "ColorMatch";
export type DebugToolId = "inspector" | "action" | "recognition" | "node-emulator";

interface DebugState {
  showSelection: boolean;
  showRoi: boolean;
  showResultRoi: boolean;
  activeToolId: DebugToolId;
  nodeEmulatorDraft: NodeEmulatorDraft;
  nodeEmulatorInitialized: boolean;

  actionType: ActionType;
  actionFields: Record<string, string>;
  recogType: RecogType;
  recogFields: Record<string, string>;

  setShowSelection: (show: boolean) => void;
  setShowRoi: (show: boolean) => void;
  setShowResultRoi: (show: boolean) => void;
  setActionType: (type: ActionType) => void;
  setActionField: (key: string, value: string) => void;
  clearActionFields: () => void;
  setRecogType: (type: RecogType) => void;
  setRecogField: (key: string, value: string) => void;
  clearRecogFields: () => void;
  setActiveToolId: (id: DebugToolId) => void;
  setNodeEmulatorDraft: (draft: NodeEmulatorDraft) => void;
  initializeNodeEmulatorTarget: (target: PipelineTarget) => void;
  useNodeEmulatorTarget: (target: PipelineTarget) => void;
}

export const useDebugStore = create<DebugState>()(
  persist(
    (set) => ({
      showSelection: true,
      showRoi: true,
      showResultRoi: true,

      actionType: "Click" as ActionType,
      actionFields: {},
      recogType: "DirectHit" as RecogType,
      recogFields: {},
      activeToolId: "inspector" as DebugToolId,
      nodeEmulatorDraft: createNodeEmulatorDraft(),
      nodeEmulatorInitialized: false,

      setShowSelection: (show) => set({ showSelection: show }),
      setShowRoi: (show) => set({ showRoi: show }),
      setShowResultRoi: (show) => set({ showResultRoi: show }),
      setActionType: (type) => set({ actionType: type }),
      setActionField: (key, value) =>
        set((s) => ({ actionFields: { ...s.actionFields, [key]: value } })),
      clearActionFields: () => set({ actionFields: {} }),
      setRecogType: (type) => set({ recogType: type }),
      setRecogField: (key, value) =>
        set((s) => ({ recogFields: { ...s.recogFields, [key]: value } })),
      clearRecogFields: () => set({ recogFields: {} }),
      setActiveToolId: (activeToolId) => set({ activeToolId }),
      setNodeEmulatorDraft: (nodeEmulatorDraft) => set({ nodeEmulatorDraft, nodeEmulatorInitialized: true }),
      initializeNodeEmulatorTarget: (target) => set((state) => state.nodeEmulatorInitialized
        ? state
        : {
            nodeEmulatorDraft: { ...state.nodeEmulatorDraft, target },
            nodeEmulatorInitialized: true,
          }),
      useNodeEmulatorTarget: (target) => set((state) => ({
        nodeEmulatorDraft: { ...state.nodeEmulatorDraft, target },
        nodeEmulatorInitialized: true,
      })),
    }),
    {
      name: "watchdog:debug",
      storage: watchdogJsonStorage,
      partialize: (state) => ({
        actionType: state.actionType,
        actionFields: state.actionFields,
        recogType: state.recogType,
        recogFields: state.recogFields,
        activeToolId: state.activeToolId,
        nodeEmulatorDraft: state.nodeEmulatorDraft,
        nodeEmulatorInitialized: state.nodeEmulatorInitialized,
      }),
      merge: (persisted, current) => {
        const stored = persisted as Partial<DebugState>;
        const storedDraft = stored.nodeEmulatorDraft;
        return {
          ...current,
          ...stored,
          nodeEmulatorInitialized: stored.nodeEmulatorInitialized ?? Boolean(storedDraft),
          nodeEmulatorDraft: storedDraft
            ? {
                ...current.nodeEmulatorDraft,
                ...storedDraft,
                recognition: { ...current.nodeEmulatorDraft.recognition, ...storedDraft.recognition },
                actions: Array.isArray(storedDraft.actions)
                  ? storedDraft.actions.map((action) => ({ ...defaultActionDraft(), ...action }))
                  : current.nodeEmulatorDraft.actions,
                params: Array.isArray(storedDraft.params)
                  ? storedDraft.params.map((param) => ({ ...defaultParamDraft(), ...param }))
                  : current.nodeEmulatorDraft.params,
              }
            : current.nodeEmulatorDraft,
        };
      },
    },
  ),
);
