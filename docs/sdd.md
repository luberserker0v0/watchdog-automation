# Software Design Description — WatchDog

- **Product:** watch-dog (`0.1.0`)
- **Status:** observed implementation design; source-inspected, not system-certified
- **Audit basis:** working tree inspected and automated checks run 2026-09-23; not a release baseline
- **Requirements source:** `docs/srs.md` (candidate `REQ-*` and inferred `STK-*` IDs)
- **File contracts:** `docs/pipeline-meta.md`, `docs/pipeline-layout.md`, `schemas/`

The SRS Appendix D assigns responsibility to capabilities. This SDD maps them
to components, dependencies, data, and interfaces.

| Read this section for | Location |
|-----------------------|----------|
| System boundary and executables | §1 |
| Component ownership and dependencies | §2–3 |
| Data and IPC contracts | §4 |
| Failure behavior and verification limits | §5–6 and SRS §9 |

A requirement allocation does not imply an acceptance test passed. SRS §9
records implementation evidence and outstanding native verification.

## 1. System context

Production and tester are React/WebView frontends connected to a Rust/Tauri
backend through commands, events, and the `watchdog:///frame` protocol. The
sample target is a separate executable for tests and demos. The backend reads
pixels through OS capture APIs and sends mouse/keyboard input through the OS.
Pipeline files and preferences are local data.

`src-tauri/src/lib.rs` registers commands, the frame protocol, and shared
state. `src/App.tsx` and `src/main-target.tsx` compose the frontend windows.
These roots wire capabilities together; they do not own feature behavior.

## 2. Components and dependency rules

Backend dependencies flow from consumers toward providers:

```text
core ← capture ← authoring ← plugins ← runtime ← testing
           ↑           ↑           ↑         ↑
           └───────────┴───────────┴─────────┘
```

The diagram shows allowed direction, not mandatory imports. `core/` must
not import feature modules. Capture owns target identity, capture area, stream
frames, and physical frame bounds. Authoring uses capture target types.
Plugins use authoring configuration and capture context. Runtime coordinates
those providers; testing reuses the registry with its own action execution
path and the same foreground rule for window targets (§3.3). `lib.rs` composes
the application.

The frontend has a separate composition boundary. `src/api/` wraps IPC;
`src/modules/shell/` owns navigation, preferences, and the small runtime
integration contract consumed by capture UI. `src/modules/observability/`
owns the shared execution log. Feature views consume those contracts instead
of importing pipeline state into capture or testing composition.

| Capability | Primary files | Provides | Consumes |
|------------|---------------|----------|----------|
| CAP | `src/modules/capture/`; `src-tauri/src/modules/capture/` | Screens, windows, desktops, `StreamService`, `StreamFrame`, `StreamTarget`, `PipelineTarget`, `CaptureArea`, `CaptureBounds` | Core and OS capture/window APIs |
| AUTHOR | `src/modules/pipeline/authoring/`; backend `modules/pipeline/authoring/` | Parsers, merged graph, validation, load/save | Capture target types, core paths |
| REC, ACT | Backend `modules/pipeline/plugins/` | Plugin registry, ROI evaluation, action mapping and execution | Authoring node config, capture frame/bounds |
| PIPE | `src/modules/pipeline/runtime/`; backend `modules/pipeline/runtime/` | Plan, run lifecycle, status, step evidence, runtime UI | Authoring, capture, plugins |
| TEST | `src/modules/testing/`; backend `modules/testing/` | Node tests, emulator, batch workflow | Capture, authoring, plugins, runtime |
| OBS | `src/modules/observability/`; backend runtime evidence | Shared execution log, events, snapshots, progress, debug images | Run and capture state |
| SET | `src/modules/settings/`; `src/modules/shell/stores/usePreferencesStore.ts`; `src/i18n/` | Persisted preferences, session flags, and English/Traditional Chinese localization | Browser storage, stream status, i18next |
| NOT | `src/modules/notifications/`; Tauri notification plugin | Permission-aware, task-ID-deduplicated native notifications | Terminal pipeline status, recorder state, persisted opt-in |
| RCD | `src/modules/recorder/`; backend `modules/recorder/` | Recorder state machine, Windows input hooks, review drafts, global controls | Capture selection and bounds, foreground state, settings |

