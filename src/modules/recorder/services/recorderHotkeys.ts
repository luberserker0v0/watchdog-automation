import { register, unregister } from "@tauri-apps/plugin-global-shortcut";
import { useRecorderStore } from "../stores/useRecorderStore";

let registered: string[] = [];
let operation: Promise<void> = Promise.resolve();

async function releaseNow(): Promise<void> {
  const current = registered;
  registered = [];
  await Promise.all(current.map((shortcut) => unregister(shortcut).catch(() => undefined)));
}

function serialize(work: () => Promise<void>): Promise<void> {
  const next = operation.then(work, work);
  operation = next.catch(() => undefined);
  return next;
}

export async function configureRecorderHotkeys(startStop: string, pause: string): Promise<void> {
  return serialize(async () => {
    await releaseNow();
    if (!startStop.trim() || !pause.trim()) throw new Error("Both recorder shortcuts are required.");
    if (startStop.trim().toLowerCase() === pause.trim().toLowerCase()) throw new Error("Start/Stop and Pause/Resume shortcuts must be different.");
    try {
      await register(startStop, (event) => {
        if (event.state === "Pressed") void useRecorderStore.getState().toggle();
      });
      registered.push(startStop);
      await register(pause, (event) => {
        if (event.state === "Pressed") void useRecorderStore.getState().togglePause();
      });
      registered.push(pause);
    } catch (error) {
      await releaseNow();
      throw error;
    }
  });
}

export async function releaseRecorderHotkeys(): Promise<void> {
  return serialize(releaseNow);
}
