import { isTauri } from "@tauri-apps/api/core";
import { BaseDirectory, writeTextFile } from "@tauri-apps/plugin-fs";
import type { RecordedActionDraft } from "../../../api/recorder";
import { toPipelineActions } from "../utils/recordingTiming";

export async function exportRecording(
  actions: RecordedActionDraft[],
  timestamp = Date.now(),
): Promise<string> {
  const filename = `watchdog-action-recording-${timestamp}.json`;
  const content = JSON.stringify(
    { version: 2, actions, pipeline_actions: toPipelineActions(actions) },
    null,
    2,
  );
  if (isTauri()) {
    await writeTextFile(filename, content, { baseDir: BaseDirectory.Download });
    return filename;
  }
  const blob = new Blob([content], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  anchor.click();
  URL.revokeObjectURL(url);
  return filename;
}