`src/modules/sample_target/` contains the sample target UI;
`src/main-target.tsx` mounts it. Shell layout, shared utilities, and the sample
target support SRS capabilities without receiving
new requirement prefixes.

### 2.1 Main-app startup maintenance

`lib.rs::run_app` creates user directories and initializes the logger before
calling `core/maintenance.rs` and setting up Tauri. The tester and sample
target startup paths do not call maintenance.

| Data | Rule | Failure behavior |
|------|------|------------------|
| Dated application logs | Retain the 30 newest, including the current log. | Log the error; continue startup. |
| Generated visions | On first startup without a marker, or at least seven days after successful cleanup, remove generated contents and recreate the debug directory. | Log the error; continue startup. Update the marker only after success. |

Cleanup is confined to canonical user-data logs and visions directories and
does not follow symbolic links.

## 3. Component design

### 3.1 Capture (CAP)

Backend `capture/repositories/` wraps screen, window, Win32, and virtual
desktop APIs. `capture/services/stream.rs` is the `StreamService` facade;
`capture/services/stream/models.rs` owns frame and target models and
`capture/services/stream/ownership.rs` owns typed `StreamOwner` arbitration.
`capture/services/bounds.rs` resolves a captured screen or window to physical
screen bounds. Commands expose enumeration, one-shot capture, stream control,
resize, and desktop operations.

Ownership is `none`, `manual`, or `pipeline` plus run ID. A manual stream
blocks pipeline acquisition; a pipeline blocks manual acquisition and other
pipelines. Release checks owner kind and, for pipelines, run ID. Each
`StreamFrame` carries a session ID, sequence, capture time, encoded bytes,
dimensions, target, and optional bounds. Runtime rejects stale frames using
session, sequence, and time boundaries.

### 3.2 Authoring (AUTHOR)

`parser/mermaid.rs` parses topology; `parser/meta.rs` parses templates,
instances, target, and launch data; `parser/layout.rs` parses positions.
`parser/mod.rs` uses an explicit instance's referenced template when one is
present. Without an instance it looks up the Mermaid label and then the node
ID. It applies supported instance overrides to the template to build
`PipelineGraph`. Instance parameter definitions enter the graph with
`param_values` applied as visible defaults. At run start, values resolve from
definition defaults, then instance values, then explicit caller values.

`editor.rs` checks source coverage and graph rules when loading.
`save_pipeline(path, mermaid, meta)` validates and writes original source text,
retaining launch settings and template/instance structure; a legacy graph-only
call receives an error. Both pipeline and layout saves are confined to the
canonical user pipeline root, with symlink escape checks. Layout remains
presentation data and never enters the execution plan. Metadata and layout
parsers check the bundled Draft 2020-12 schemas before Serde conversion and
custom graph validation.

### 3.3 Recognition and actions (REC, ACT)

`plugins/mod.rs` registers `RecognizerPlugin` and `ActionPlugin` types.
Recognizers evaluate a supplied image and recognition configuration.
`plugins/shared/coordinates.rs` resolves ROI and action geometry using capture-owned
bounds. The pipeline engine and isolated node tests restore and verify a
window target's foreground state just before each registered action. If Windows
does not grant foreground ownership, they refuse input with
`target_not_foreground`. Standalone `test_action` requires a screen or window
context matching the active stream; window contexts get another foreground
check immediately before dispatch.

### 3.4 Runtime (PIPE)

#### 3.4.1 Plan, startup, and completion

`runtime/plan.rs` compiles one Entry and indexed next/error routes into an
`ExecutionPlan`. `runtime/runner.rs` prepares a run in this order:

1. Parse and validate source files, then compile routes.
2. Resolve or launch the target and prepare window placement.
3. Acquire capture ownership, start and verify the stream, then publish the run handle.

Startup failures and run completion attempt stream release and window
restoration. A failed startup restoration extends the start diagnostic.
Pipeline graphs contain no desktop selector; desktop enumeration filters only
the capture UI's window list.

Authoring requires at least one Endpoint and permits several. Endpoints have
no recognition, actions, or outgoing edges. The engine records a matched step
there without capture or input. A normal leaf creates an impending failure
with its node ID in the reason. Enabled Patrol performs final validation and
the configured recovery task before the engine either resumes an accepted
redirect or publishes failed status. `RunHandle` retains state, history, and a monotonic revision for 60
seconds after completion so the frontend can poll the final status.

#### 3.4.2 Node execution and freeze waits

