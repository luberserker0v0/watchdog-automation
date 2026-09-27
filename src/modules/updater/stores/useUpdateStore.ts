import { create } from "zustand";

export type UpdatePhase =
  | "idle"
  | "checking"
  | "current"
  | "available"
  | "downloading"
  | "installing"
  | "error";

export interface UpdateMetadata {
  currentVersion: string;
  version: string;
  publicationDate: string | null;
  notes: string;
}

interface UpdateState {
  phase: UpdatePhase;
  currentVersion: string | null;
  update: UpdateMetadata | null;
  downloadedBytes: number;
  totalBytes: number | null;
  lastCheckTime: number | null;
  error: string | null;
  setState: (state: Partial<Omit<UpdateState, "setState">>) => void;
}

export const useUpdateStore = create<UpdateState>((set) => ({
  phase: "idle",
  currentVersion: null,
  update: null,
  downloadedBytes: 0,
  totalBytes: null,
  lastCheckTime: null,
  error: null,
  setState: (state) => set(state),
}));

export function resetUpdateStoreForTests(): void {
  useUpdateStore.setState({
    phase: "idle",
    currentVersion: null,
    update: null,
    downloadedBytes: 0,
    totalBytes: null,
    lastCheckTime: null,
    error: null,
  });
}
