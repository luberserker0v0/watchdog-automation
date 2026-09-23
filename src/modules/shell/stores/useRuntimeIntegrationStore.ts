import { create } from "zustand";

export interface CaptureRuntimeContext {
  pipelineExecuting: boolean;
  expectedWindowSize: [number, number] | null;
  resizeBeforeSize: [number, number] | null;
}

interface RuntimeIntegrationState extends CaptureRuntimeContext {
  publishCaptureContext: (context: CaptureRuntimeContext) => void;
}

const EMPTY_CONTEXT: CaptureRuntimeContext = {
  pipelineExecuting: false,
  expectedWindowSize: null,
  resizeBeforeSize: null,
};

export const useRuntimeIntegrationStore = create<RuntimeIntegrationState>((set) => ({
  ...EMPTY_CONTEXT,
  publishCaptureContext: (context) => set(context),
}));

export function resetRuntimeIntegrationForTests(): void {
  useRuntimeIntegrationStore.setState(EMPTY_CONTEXT);
}
