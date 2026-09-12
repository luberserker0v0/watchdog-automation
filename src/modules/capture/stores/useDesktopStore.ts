import { create } from "zustand";
import type { DesktopInfo } from "../../../api/desktop";
import { getDesktops } from "../../../api/desktop";

interface DesktopState {
  desktops: DesktopInfo[];
  selectedDesktopId: string | null;
  isLoading: boolean;
  fetchDesktops: () => Promise<void>;
  setSelectedDesktop: (id: string | null) => void;
}

export const useDesktopStore = create<DesktopState>((set) => ({
  desktops: [],
  selectedDesktopId: null,
  isLoading: false,

  fetchDesktops: async () => {
    set({ isLoading: true });
    try {
      const desktops = await getDesktops();
      set({
        desktops,
        selectedDesktopId: null,
        isLoading: false,
      });
    } catch (e) {
      console.error("[DesktopFilter] failed to fetch desktops:", e);
      set({ isLoading: false });
    }
  },

  setSelectedDesktop: (id) => {
    set({ selectedDesktopId: id });
  },
}));
