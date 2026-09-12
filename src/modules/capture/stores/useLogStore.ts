import { create } from "zustand";

export interface LogEntry {
  id: number;
  source: "pipeline" | "debug";
  label: string;
  detail: string;
  result?: "success" | "error" | "info";
  timestamp: number;
}

interface LogState {
  entries: LogEntry[];
  nextId: number;
  appendLog: (entry: Omit<LogEntry, "id" | "timestamp">) => void;
  clearLogs: () => void;
}

export const useLogStore = create<LogState>()(
  // persist(
    (set) => ({
      entries: [],
      nextId: 1,
      appendLog: (entry) =>
        set((s) => ({
          entries: [
            ...s.entries.slice(-499),
            { ...entry, id: s.nextId, timestamp: Date.now() },
          ],
          nextId: s.nextId + 1,
        })),
      clearLogs: () => set({ entries: [], nextId: 1 }),
    }),
  //   {
  //     name: "watchdog:logs",
  //     partialize: (state) => ({
  //       entries: state.entries.slice(-200),
  //     }),
  //   },
  // ),
);