`runtime/engine/` tries children within the parent's quota and manages error
routes, recognition retries, action repeats, pause/stop, and `StepResult`
evidence. The node's input path follows this sequence:

| Order | Operation | On failure or interruption |
|-------|-----------|----------------------------|
| 1 | Optional `pre_wait_freezes` on the active capture target/session. | Timeout or stop ends the wait; failure follows the node's error handling. |
| 2 | Cancellable `pre_delay`; fresh-frame recognition and action-target mapping. | Failed refreshed recognition follows the error route. |
| 3 | Ordered action list, repeated with configured pacing. | Action error is recorded; window input is guarded immediately before dispatch (§3.3). |
| 4 | Optional `post_wait_freezes` after dispatched input. | Timeout follows node error handling. |
| 5 | Cancellable `post_delay`. | Runs after action attempts, including action or post-wait errors. |

`engine/freeze.rs` compares downsampled frames, optionally inside a
capture-local ROI. Visible change resets its stable-duration timer. Paused time
does not count, and a separate timeout bounds active waiting.

#### 3.4.3 Patrol scheduling and VLM tasks

The Settings View supplies patrol enablement, cycle seconds (default 20), K
hops, score threshold, minimum margin, recovery policy, and redirect limit to
`start_pipeline`. Patrol settings
are absent from pipeline metadata; a legacy `patrol` field receives a field
diagnostic. Invalid or unavailable configuration is logged without preventing
the pipeline from starting.

| Phase | Behavior |
|-------|----------|
| Periodic start | `engine/runner.rs` starts a worker with the run. The first sample is due one configured cycle after start. |
| Later samples | The worker waits for the previous response and for one cycle since that request began. A response slower than the cycle allows the next request immediately. At most one VLM request is in flight. |
| Task selection | A completed, sent, unresolved action attempt with a newer frame on the same target/session selects Action Effect Verifier. Otherwise the sample selects Current State Matcher. |
| Current State Matcher | `runtime/patrol.rs` scores the latest frame against the expected node and undirected K-hop neighbors. It receives no action history. A request includes at most 16 candidates; missing references use descriptions. An inconclusive local result expands to 2K hops and shared-parent siblings. |
| Action Effect Verifier | Compares one action batch's exact pre-dispatch image and a newer image with its ordered sanitized actions, direct normal-node successor descriptions/references, and the declared error successor when visual. The VLM returns the explicit classes `expected_successor`, `error_successor`, `unchanged`, `in_progress`, or `inconclusive`. Runtime reports map the first two to `achieved` and `unexpected`; matching the error successor is never treated as achievement. |
| Recovery task | Runs only at an impending terminal failure after the final action/state check. Hint policy requests one advisory candidate. Redirection policy requests one route candidate. Both receive failure context, recent path, error-handler before/current frames and sanitized actions, and at most 16 engine-selected normal nodes with descriptions, sanitized actions, and available references. |
| Report | State results use `aligned`, `possible_mismatch`, or `inconclusive`. Action results use the five effect values. Both include run, task, target, capture session, and frame sequence and go to the Execution Log and run evidence. |

Each request performs one task with a dedicated system prompt and response
type. The VLM HTTP request has a separate 120-second timeout. Analysis runs while the
pipeline continues recognition, actions, and routing, so a periodic report can arrive
after node advancement. A report, model error, or reference save failure does
not change routing, block input, or fail the run. Only the failure-boundary
Recovery Route Selector may propose a redirect; the engine accepts or rejects
that proposal.

Both task responses receive strict JSON and semantic validation. The parser
accepts an optional outer Markdown JSON fence, then rejects unknown fields,
unknown or duplicate IDs, missing candidates, non-finite or out-of-range
scores, unknown effects, and invalid successor combinations. One invalid
response causes one same-task request with the original inputs plus the
validation error. A second invalid response produces an inconclusive advisory
result. The Chat Room retains both exact responses and the validation error.

After a successful node visit, a background worker saves an unannotated full
target frame. `ReferenceStore` places it under a stable pipeline hash and the
run ID. It reads only that run's references; a new run uses descriptions until
its own visits produce images. No baseline run is needed.

