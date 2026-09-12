import { create } from "zustand";
import type { CaptureMode } from "../../core/stores/usePreferencesStore";

interface CaptureFrame {
  previewUrl: string | null;
  captureBuf: number[] | null;
  objectUrl: boolean;
}

interface CaptureFrameState {
  frames: Record<CaptureMode, CaptureFrame>;
  setCapturedFrame: (mode: CaptureMode, url: string, captureBuf: number[]) => void;
  setStreamFrameUrl: (mode: CaptureMode, url: string) => void;
  clearFrame: (mode: CaptureMode) => void;
}

const emptyFrame = (): CaptureFrame => ({ previewUrl: null, captureBuf: null, objectUrl: false });

function revoke(frame: CaptureFrame) {
  if (frame.objectUrl && frame.previewUrl) URL.revokeObjectURL(frame.previewUrl);
}

export const useCaptureFrameStore = create<CaptureFrameState>((set) => ({
  frames: { screen: emptyFrame(), window: emptyFrame() },
  setCapturedFrame: (mode, previewUrl, captureBuf) => set((state) => {
    revoke(state.frames[mode]);
    return {
      frames: {
        ...state.frames,
        [mode]: { previewUrl, captureBuf, objectUrl: true },
      },
    };
  }),
  setStreamFrameUrl: (mode, previewUrl) => set((state) => {
    revoke(state.frames[mode]);
    return {
      frames: {
        ...state.frames,
        [mode]: { ...state.frames[mode], previewUrl, objectUrl: false },
      },
    };
  }),
  clearFrame: (mode) => set((state) => {
    revoke(state.frames[mode]);
    return { frames: { ...state.frames, [mode]: emptyFrame() } };
  }),
}));
