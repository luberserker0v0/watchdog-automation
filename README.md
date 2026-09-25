# WatchDog

WatchDog is a Windows desktop automation and visual pipeline application built with Tauri 2, React, TypeScript, and Rust. It captures screens or windows, recognizes content with OCR, template matching, and other nodes, then executes mouse or keyboard actions.

The repository produces three applications from one Rust binary target:

| Executable | Purpose | Cargo feature |
| --- | --- | --- |
| `watch-dog.exe` | Production pipeline and capture application | none |
| `watch-dog-tester.exe` | Developer UI for testing pipelines and individual nodes | `app-tester` |
| `watch-dog-target.exe` | Sample Target UI used by automated node tests | `app-target` |

## Quick start

After installing the
[Windows prerequisites](docs/tutorials/00-install-and-build.md#01-install-prerequisites),
install dependencies and start the production application:

```powershell
npm install
npm run tauri -- dev
```

Build all debug or optimized standalone executables:

```powershell
# Fast development executables
npm run build:all-targets:debug

# Optimized release executables
npm run build:all-targets
```

See [Chapter 0: Install and build WatchDog](docs/tutorials/00-install-and-build.md)
for individual targets, output paths, runtime DLLs, installers, verification,
and troubleshooting.

## Documentation

### Tutorial outline

| Chapter | Tutorial |
| --- | --- |
| 0 | [Install and build WatchDog](docs/tutorials/00-install-and-build.md) |
| 1 | [Create your first pipeline](docs/tutorials/01-first-pipeline.md) |
| 2 | [Draw the pipeline graph in Mermaid](docs/tutorials/02-mermaid-graph.md) |
| 3 | [Configure pipeline metadata JSON](docs/tutorials/03-metadata-basics.md) |
| 4 | [Recognize visual states](docs/tutorials/04-recognition.md) |
| 5 | [Execute actions and control timing](docs/tutorials/05-actions-and-timing.md) |
| 6 | [Route failures and complete runs](docs/tutorials/06-routing-and-lifecycle.md) |
| 7 | [Select and launch capture targets](docs/tutorials/07-targets-launch-capture.md) |
| 8 | [Reuse nodes and parameters](docs/tutorials/08-reuse-and-parameters.md) |
| 9 | [Test and debug pipelines](docs/tutorials/09-testing-and-debugging.md) |
| 10 | [Add Patrol and recovery](docs/tutorials/10-patrol-and-recovery.md) |
| 11 | [Record actions and receive notifications](docs/tutorials/11-recorder-and-notifications.md) |

The [tutorial index](docs/tutorials/README.md) describes the learning path and
links the related reference documents.

### Reference and engineering documents

- [Apache License 2.0](LICENSE)
- [Changelog](CHANGELOG.md)
- [Contributing guide](CONTRIBUTING.md)
- [Security policy](SECURITY.md)
- [Release-readiness checklist](docs/release-readiness.md)
- [Version 1.0.0 release notes](docs/releases/1.0.0.md)
- [Development, builds, resources, and troubleshooting](docs/development.md)
- [Pipeline Mermaid and execution metadata reference](docs/pipeline-meta.md)
- [Shareable pipeline graph layouts](docs/pipeline-layout.md)
- [Action Recorder reference](docs/action-recorder.md)
- [Software Requirements Specification](docs/srs.md)
- [Software Design Description](docs/sdd.md)