Before terminal status, the engine stops periodic sampling, cancels unfinished
periodic VLM work, and waits for reference saves. On failure it also attempts
one final validation and waits for its report. It verifies the latest unresolved
action when usable action evidence exists; otherwise it performs state matching. The check prefers a newer
cached frame in the same capture session; if no frame exists, it logs that
validation was unavailable. The run then closes an atomic report gate before
publishing terminal status. No Patrol report follows terminal status.

#### 3.4.4 Patrol evidence and run status

`engine/patrol_state.rs` holds bounded evidence for each run:

| Evidence | Recorded facts |
|----------|----------------|
| Capture | Node, target, session, frame sequence, and recognizer result. |
| Input dispatch | Sanitized intent, source frame, action-attempt ID, and dispatch outcome. Successful dispatch starts with an unverified visual effect. Allowlisted control keys use names; text and arbitrary keys remain redacted. |
| Action attempt | Source node, ordered input IDs, direct visual successors, before/after frame identities, timing, and effect result. One node visit forms one attempt. |
| Action images | Exact before image and newer after image are retained only in a bounded private worker cache and the session preview cache, never in pipeline status. |
| State decision | Anchor, possible matched node, capture identity, outcome, and reason. State decisions never confirm action effects. |
| Recovery decision | Policy, failure node, proposed target, confidence, reason, validation result, and accepted redirect count. |

Pipeline status exposes this ledger. Text input, arbitrary character keys, and
custom parameters are excluded from status and logs. Allowlisted non-text
control keys use semantic names. An action becomes `achieved` only when the verifier returns a declared
direct normal-node successor using a newer frame from the same target/session.
`in_progress` and parse-level `inconclusive` attempts may be sampled again with
a newer frame. Endpoint successors have no visual state, so an endpoint-only
transition creates no action-effect request; authors use a normal verification
node before Endpoint when visual confirmation is required.

### 3.5 Testing, observability, and settings (TEST, OBS, SET)

#### 3.5.1 Testing

`modules/testing/node_test.rs` is the command facade;
`modules/testing/node_test/metadata.rs` discovers and loads metadata. The
facade tests isolated recognition
or action against a supplied or cached frame. The frontend emulator validates
draft fields. The batch controller runs selected pipelines sequentially.

#### 3.5.2 Run evidence

| Channel | Producer and consumer |
|---------|-----------------------|
| `pipeline-status` | Runtime sends revisioned snapshots and appended steps. The frontend shows the latest ten steps; backend handles retain full history. |
| `patrol-report` | Runtime sends independent advisory reports. The Execution Log shows a concise state transition; technical capture identity and exact VLM reasoning remain in the Chat Room. |
| `patrol-recovery` | Runtime reports a hint, accepted redirect, or rejected redirect before terminal status. The frontend renders a concise recovery path and decision with a persistent `HINT ONLY` or `REDIRECT` policy tag. |
| Debug images and timings | Engine produces these when the corresponding options are enabled. |
| Cleanup fields | Status and events carry `cleanup_complete` and optional `cleanup_error`. The frontend follows a terminal run until cleanup finishes and shows restoration failure separately. |

The frontend remembers terminal run IDs and discards cross-channel Patrol
events delivered after terminal status has already been observed.

#### 3.5.3 Patrol Chat Room

An app-level listener collects `patrol-chat-event` messages before runs start
and while the room is closed. Each structured event carries run/sample IDs,
capture identity, sender, operation kind, task tier, exact prompt text parts,
exact response text, validation diagnostics, score rows, sanitized actions,
and before/after/reference image IDs. It excludes the API key and image base64.

| View behavior | Contract |
|---------------|----------|
| Layout | VLM messages on the left; Patrol operations on the right. Details are closed by default. |
| Session controls | Run selector, auto-scroll, and clear-session action for bounded in-memory history. |
| Previews | Backend caches images by opaque ID. Expansion fetches a preview; eviction displays an unavailable state. |
| Effect | Trace events only describe activity; they cannot change routes or dispatch actions. |

#### 3.5.4 Settings persistence

Preferences persist in `watchdog:settings`. Save-to-disk and
save-debug-images are session-only and rehydrate as off. The VLM endpoint and
model name persist; the API key remains in memory for the app session, is
passed to `start_pipeline` only when a run starts, and is omitted from stored
preferences and logs.

### 3.6 Task notification and action recording (NOT, RCD)

