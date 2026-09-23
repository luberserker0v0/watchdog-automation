import { useEffect, useRef } from "react";
import { usePreferencesStore } from "../../shell/stores/usePreferencesStore";
import { configureRecorderHotkeys, releaseRecorderHotkeys } from "../services/recorderHotkeys";
import { useRecorderStore } from "../stores/useRecorderStore";

export default function RecorderCoordinator() {
  const startStop = usePreferencesStore((state) => state.recorderStartStopHotkey);
  const pause = usePreferencesStore((state) => state.recorderPauseHotkey);
  const setStartStop = usePreferencesStore((state) => state.setRecorderStartStopHotkey);
  const setPause = usePreferencesStore((state) => state.setRecorderPauseHotkey);
  const initialize = useRecorderStore((state) => state.initialize);
  const setHotkeyError = useRecorderStore((state) => state.setHotkeyError);
  const shortcutCaptureActive = useRecorderStore((state) => state.shortcutCaptureActive);
  const lastGood = useRef({ startStop, pause });

  useEffect(() => { void initialize(); }, [initialize]);
  useEffect(() => {
    let active = true;
    if (shortcutCaptureActive) {
      void releaseRecorderHotkeys();
      return () => { active = false; };
    }
    void configureRecorderHotkeys(startStop, pause)
      .then(() => { if (active) { lastGood.current = { startStop, pause }; setHotkeyError(null); } })
      .catch((error) => {
        if (!active) return;
        setHotkeyError(error instanceof Error ? error.message : String(error));
        if (lastGood.current.startStop !== startStop) setStartStop(lastGood.current.startStop);
        if (lastGood.current.pause !== pause) setPause(lastGood.current.pause);
      });
    return () => { active = false; void releaseRecorderHotkeys(); };
  }, [pause, setHotkeyError, setPause, setStartStop, shortcutCaptureActive, startStop]);
  return null;
}
