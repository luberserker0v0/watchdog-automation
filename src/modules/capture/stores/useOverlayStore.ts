import { create } from "zustand";

export interface OverlayRect {
  x: number;
  y: number;
  w: number;
  h: number;
}

interface OverlayState {
  cursorX: number | null;
  cursorY: number | null;
  selection: OverlayRect | null;
  roi: OverlayRect | null;
  resultRoi: OverlayRect | null;
  imageDimensions: { width: number; height: number } | null;
  setCursor: (x: number, y: number) => void;
  clearCursor: () => void;
  setSelection: (selection: OverlayRect | null) => void;
  clearSelection: () => void;
  setRoi: (roi: OverlayRect | null) => void;
  setResultRoi: (resultRoi: OverlayRect | null) => void;
  setImageDimensions: (imageDimensions: { width: number; height: number } | null) => void;
  clearRecognition: () => void;
  clearAll: () => void;
}

export const useOverlayStore = create<OverlayState>((set) => ({
  cursorX: null,
  cursorY: null,
  selection: null,
  roi: null,
  resultRoi: null,
  imageDimensions: null,
  setCursor: (cursorX, cursorY) => set({ cursorX, cursorY }),
  clearCursor: () => set({ cursorX: null, cursorY: null }),
  setSelection: (selection) => set({ selection }),
  clearSelection: () => set({ selection: null }),
  setRoi: (roi) => set({ roi }),
  setResultRoi: (resultRoi) => set({ resultRoi }),
  setImageDimensions: (imageDimensions) => set({ imageDimensions }),
  clearRecognition: () => set({ roi: null, resultRoi: null }),
  clearAll: () => set({ selection: null, roi: null, resultRoi: null }),
}));
