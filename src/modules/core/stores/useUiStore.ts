import { create } from "zustand";

export type ActiveView = "pipeline" | "nodeTest" | "settings";

interface UiState {
  activeView: ActiveView;
  sidebarOpen: boolean;
  previewOpen: boolean;
  setActiveView: (activeView: ActiveView) => void;
  toggleSidebar: () => void;
  setPreviewOpen: (previewOpen: boolean) => void;
}

export const useUiStore = create<UiState>((set) => ({
  activeView: "pipeline",
  sidebarOpen: false,
  previewOpen: false,
  setActiveView: (activeView) => set({ activeView }),
  toggleSidebar: () => set((state) => ({ sidebarOpen: !state.sidebarOpen })),
  setPreviewOpen: (previewOpen) => set({ previewOpen }),
}));
