import { create } from "zustand";
import { persist } from "zustand/middleware";

type ActionType = "Click" | "LongPress" | "Swipe" | "Scroll" | "InputText" | "ClickKey";
type RecogType = "DirectHit" | "TemplateMatch" | "OCR" | "ColorMatch";

interface DebugState {
  showSelection: boolean;
  showRoi: boolean;
  showResultRoi: boolean;
  debugPanelOpen: boolean;

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
  toggleDebugPanel: () => void;
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
      debugPanelOpen: false,

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
      toggleDebugPanel: () => set((s) => ({ debugPanelOpen: !s.debugPanelOpen })),
    }),
    {
      name: "watchdog:debug",
      partialize: (state) => ({
        showSelection: state.showSelection,
        showRoi: state.showRoi,
        showResultRoi: state.showResultRoi,
        actionType: state.actionType,
        actionFields: state.actionFields,
        recogType: state.recogType,
        recogFields: state.recogFields,
        debugPanelOpen: state.debugPanelOpen,
      }),
    },
  ),
);
