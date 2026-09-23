import type { Action } from "../../../api/pipeline";
import type { RecordedActionDraft } from "../../../api/recorder";

function actionDuration(action: Action): number {
  return action.type === "LongPress" || action.type === "Swipe" ? action.duration ?? 0 : 0;
}

/** Returns idle time after the previous recorded action completed. */
export function recordedDelayBefore(actions: RecordedActionDraft[], index: number): number {
  if (index <= 0 || index >= actions.length) return 0;
  const current = actions[index];
  const previous = actions[index - 1];
  const currentStartedAt = Math.max(0, current.offset_ms - actionDuration(current.action));
  return Math.max(0, currentStartedAt - previous.offset_ms);
}

/** Expands a recording draft into metadata-compatible actions with explicit waits. */
export function toPipelineActions(actions: RecordedActionDraft[]): Action[] {
  const result: Action[] = [];
  actions.forEach((draft, index) => {
    const delay = recordedDelayBefore(actions, index);
    if (delay > 0) result.push({ type: "Wait", duration: delay });
    result.push(draft.action);
  });
  return result;
}
