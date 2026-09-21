import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { PipelineStatusEvent } from "../../../../api/pipeline";
export type { PipelineStatusEvent } from "../../../../api/pipeline";

export interface PatrolReportEvent {
  run_id: string;
  trigger?: "cycle" | "failure";
  expected_node_id: string;
  matched_node_id: string | null;
  target: string;
  session: number;
  frame: number;
  outcome: "aligned" | "possible_mismatch" | "inconclusive";
  reason: string | null;
}

/**
 * Owns the app-wide pipeline status and patrol report subscriptions. The store supplies the
 * state mutation, keeping Tauri listener lifetime out of Zustand and making
 * command polling a recovery mechanism rather than the primary data path.
 */
export class PipelineBatchController {
  private unlisten: UnlistenFn | null = null;
  private unlistenPatrol: UnlistenFn | null = null;
  private starting: Promise<boolean> | null = null;

  start(onStatus: (event: PipelineStatusEvent) => void, onPatrol?: (event: PatrolReportEvent) => void): Promise<boolean> {
    if (this.unlisten) return Promise.resolve(true);
    if (this.starting) return this.starting;
    this.starting = listen<PipelineStatusEvent>("pipeline-status", ({ payload }) => {
      onStatus(payload);
    }).then(async (unlisten) => {
      this.unlisten = unlisten;
      if (onPatrol) {
        try {
          this.unlistenPatrol = await listen<PatrolReportEvent>("patrol-report", ({ payload }) => onPatrol(payload));
        } catch {
          // Native log and status evidence remain available if this listener fails.
        }
      }
      return true;
    }).catch((error) => {
      // Browser/Vitest environments do not have the Tauri event bridge. The
      // command-status recovery path remains valid there.
      // Do not surface this in browser/test environments; polling continues.
      void error;
      return false;
    }).finally(() => {
      this.starting = null;
    });
    return this.starting;
  }

  async stop(): Promise<void> {
    await this.starting;
    const unlisten = this.unlisten;
    this.unlisten = null;
    if (unlisten) unlisten();
    const unlistenPatrol = this.unlistenPatrol;
    this.unlistenPatrol = null;
    if (unlistenPatrol) unlistenPatrol();
  }
}
