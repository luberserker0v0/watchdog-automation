# Chapter 3: Configure pipeline metadata

The metadata file supplies target selection, launch behavior, reusable node
definitions, instances, and parameters. It is validated against the bundled
JSON schema when loaded, saved, and started.

## 3.1 Start with the top-level shape

```json
{
  "target": {
    "type": "window",
    "title": "^My Application$",
    "capture_area": "client"
  },
  "launch": {
    "exe": "my-application.exe",
    "wait_for_window": true
  },
  "nodes": {},
  "instances": {}
}
```

`target` and `launch` are optional when the user will select an existing
capture target manually. `nodes` and `instances` describe pipeline behavior.

## 3.2 Define a normal node

```json
{
  "nodes": {
    "ClickLogin": {
      "desc": "Click the visible Login button.",
      "recognition": {
        "type": "OCR",
        "expected": "Login",
        "threshold": 0.6
      },
      "actions": [
        { "type": "Click" }
      ],
      "timeout": 10000,
      "rate_limit": 500,
      "max_hit": 1
    }
  }
}
```

`desc` helps humans and Patrol understand intent. Recognition decides whether
the node matches. Actions run only after a match.

## 3.3 Connect graph IDs with instances

```json
{
  "instances": {
    "LoginButton": {
      "node": "ClickLogin"
    }
  }
}
```

Here, `LoginButton` is the Mermaid graph ID and `ClickLogin` is the reusable
definition. When IDs and definition names are identical, the mapping is still
useful because it makes the source relationship explicit.

## 3.4 Keep concerns in the correct file

| Concern | File |
| --- | --- |
| Nodes and edges | `.md` |
| Recognition, actions, timing, target, launch | `.meta.json` |
| Node positions | `.layout.json` |
| Patrol enablement and VLM credentials | Settings View |

Patrol configuration inside pipeline metadata is rejected. This keeps runtime
policy under user control.

## 3.5 Read diagnostics

A validation error identifies the source file and field path. Fix the source
rather than deleting unknown properties until the file happens to load. The
complete field catalog is in the
[metadata reference](../pipeline-meta.md).

Next: [Recognize visual states](04-recognition.md).
