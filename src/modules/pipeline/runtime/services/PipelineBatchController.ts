import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { PipelineStatusEvent } from "../../../../api/pipeline";
export type { PipelineStatusEvent } from "../../../../api/pipeline";

export interface PatrolReportEvent {
  run_id: string;
  trigger?: "cycle" | "failure";
  task?: "state_match" | "action_effect";
  action_attempt_id?: number | null;
  expected_node_id: string;
  matched_node_id: string | null;
  target: string;
  session: number;
  frame: number;
  outcome: "aligned" | "possible_mismatch" | "achieved" | "unchanged" | "in_progress" | "unexpected" | "inconclusive" | "circuit_open";
  reason: string | null;
}

export interface PatrolRecoveryEvent {
  run_id: string;
  policy: "hint_only" | "redirection_only";
  failure_node: string;
  target_node: string | null;
  decision: "suggest" | "redirect" | "no_recovery" | "inconclusive";
  confidence: number | null;
  reason: string;
  accepted: boolean;
  redirect_count: number;
  max_redirects: number;
  location_node?: string | null;
  location_confidence?: number | null;
  location_source?: "state_matcher" | "action_expected_successor" | "action_error_successor" | null;
  location_frame_stable?: boolean | null;
  location_uncertain?: boolean;
}

/**
 * Owns the app-wide pipeline status and patrol report subscriptions. The store supplies the
 * state mutation, keeping Tauri listener lifetime out of Zustand and making
 * command polling a recovery mechanism rather than the primary data path.
 */
export class PipelineBatchController {
  private unlisten: UnlistenFn | null = null;
  private unlistenPatrol: UnlistenFn | null = null;
  private unlistenRecovery: UnlistenFn | null = null;
  private starting: Promise<boolean> | null = null;

  start(onStatus: (event: PipelineStatusEvent) => void, onPatrol?: (event: PatrolReportEvent) => void, onRecovery?: (event: PatrolRecoveryEvent) => void): Promise<boolean> {
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
      if (onRecovery) {
        try {
          this.unlistenRecovery = await listen<PatrolRecoveryEvent>("patrol-recovery", ({ payload }) => onRecovery(payload));
        } catch {
          // Native log and Chat Room evidence remain available if this listener fails.
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
    const unlistenRecovery = this.unlistenRecovery;
    this.unlistenRecovery = null;
    if (unlistenRecovery) unlistenRecovery();
  }
}
