import { create } from "zustand";
import { persist } from "zustand/middleware";

export interface GraphNodePosition {
  x: number;
  y: number;
}

type PipelineLayout = Record<string, GraphNodePosition>;

interface GraphLayoutState {
  layouts: Record<string, PipelineLayout>;
  minimapVisible: boolean;
  setNodePosition: (pipelineKey: string, nodeId: string, position: GraphNodePosition) => void;
  setNodePositions: (pipelineKey: string, positions: PipelineLayout) => void;
  replaceLayout: (pipelineKey: string, positions: PipelineLayout) => void;
  clearLayout: (pipelineKey: string) => void;
  setMinimapVisible: (visible: boolean) => void;
}

export const useGraphLayoutStore = create<GraphLayoutState>()(
  persist(
    (set) => ({
      layouts: {},
      minimapVisible: true,
      setNodePosition: (pipelineKey, nodeId, position) => set((state) => ({
        layouts: {
          ...state.layouts,
          [pipelineKey]: {
            ...state.layouts[pipelineKey],
            [nodeId]: {
              x: Math.round(position.x),
              y: Math.round(position.y),
            },
          },
        },
      })),
      setNodePositions: (pipelineKey, positions) => set((state) => ({
        layouts: {
          ...state.layouts,
          [pipelineKey]: {
            ...state.layouts[pipelineKey],
            ...Object.fromEntries(Object.entries(positions).map(([nodeId, position]) => [
              nodeId,
              { x: Math.round(position.x), y: Math.round(position.y) },
            ])),
          },
        },
      })),
      replaceLayout: (pipelineKey, positions) => set((state) => ({
        layouts: {
          ...state.layouts,
          [pipelineKey]: Object.fromEntries(Object.entries(positions).map(([nodeId, position]) => [
            nodeId,
            { x: Math.round(position.x), y: Math.round(position.y) },
          ])),
        },
      })),
      clearLayout: (pipelineKey) => set((state) => {
        if (!state.layouts[pipelineKey]) return state;
        const layouts = { ...state.layouts };
        delete layouts[pipelineKey];
        return { layouts };
      }),
      setMinimapVisible: (minimapVisible) => set({ minimapVisible }),
    }),
    {
      name: "watchdog:graph-layout",
      partialize: (state) => ({ minimapVisible: state.minimapVisible }),
      merge: (persisted, current) => ({
        ...current,
        minimapVisible: (persisted as Partial<GraphLayoutState>).minimapVisible ?? true,
      }),
    },
  ),
);
