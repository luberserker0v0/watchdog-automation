# Action Recorder

Action Recorder observes operator input and converts it into a reviewable list
of existing WatchDog actions. It does not edit, save, or execute a pipeline.

## Start a recording

1. In Capture, select the screen or window that contains the intended actions.
2. For a window target, WatchDog activates and verifies that window at start;
   keep it foreground while recording target input.
3. Open **Action Recorder** and select **Start recording**, or use the configured Start/Stop shortcut.
4. Perform the actions in the target.
5. Use Pause/Resume when the interruption should remain within one session.
6. Stop the recording and review every draft action before copying or exporting it.

The default shortcuts are:

| Control | Default |
|---------|---------|
| Start / Stop | `CommandOrControl+Alt+F9` |
| Pause / Resume | `CommandOrControl+Alt+F10` |

Both are global shortcuts. Settings rejects an empty or duplicate pair, and
registration failure restores the last working pair. Recorder control keys do
not enter the action draft.

Select **Record** beside either shortcut, then press the desired combination.
Existing recorder shortcuts are temporarily unregistered while Settings is
listening. `Escape` cancels. A modifier is required for ordinary letters and
digits; a function key may be used by itself.

## Persistent status bar

The bar is the authoritative recording indicator and remains visible across
Pipeline, Node Test, Patrol Chat Room, Action Recorder, and Settings. It shows:

- recording, paused, or stopping state;
- bound target;
- elapsed time and action count;
- target or recorder error;
- Pause/Resume and Stop controls.

Optional system notifications confirm recorder start and stop. Disabling them
does not hide the bar.

## Draft actions

| Operator input | Draft |
|----------------|-------|
| Short left click | `Click` |
| Left press for at least 600 ms | `LongPress` |
| Drag at least 8 capture pixels | `Swipe` |
| Mouse wheel | `Scroll` |
| Control or non-printable key | `ClickKey` |
| Printable characters within 750 ms | Grouped `InputText` |

Pointer positions are relative to the selected capture area. Input outside that
area is omitted. Window keyboard and completed pointer actions require the
bound HWND to be foreground. If the target disappears or fails that check, the
recorder pauses and explains why.

The review view formats each action with its relative time and warnings. A user
can remove unwanted actions, copy the pipeline action array, export the full
draft, or clear the stopped session.

Ordinary pointer movement is not recorded. Dragging remains a `Swipe` with its
start point, end point, and duration.

Timing uses active recording time, so Pause/Resume does not add a replay gap.
Copying produces an ordered pipeline action array with an explicit
`{"type":"Wait","duration":...}` action for each positive idle interval
between retained actions. The exported version 2 draft preserves the original
offset evidence and includes the same expanded array as `pipeline_actions`.
For `LongPress` and `Swipe`, gesture duration is subtracted when calculating
the preceding idle interval so playback does not count that duration twice.

## Text privacy

**Capture typed text** is off by default. Printable typing then becomes one
`{{recorded_text}}` placeholder instead of storing the characters. When enabled,
the draft stores basic Latin letters, digits, and spaces and displays a review
warning. Passwords and other sensitive values should remain parameters rather
than literal action text.

Keyboard layout and IME composition are not reconstructed in the current
Windows implementation. Review typed drafts before using them.

## Platform status

| Platform | Status |
|----------|--------|
| Windows | Low-level mouse and keyboard hooks implemented |
| macOS | Command boundary present; recording start reports unsupported |
| Linux | Command boundary present; recording start reports unsupported |

Windows input marked as injected is discarded, which prevents WatchDog engine
actions from contaminating a recording.
