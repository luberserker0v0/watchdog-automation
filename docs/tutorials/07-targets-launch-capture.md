# Chapter 7: Select and launch capture targets

Recognition coordinates and input safety depend on a stable target. Pipelines
can use a screen or a window.

## 7.1 Screen target

```json
"target": {
  "type": "screen",
  "index": 0
}
```

Screen actions are allowed when the active capture context matches that screen.
Coordinates are relative to the captured screen and account for monitor origin.

## 7.2 Window target

```json
"target": {
  "type": "window",
  "title": "^Target UI$",
  "capture_area": "client",
  "window_size": [500, 600]
}
```

- `title` is a regular expression.
- `capture_area` selects the client area or full window frame.
- `window_size` describes the required client size.

For every input action, the engine attempts to foreground the target and then
verifies that the exact window is foreground. If verification fails, input is
blocked rather than sent elsewhere.

Virtual desktops only filter the window list. Pipelines do not switch desktops.

## 7.3 Launch the target

```json
"launch": {
  "exe": "watch-dog-target.exe",
  "args": ["dashboard"],
  "wait_for_window": true,
  "wait_timeout_ms": 10000
}
```

WatchDog launches the process, waits for a matching target window, and then
acquires capture ownership. A launch configuration should be specific enough
to avoid selecting another window with a similar title.

## 7.4 Placement and cleanup

When WatchDog changes window placement to satisfy a requested size, it records
the original placement and attempts to restore it during cleanup. Windows may
refuse restoration; that failure is reported separately from the pipeline
execution result.

## 7.5 Diagnose target problems

- Confirm the title expression matches one intended window.
- Confirm client versus frame capture agrees with node coordinates.
- Stop manual capture before starting a pipeline that needs ownership.
- Check the requested client size.
- Use the Capture view to verify the exact target before testing actions.

Next: [Reuse nodes and parameters](08-reuse-and-parameters.md).
