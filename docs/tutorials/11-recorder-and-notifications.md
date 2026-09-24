# Chapter 11: Record actions and receive notifications

Action Recorder produces an editable draft from native input. Task Notifier
reports pipeline completion and recorder state through system notifications.

## 11.1 Configure shortcuts

Open **Settings → Action Recorder**. Use the shortcut capture control rather
than typing key names manually.

- **Start / Stop shortcut** toggles recording.
- **Pause / Resume shortcut** temporarily excludes input without ending the
  draft.
- The shortcuts must be different.

The persistent recorder bar shows whether recording is active or paused.

## 11.2 Select a recording target

Select a screen or window in Capture before starting. Recorder coordinates are
converted into the selected capture target's coordinate space. On Windows,
native hooks observe keyboard and pointer input and exclude WatchDog-injected
input.

## 11.3 Understand recorded events

The recorder converts gestures into draft actions such as:

- click;
- long press;
- swipe;
- scroll;
- key input;
- optional masked text input.

Ordinary pointer movement is not recorded. The elapsed active time before each
action is retained so spacing can be represented when the draft becomes
pipeline actions.

## 11.4 Review and export

Use the Action Recorder timeline to review, edit, and remove events. Export
writes JSON to the downloads folder listed under Settings **Related details**.
Open that folder from Settings to inspect the file.

An export is a draft. Review coordinates, replace fragile delays with visual
recognition or freeze waits, and decide which events belong in one node.

## 11.5 Protect typed text

Typed-text capture is off by default. Printable input is masked unless the
user explicitly enables capture for non-sensitive data. Never record passwords,
tokens, or personal data into a reusable pipeline.

## 11.6 Configure notifications

Open **Settings → Task notifications** and enable:

- pipeline completion;
- recorder state.

Use **Send test notification** to verify system delivery. Development-mode
Windows notifications may display the PowerShell host identity; an installed
application bundle supplies the proper application identity and icon.

For complete recorder behavior, see the
[Action Recorder reference](../action-recorder.md).

Return to the [tutorial index](README.md).
