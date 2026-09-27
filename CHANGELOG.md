# Changelog

All notable changes to WatchDog will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and releases will use [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.1.0] - 2026-09-27

### Added

- Production and Tester splash startup with a determinate loading bar for
  maintenance, desktop services, schemas, pipeline resources, OCR, and main UI
  readiness.
- Tauri Store, FS, and Log plugin integration with scoped Recorder exports and
  application-directory storage.
- A signing-ready Windows workflow that verifies Authenticode signatures before
  packaging release artifacts.
- Production update checks, a localized update banner and Settings card, and
  guarded passive installation through the official Tauri Updater.

### Changed

- User resources, generated evidence, logs, settings, and exports now resolve
  through Tauri application directories. Startup safely migrates the legacy
  `%USERPROFILE%\.watch_dog` tree without overwriting conflicts.
- Frontend application logs now use the Tauri Log plugin while the Execution Log
  remains a separate product-facing timeline.
- Version tags now publish the signed NSIS updater, Tauri signature,
  `latest.json`, portable ZIP, and checksums to GitHub Releases.

## [1.0.0] - 2026-09-25

### Added

- Windows portable packaging with required-file checks, a content manifest,
  and SHA-256 checksums.
- Release configuration verification for versions, licensing, and bundled
  native resources.
- Mermaid pipeline authoring with source-preserving metadata and layout files.
- Screen and window capture with explicit stream ownership.
- OCR, template, color, and direct-hit recognition.
- Mouse, keyboard, text, wait, swipe, scroll, and custom actions.
- Entry and Endpoint node kinds, ordered fan-out, and explicit error routing.
- Parameter definitions, reusable node templates, and instance overrides.
- Stable-screen waits before and after actions.
- Node Test, annotated debug images, execution timing, and application logs.
- Patrol state matching, action-effect verification, recovery hints, and guarded
  redirection through an OpenAI-compatible VLM endpoint.
- Patrol Chat Room with structured prompts, responses, evidence, and recovery
  location handoffs.
- Windows Action Recorder with shortcuts, pause/resume, review, and JSON export.
- Native task notifications for pipeline completion and recorder state.
- English and Traditional Chinese interfaces.
- Startup cleanup for generated vision data and bounded dated logs.
- A numbered tutorial series covering installation, pipelines, metadata,
  testing, Patrol, recording, and notifications.

### Changed

- Pipeline actions now verify and restore the intended foreground window before
  dispatch.
- Patrol periodic work is observational; only failure-boundary recovery may
  propose a guarded redirect.
- Action evidence contains only successfully dispatched effect-eligible actions.
- Failure recovery now uses a validated Task 2 to Task 1 to Task 3 location
  handoff.
- Production code is organized into responsibility-based backend and frontend
  modules with documented dependency direction.

### Fixed

- Preserved source metadata across pipeline load/save cycles.
- Prevented pipeline path traversal and symlink escapes.
- Reported cleanup failures separately from execution results.
- Prevented Patrol reports from appearing after terminal pipeline status.
- Preserved active elapsed time between recorded actions.
- Improved multiline OCR candidate grouping and selection diagnostics.
- Included the Sample Target companion in the production installer so bundled
  example pipelines can launch their declared target.
- Made Patrol action-effect prompts distinguish exact node IDs from visible UI
  text, allowed the three-task failure handoff a 90-second active budget, and
  stopped labeling inconclusive recovery as a rejected redirect.

[Unreleased]: https://github.com/luberserker0v0/watchdog-automation/compare/v1.1.0...HEAD
[1.1.0]: https://github.com/luberserker0v0/watchdog-automation/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/luberserker0v0/watchdog-automation/releases/tag/v1.0.0
