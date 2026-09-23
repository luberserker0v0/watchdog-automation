import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";

export type TaskNotificationType =
  | "pipeline_completed"
  | "recording_started"
  | "recording_stopped";

export interface TaskNotification {
  id: string;
  type: TaskNotificationType;
  title: string;
  body: string;
}

const delivered = new Set<string>();
const inFlight = new Map<string, Promise<boolean>>();

export async function ensureNotificationPermission(): Promise<boolean> {
  try {
    if (await isPermissionGranted()) return true;
    return (await requestPermission()) === "granted";
  } catch {
    return false;
  }
}

export async function notifyTask(notification: TaskNotification): Promise<boolean> {
  if (delivered.has(notification.id) || inFlight.has(notification.id)) return false;
  const delivery = (async () => {
    if (!(await ensureNotificationPermission())) return false;
    try {
      sendNotification({ title: notification.title, body: notification.body });
      delivered.add(notification.id);
      return true;
    } catch {
      return false;
    }
  })();
  inFlight.set(notification.id, delivery);
  try { return await delivery; }
  finally { inFlight.delete(notification.id); }
}

export function resetTaskNotifierForTests(): void {
  delivered.clear();
  inFlight.clear();
}
