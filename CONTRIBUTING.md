# Contributing to WatchDog

Thank you for improving WatchDog. This project combines a Tauri backend, a
React frontend, native Windows capture and input, and pipeline documentation.

## Development setup

Follow [Chapter 0: Install and build WatchDog](docs/tutorials/00-install-and-build.md).
Use [docs/development.md](docs/development.md) for detailed commands and
troubleshooting.

## Before changing code

1. Create a focused branch.
2. Read the SRS and SDD sections related to the feature.
3. Preserve Tauri command names, event payloads, saved file formats, and routes
   unless the change intentionally introduces a documented migration.
4. Keep credentials, captured personal data, generated visions, debug images,
   and local model files out of commits.

## Architecture rules

Backend feature dependencies follow:

```text
core → capture → authoring → plugins → runtime → testing
```

Use capability facades for cross-module work. Keep implementation helpers
private or `pub(crate)` unless they are part of an intentional contract.

Frontend state belongs in stores. Put orchestration and side effects in typed
services. Cross-feature execution logs belong to observability rather than a
capture or pipeline component.

Production files should remain near 500 lines when practical. Split by domain
or responsibility when a file contains unrelated state, I/O, protocol, and UI
logic.

## Tests

Run the complete automated checks before requesting review:

```powershell
npm test
npm run test:render
npm run build
cargo fmt --manifest-path src-tauri\Cargo.toml -- --check
cargo check --manifest-path src-tauri\Cargo.toml
cargo check --manifest-path src-tauri\Cargo.toml --features app-tester
cargo check --manifest-path src-tauri\Cargo.toml --features app-target
cargo test --no-run --manifest-path src-tauri\Cargo.toml
cargo test --manifest-path src-tauri\Cargo.toml
```

Add tests for parsing, routing, safety checks, persistence, and state
transitions. Avoid tests that only repeat implementation details.

Native features also require focused Windows walkthroughs. Record the target,
scenario, observed result, and relevant log evidence in the pull request.

## Documentation

- Update the tutorial that teaches the affected workflow.
- Update `docs/pipeline-meta.md` when the pipeline format changes.
- Update SRS requirements and SDD interfaces when behavior or architecture
  changes.
- Keep examples free of real credentials and personal data.
- Verify local Markdown links.

## Pull requests

Keep each pull request focused and describe:

- what changed;
- why it changed;
- how it was tested;
- native verification performed;
- compatibility or migration effects;
- remaining limitations.

Do not commit build output from `dist`, `node_modules`, or `src-tauri/target`.
