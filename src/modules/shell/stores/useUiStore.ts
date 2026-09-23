import { create } from "zustand";

export type ActiveView = "pipeline" | "nodeTest" | "patrolChat" | "actionRecorder" | "settings";

interface UiState {
  activeView: ActiveView;
  sidebarOpen: boolean;
  previewOpen: boolean;
  capturePanelOpen: boolean;
  setActiveView: (activeView: ActiveView) => void;
  toggleSidebar: () => void;
  setPreviewOpen: (previewOpen: boolean) => void;
  setCapturePanelOpen: (capturePanelOpen: boolean) => void;
  toggleCapturePanel: () => void;
}

export const useUiStore = create<UiState>((set) => ({
  activeView: "pipeline",
  sidebarOpen: false,
  previewOpen: false,
  capturePanelOpen: false,
  setActiveView: (activeView) => set({ activeView }),
  toggleSidebar: () => set((state) => ({ sidebarOpen: !state.sidebarOpen })),
  setPreviewOpen: (previewOpen) => set({ previewOpen }),
  setCapturePanelOpen: (capturePanelOpen) => set({ capturePanelOpen }),
  toggleCapturePanel: () => set((state) => ({ capturePanelOpen: !state.capturePanelOpen })),
}));
