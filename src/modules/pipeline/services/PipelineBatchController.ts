import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { PipelineStatusEvent } from "../../../api/pipeline";
export type { PipelineStatusEvent } from "../../../api/pipeline";

/**
 * Owns the one app-wide pipeline-status subscription. The store supplies the
 * state mutation, keeping Tauri listener lifetime out of Zustand and making
 * command polling a recovery mechanism rather than the primary data path.
 */
export class PipelineBatchController {
  private unlisten: UnlistenFn | null = null;
  private starting: Promise<boolean> | null = null;

  start(onStatus: (event: PipelineStatusEvent) => void): Promise<boolean> {
    if (this.unlisten) return Promise.resolve(true);
    if (this.starting) return this.starting;
    this.starting = listen<PipelineStatusEvent>("pipeline-status", ({ payload }) => {
      onStatus(payload);
    }).then((unlisten) => {
      this.unlisten = unlisten;
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
    const unlisten = this.unlisten;
    this.unlisten = null;
    if (unlisten) unlisten();
    await this.starting;
  }
}
