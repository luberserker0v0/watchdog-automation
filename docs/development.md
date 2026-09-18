# WatchDog development guide

This guide covers the detailed Windows development, build, runtime-resource, and troubleshooting workflows. For initial setup and the shortest common commands, see the [project README](../README.md).

Verify the required toolchain before the first build:

```powershell
node --version
npm --version
rustc --version
cargo --version
```

## Development applications

Run one Tauri development command at a time because each command starts Vite on port `12420`.

### Production application

```powershell
npm run tauri -- dev
```

### Pipeline and node tester

```powershell
npm run dev:tester
```

### Target UI

```powershell
npm run dev:target
```

The Target application opens the combined flow by default:

```text
Login -> Dashboard -> Settings
```

The node tester can launch an isolated Target page by passing an argument to the same executable:

```powershell
src-tauri\target\debug\watch-dog-target.exe login
src-tauri\target\debug\watch-dog-target.exe dashboard
src-tauri\target\debug\watch-dog-target.exe settings
```

These arguments create windows named `Target UI - Login`, `Target UI - Dashboard`, and `Target UI - Settings`. They are not separate executables.

### Frontend only

```powershell
npm run dev
```

This starts Vite without the native Tauri backend. Screens, windows, streaming, filesystem access, and other Tauri commands will not work in a normal browser.

## Tests and frontend builds

```powershell
# Run all frontend tests once
npm test

# Run frontend tests continuously
npm run test:watch

# Run render-isolation tests
npm run test:render

# Compile TypeScript and build every Vite HTML entrypoint
npm run build

# Run Rust tests and check formatting
cargo test --manifest-path src-tauri\Cargo.toml
cargo fmt --manifest-path src-tauri\Cargo.toml -- --check
```