`taskNotifier.ts` is the only frontend adapter for native notifications. It
checks or requests permission, deduplicates messages by stable task ID, and
contains delivery failures. Pipeline completion uses `pipeline:<run_id>` only
after status is terminal and cleanup is complete, so live-event and polling
paths converge on one notification. Recorder start and stop use session IDs and
an independent opt-in.

The recorder backend owns one `RecorderService` in Tauri managed state. Its
platform-neutral commands expose status, start, pause, resume, stop, and clear.
On Windows, a dedicated message-loop thread installs `WH_MOUSE_LL` and
`WH_KEYBOARD_LL` hooks. Hook callbacks only copy small input records into a
channel; a processor thread performs bounds checks, grouping, state mutation,
and event publication. `LLMHF_INJECTED` and `LLKHF_INJECTED` observations are
discarded.

Each session binds to one `RecordingTarget`. Screen bounds come from monitor
enumeration. Start activates and verifies a selected window so the in-app Start
button is usable. Window bounds reuse capture-area mapping and require the exact
foreground HWND before keyboard or committed pointer actions. A missing or
non-foreground target pauses the session and emits an explicit reason.

| Input sequence | Draft mapping |
|----------------|---------------|
| Short left press | `Click` with capture-relative coordinate |
| Left press of at least 600 ms | `LongPress` with observed duration |
| Pointer displacement of at least 8 px | `Swipe` with endpoints and duration |
| Wheel message | `Scroll` at the relative pointer location |
| Control or non-printable key | `ClickKey` with held modifiers |
| Printable keys within 750 ms | One `InputText`; masked unless capture was explicitly enabled |

The frontend registers two distinct global shortcuts and restores the last
working pair if registration fails. `RecordingStatusBar` stays mounted above
every main view. `ActionRecorderView` formats the draft and permits removal,
copy, export, and clear; it never mutates or executes a pipeline. macOS and
Linux retain the command model and report unsupported at start.

## 4. Data and interface contracts

| Contract | Owner | Invariant |
|----------|-------|-----------|
| Mermaid `.md` + `.meta.json` | AUTHOR | Exactly one Entry and at least one Endpoint; metadata resolves for each node, including optional pre/post freeze waits; validation precedes execution |
| Optional `.layout.json` | AUTHOR | Positions cannot change routes or actions |
| `PipelineTarget`, `CaptureArea` | CAP | One target and capture-area representation across layers |
| `StreamFrame`, `CaptureBounds` | CAP | Bounds map encoded pixels to physical pixels; session/sequence identify freshness |
| `ExecutionPlan` | PIPE | Entry and routes are compiled before capture-driven execution |
| `PipelineStatus`, `StepResult`, `PatrolState` | PIPE | Revision increases per run; terminal outcomes carry evidence and cleanup completion/error; status exposes bounded patrol evidence |
| `patrol-report` event | PIPE / OBS | A completed background sample carries task and optional action-attempt identity, run/capture identity, task-specific outcome, match, and reason; the frontend formats it independently in the Execution Log |
| `patrol-chat-event`, image cache commands | PIPE / OBS | Ordered session trace contains formatted text and image IDs, excludes credentials and inline image data; previews load on demand |
| `ValidationError` | AUTHOR | Type and message distinguish authoring failures |
| `RecorderStatus`, `RecordingSession`, `RecordedActionDraft` | RCD | One session at a time; actions use capture-relative coordinates and existing action types; drafts remain observational until copied or exported |
| Native task notification | NOT | A stable event ID produces at most one delivery after permission; failure cannot alter pipeline or recorder state |

The Tauri command list in `src-tauri/src/lib.rs` implements SRS
`REQ-IF-010`–`017` and `REQ-IF-022`; `src/api/` is its frontend wrapper. Events include
`pipeline-status`, `patrol-report`, `patrol-recovery`, `patrol-chat-event`,
`recorder-status`, `recorder-action`, `recorder-error`, and `update_capture`
(`REQ-IF-018/019/022`), and the frame
protocol is `watchdog:///frame` (`REQ-IF-020`). Schemas under `schemas/`
describe authoring files. OS interfaces stay behind capture repositories and
action plugins.

## 5. Failure and recovery

- Authoring failures return validation errors before a run is registered.
- Ownership conflicts reject a start without replacing the current stream.
- Startup and run-end cleanup attempt to restore window placement and release
  capture. Startup diagnostics include restore failures; completed runs expose
  them through status and events.
- Window-target action paths try to activate the target before input and refuse
  input if that window cannot be verified foreground immediately before dispatch.
