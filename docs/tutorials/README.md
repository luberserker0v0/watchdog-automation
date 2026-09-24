# WatchDog tutorials

This learning path starts with a two-node pipeline and builds toward reusable,
tested automation with visual recovery. Read the chapters in order when you
are new to WatchDog, or open one chapter for a focused example.

| Chapter | Outcome |
| --- | --- |
| [0. Install and build WatchDog](00-install-and-build.md) | Install prerequisites and build debug, release, or installer outputs. |
| [1. Create your first pipeline](01-first-pipeline.md) | Create the required files, load them, and run a minimal pipeline. |
| [2. Draw the pipeline graph](02-mermaid-graph.md) | Define Entry, normal, Endpoint, next, error, and fan-out routes in Mermaid. |
| [3. Configure pipeline metadata](03-metadata-basics.md) | Connect graph IDs to target, launch, node, and instance configuration. |
| [4. Recognize visual states](04-recognition.md) | Choose DirectHit, OCR, TemplateMatch, or ColorMatch and constrain the search area. |
| [5. Execute actions and control timing](05-actions-and-timing.md) | Add safe input actions, delays, repeats, and freeze waits. |
| [6. Route failures and complete runs](06-routing-and-lifecycle.md) | Understand candidate fan-out, error edges, timeouts, and Endpoint completion. |
| [7. Select and launch capture targets](07-targets-launch-capture.md) | Configure screen or window capture and optional target startup. |
| [8. Reuse nodes and parameters](08-reuse-and-parameters.md) | Build templates and instances and apply parameter precedence. |
| [9. Test and debug pipelines](09-testing-and-debugging.md) | Use Node Test, debug images, logs, and focused diagnosis. |
| [10. Add Patrol and recovery](10-patrol-and-recovery.md) | Configure the VLM, read the Chat Room, and test hints or guarded redirection. |
| [11. Record actions and receive notifications](11-recorder-and-notifications.md) | Capture an action draft and configure completion notifications. |

## Reference documents

Tutorials teach a workflow. These documents define the complete formats and
engineering contracts:

- [Pipeline metadata reference](../pipeline-meta.md)
- [Pipeline graph layout reference](../pipeline-layout.md)
- [Action Recorder reference](../action-recorder.md)
- [Development and build guide](../development.md)
- [Software Requirements Specification](../srs.md)
- [Software Design Description](../sdd.md)