`npm run build` creates files under `dist\`; it does not rebuild any `.exe` file.

## Debug executable builds

Debug builds compile faster and are recommended during development.

| Application | Command | Output |
| --- | --- | --- |
| Tester | `npm run build:tester:debug` | `src-tauri\target\debug\watch-dog-tester.exe` |
| Target UI | `npm run build:target:debug` | `src-tauri\target\debug\watch-dog-target.exe` |
| Production | `npm run build:app:debug` | `src-tauri\target\debug\watch-dog.exe` |

The tester and production builds also install `onnxruntime.dll` beside their executable.

To build all three, first close every running WatchDog executable, then run:

```powershell
npm run build:all-targets:debug
```

The command builds tester first, Target second, and production last. Feature builds temporarily overwrite Cargo's intermediate `watch-dog.exe`; production-last ordering guarantees that the final file contains the production application. A locked destination fails the build instead of silently retaining an older executable.

## Release executable builds

Release builds enable optimization, LTO, stripping, and `panic = "abort"`. A clean release build can take several minutes.

| Application | Command | Output |
| --- | --- | --- |
| Tester | `npm run build:tester` | `src-tauri\target\release\watch-dog-tester.exe` |
| Target UI | `npm run build:target` | `src-tauri\target\release\watch-dog-target.exe` |
| Production | `npm run build:app` | `src-tauri\target\release\watch-dog.exe` |

Build all three standalone executables without installers:

```powershell
npm run build:all-targets
```

This uses the same tester → Target → production ordering as the debug command. The resulting directory also contains `onnxruntime.dll` and `DirectML.dll` where required.

Keep the DLLs installed by `scripts\install-onnxruntime.ps1` beside `watch-dog.exe` and `watch-dog-tester.exe`; OCR cannot initialize without them. The script installs the DirectML-enabled ONNX Runtime package, supporting CPU inference and GPU inference on DirectX 12-capable NVIDIA, AMD, and Intel hardware. Select the OCR device under Settings → Debug Settings. GPU initialization falls back to CPU when DirectML is unavailable.

## Windows installers

Build production installers with:

```powershell
npm run tauri -- build
```

Tauri writes installer bundles under:

```text
src-tauri\target\release\bundle\msi\
src-tauri\target\release\bundle\nsis\
```

Standalone executables are the supported development workflow. Before distributing an installer, verify that `onnxruntime.dll` and the required application resources are included in the installed application directory.

## Why the final Rust link is slow

Output similar to this is normal:

```text
Building [=======================> ] 658/661: watch-dog(bin)
```

The final executable links Tauri, WebView2 bindings, capture libraries, image processing, OCR/ONNX Runtime bindings, and the embedded frontend. Release LTO makes this link especially slow. Tester and Target use Cargo features, so each feature set requires a separate final link even when dependencies are cached.

For normal UI work, prefer `npm run dev:target`, `npm run dev:tester`, or a debug executable build. Reserve release builds for final verification or distribution.

## Rebuilding after frontend changes

An existing `.exe` contains the frontend generated at build time. Running only `npm run build` updates `dist\` but does not update an already-built executable.

For example, after changing the Target UI:

```powershell
npm run build:target:debug
src-tauri\target\debug\watch-dog-target.exe
```

If old behavior remains, confirm that the launched executable comes from the expected `debug` or `release` directory.

## Pipeline files

Each pipeline has a Mermaid `<name>.md` flow and matching `<name>.meta.json` execution configuration. It may also have a shareable `<name>.layout.json` containing graph node positions. The node tester uses the same metadata shape.

- See the [pipeline metadata guide](pipeline-meta.md) for targets, nodes, recognition, actions, parameters, and runtime defaults.
- See the [pipeline layout guide](pipeline-layout.md) for graph editing, the layout file format, and sharing behavior.

## Project and runtime resources

```text
src/                         React and TypeScript frontend
src/api/                     Tauri command wrappers and frontend API types
src/modules/capture/         Capture, streaming, preview, and debug UI
src/modules/pipeline/        Pipeline UI, tester, and sample Target UI
src-tauri/src/               Rust backend and Tauri commands
src-tauri/resources/         Bundled pipelines, node metadata, OCR models, and images
scripts/                     Windows build-artifact and runtime helpers
dist/                        Generated Vite frontend
src-tauri/target/            Generated Rust executables and bundles
```

Generated directories such as `dist\`, `node_modules\`, and `src-tauri\target\` should not be edited manually.

User-provided resources and runtime output are stored separately:

```text
%USERPROFILE%\.watch_dog\
├── resources\
│   ├── pipelines\       User Mermaid, metadata, and optional graph-layout files
│   ├── pipeline-nodes\  User node-test metadata
│   ├── images\          User template-matching images
│   └── models\          User/custom model files
├── logs\                Runtime logs
└── visions\             Captures and debug images
```

Template image resolution checks user `resources\images` first, then falls back to bundled `src-tauri\resources\images`. On startup, pipelines from the legacy `%USERPROFILE%\.watch_dog\pipelines` location move to `%USERPROFILE%\.watch_dog\resources\pipelines` without overwriting an existing pipeline.

## Troubleshooting

### `EBUSY: resource busy or locked` while copying an executable

Windows does not allow a running `.exe` to be overwritten. Close the corresponding application window or stop its process, then rerun the build. The Rust compilation has already succeeded when this error appears; only the final feature-specific artifact copy failed. The copy helper retries short-lived antivirus or indexer locks before reporting the error.

### `'cp' is not recognized`

The project uses `scripts\copy-build-artifact.mjs` instead of the Unix-only `cp` command. Use the npm build scripts rather than copying artifacts with `cp` on Windows.

### OCR reports that ONNX Runtime is missing

Install the DLL beside the executable:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\install-onnxruntime.ps1 -Profile debug
```

Replace `debug` with `release` when running a release executable.

### Port `12420` is already in use

Only one Tauri development command can own the configured Vite port. Stop the existing `npm run dev`, `dev:tester`, or `dev:target` process before starting another.

### Target UI changes do not appear

Use `npm run dev:target` for live development or rebuild `watch-dog-target.exe`. Double-clicking an older executable does not load current source files.
