# Chapter 5: Execute actions and control timing

Actions run in array order after recognition succeeds. For window targets,
WatchDog activates and verifies the target window immediately before each
input action.

## 5.1 Add common actions

Click the center of the recognized region:

```json
"actions": [{ "type": "Click" }]
```

Move the click relative to that region:

```json
"actions": [{
  "type": "Click",
  "target_offset": [35, 20, 0, 0]
}]
```

Enter text after clicking an input:

```json
"actions": [
  { "type": "Click", "target_offset": [30, 30, 0, 0] },
  { "type": "InputText", "input_text": "{{account}}" }
]
```

Other built-in types include `LongPress`, `Swipe`, `Scroll`, `ClickKey`,
`Wait`, `DoNothing`, and `StopTask`.

## 5.2 Control node timing

```json
{
  "pre_delay": 100,
  "post_delay": 300,
  "rate_limit": 500,
  "timeout": 10000,
  "repeat": 1,
  "max_hit": 1
}
```

- `pre_delay` waits after recognition and before actions.
- `post_delay` waits after actions, including after an action error.
- `rate_limit` controls recognition retry cadence.
- `timeout` bounds the node attempt.
- `repeat` repeats the complete action list.
- `max_hit` limits successful visits in one execution epoch.

Delays are cancellable and pause-aware.

## 5.3 Wait for the screen to settle

Use freeze waits when a fixed delay is unreliable:

```json
{
  "pre_wait_freezes": {
    "time": 500,
    "timeout": 5000,
    "threshold": 0.98
  },
  "post_wait_freezes": {
    "time": 800,
    "timeout": 8000,
    "threshold": 0.98
  }
}
```

The wait completes only after the screen stays sufficiently similar for the
configured duration. `pre_wait_freezes` protects actions from loading phases;
`post_wait_freezes` gives routing and Patrol a settled after-frame.

## 5.4 Preserve recorded timing

Action Recorder exports an elapsed delay before each recorded action. Keep it
when the real interaction requires spacing, then replace fragile fixed waits
with recognition or freeze waits where possible.

Next: [Route failures and complete runs](06-routing-and-lifecycle.md).