- Polling recovers missed status events; the UI discards stale revisions.
- At an impending terminal failure, Patrol completes the final action or state
  check and then runs the configured recovery task. `hint_only` emits an
  advisory suggestion and preserves failure. `redirection_only` can resume the
  same run at one engine-generated candidate when confidence, capture identity,
  node validity, redirect limit, and route-deduplication checks pass.
- An accepted redirect clears failure and child state, resets hit quotas,
  resets the node-attempt and timeout epoch, retains step history and the run
  ID, restarts periodic Patrol timing, and sends the selected node through
  ordinary recognition and action safety. Parameters, capture identity,
  redirect count, and accepted-route history remain intact.
  Rejection or an invalid VLM response preserves the original failure.

## 6. Traceability and verification

SRS Appendix D is the responsibility map; §2 maps it to code. SRS §7 is a
candidate functional contract; SRS §9 records inspected alignment and
verification limits. SRS Appendix F starts the REQ → test-case matrix for
high-risk examples; complete STK → REQ → design → test traceability remains
open. Current passing checks are `npm run build`, `npm test`, `cargo check`,
`cargo test --no-run`, and `cargo test`. Windows capture, real
foreground input, OCR, VLM action classification, hint recovery, and accepted
redirection have live executable-level evidence. Window restoration refusal,
malformed-response retry, redirect-rejection branches, installed notification
delivery, and global input recording still need live
executable-level verification.

## 7. Design decisions

- Capture owns target and frame-bound types because it produces frames and
  resolves their physical coordinates. Authoring re-exports target types for
  compatibility with existing graph and IPC consumers.
- Core owns the user-data root name and never imports a feature module.
- Runtime constructs the plugin registry; isolated tests reuse the same
  registrations and plugin behavior.
- The SRS maps capabilities; this document records code placement.
- `src/api/pipeline.ts` is a compatibility barrel. Focused authoring, runtime,
  Patrol model, and testing contracts live below `src/api/pipeline/`.
- `runtime/patrol.rs` exposes the runtime contract. Configuration, reference
  storage, search, strict response protocol, and VLM transport remain in
  capability-local modules below `runtime/patrol/`.
- `runtime/engine/runner.rs` remains the `PipelineRunSession` aggregate and
  `run_pipeline` facade. Its Patrol worker, target acquisition, and parameter
  substitution are private modules below `runtime/engine/runner/`.
- Feature stylesheet entry files contain ordered imports. Component rules live
  in ordered partials so the existing cascade stays deterministic.

## 8. Open issues

- The source-preserving `save_pipeline` IPC shape differs from the retired
  graph-only shape. Existing external callers must pass Mermaid and metadata
  source text.
- Existing user files outside the bundled JSON Schema constraints now fail
  at load/start with a field diagnostic; they require manual correction.
- `build.rs` delay-loads `comctl32.dll`. The unit-test harness can start
  without resolving Tauri's unused `TaskDialogIndirect` import against legacy
  Common Controls; the application still uses Tauri's embedded activation
  manifest. All enabled library tests now execute.

### 8.1 Source-size audit

The refactor uses 500 lines as a review signal rather than a compiler limit.
The following production coordinators remain above that signal and need later
behavior-preserving extraction. None is classified as a static-data exception.

| Coordinator | Current responsibility to split next |
|-------------|--------------------------------------|
| `runtime/engine/runner.rs` | Node attempt state machine and terminal cleanup inside `PipelineRunSession::run` |
| `testing/node_test.rs` | Target preparation, restoration guard, isolated execution, and result formatting |
| `runtime/engine/debug_image.rs` | Overlays, action symbols, information panels, and font data |
| `runtime/runner.rs` | Startup, registry construction, controls, and standalone tests |
| `capture/services/stream.rs` | Frame/session lifecycle and platform capture loop |
| `pipeline/authoring/components/MermaidGraph.tsx` | Viewport, dragging, selection, minimap, and layout persistence |
| `pipeline/runtime/stores/usePipelineStore.ts` | State transitions, event handling, coordination, and notification adapters |
| `sample_target/CombinedPage.tsx` | Login, dashboard, settings, and recovery lab sections |

The large CSS entry files were replaced by ordered partials. One responsive
capture partial remains slightly above the signal because its enclosing media
query is one cohesive cascade block; it is a style-only exception.
