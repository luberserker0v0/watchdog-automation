# Pipeline graph layout

WatchDog stores shareable graph positions separately from pipeline execution metadata.

## File set

A pipeline uses two required files and one optional layout file. All files use the same base name and live in the same directory.

| File | Required | Purpose |
| --- | --- | --- |
| `<name>.md` | Yes | Mermaid nodes, labels, and edges |
| `<name>.meta.json` | Yes | Target, recognition, actions, parameters, timing, and edge behavior |
| `<name>.layout.json` | No | Visual node positions in the Pipeline graph |

The layout file changes presentation only. It is not read by the execution engine when deciding which node to recognize or execute.

## Format

```json
{
  "nodes": {
    "InputAccount": { "x": 120, "y": 80 },
    "InputPassword": { "x": 120, "y": 180 }
  }
}
```

Each key must exactly match a node ID in the adjacent Mermaid and metadata files. `x` and `y` are signed integer pixel coordinates for the node's top-left corner in graph space.

- A missing node uses automatic placement.
- An empty `nodes` object uses automatic placement for the entire graph.
- Unknown node IDs make the pipeline layout invalid.
- Extra top-level fields and extra position fields are rejected.

VS Code validates `*.layout.json` through [`schemas/watchdog-layout.schema.json`](../schemas/watchdog-layout.schema.json).

## Editing in WatchDog

Open a custom pipeline and enable **Edit layout** in the graph toolbar. Dragging a node updates the saved position map in `<name>.layout.json`; nodes without saved positions continue to use automatic placement.

- **Region select** selects several nodes so they can move together.
- **Grid** displays a visual alignment grid; it does not change saved positions.
- **Auto align** saves `{ "nodes": {} }` and returns every node to automatic placement.
- **Show/Hide minimap** is a personal display preference and is not written into the layout file.

If writing fails, WatchDog restores the previous in-memory layout and displays the error in the graph toolbar.

## Bundled and custom pipelines

Bundled pipelines under `src-tauri/resources/pipelines/` are read-only at runtime. To customize and share one, copy its complete directory to the Tauri AppLocalData resource root shown in **Settings → Related Folders**:

```text
AppLocalData/resources/pipelines/
```

Custom pipelines in that directory may create and update their adjacent layout file.

When sharing a pipeline, include:

```text
<name>.md
<name>.meta.json
<name>.layout.json   # when a custom graph arrangement is desired
```

Template images and other referenced resources must also be included in their corresponding resource directories.
