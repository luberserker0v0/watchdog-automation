# WatchDog

WatchDog is a Windows desktop automation and visual pipeline application built with Tauri 2, React, TypeScript, and Rust. It captures screens or windows, runs recognition nodes such as OCR and template matching, and executes mouse or keyboard actions.

The repository produces three applications from the same Rust binary target by enabling different Cargo features:

| Executable | Purpose | Cargo feature |
| --- | --- | --- |
| `watch-dog.exe` | Production pipeline and capture application | none |
| `watch-dog-tester.exe` | Developer UI for testing pipelines and individual nodes | `app-tester` |
| `watch-dog-target.exe` | Sample Target UI used by automated node tests | `app-target` |

## Requirements

This project is currently developed and tested on Windows x64. Install:

- Node.js LTS and npm
- Rust stable with the MSVC toolchain
- Microsoft C++ Build Tools with **Desktop development with C++**
- Microsoft Edge WebView2 Runtime
- PowerShell

See the official [Tauri Windows prerequisites](https://v2.tauri.app/start/prerequisites/) for setup details.

Verify the main tools:

```powershell
node --version
npm --version
rustc --version
cargo --version
```

## Install dependencies

From the repository root:

```powershell
npm install
```

The first Rust build downloads and compiles the Cargo dependencies, so it takes substantially longer than later incremental builds.

## Development

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

The target application opens the combined flow by default:

```text
Login -> Dashboard -> Settings
```

The node tester can launch an isolated Target UI page by passing an argument to the same executable:

```powershell
src-tauri\target\debug\watch-dog-target.exe login
src-tauri\target\debug\watch-dog-target.exe dashboard
src-tauri\target\debug\watch-dog-target.exe settings
```

These arguments create separate page windows named `Target UI - Login`, `Target UI - Dashboard`, and `Target UI - Settings`. They are not separate executables.

### Frontend only

```powershell
npm run dev
```

This starts Vite without the native Tauri backend. Screens, windows, streaming, filesystem access, and other Tauri commands will not work in a normal browser.

## Tests and frontend builds

Run all frontend tests:

```powershell
npm test
```

Run tests continuously:

```powershell
npm run test:watch
```

Run render-isolation tests:

```powershell
npm run test:render
```

Compile TypeScript and build every Vite HTML entrypoint:

```powershell
npm run build
```

`npm run build` creates files under `dist\`; it does **not** rebuild any `.exe` file.

Run Rust tests and formatting checks:

```powershell
cargo test --manifest-path src-tauri\Cargo.toml
cargo fmt --manifest-path src-tauri\Cargo.toml -- --check
```

## Fast debug executable builds

Debug builds compile faster and are recommended during development.

### Tester

```powershell
npm run build:tester:debug
```

Outputs:

```text
src-tauri\target\debug\watch-dog-tester.exe
src-tauri\target\debug\onnxruntime.dll
```

### Target UI

```powershell
npm run build:target:debug
```

Output:

```text
src-tauri\target\debug\watch-dog-target.exe
```

### Production application

```powershell
npm run tauri -- build --debug --no-bundle
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\install-onnxruntime.ps1 -Profile debug
```

Output:

```text
src-tauri\target\debug\watch-dog.exe
```

### Build all three debug executables

Build in this order. Target and tester builds temporarily overwrite Cargo's intermediate `watch-dog.exe`; building production last ensures that `watch-dog.exe` is the production application.

```powershell
npm run build:target:debug
npm run build:tester:debug
npm run tauri -- build --debug --no-bundle
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\install-onnxruntime.ps1 -Profile debug
```

Final files:

```text
src-tauri\target\debug\watch-dog.exe
src-tauri\target\debug\watch-dog-tester.exe
src-tauri\target\debug\watch-dog-target.exe
src-tauri\target\debug\onnxruntime.dll
```

> `npm run build:all-targets` currently aliases `npm run build:target:debug`. Despite its name, it only rebuilds `watch-dog-target.exe`.

## Release executable builds

Release builds enable optimization, LTO, stripping, and `panic = "abort"`. A clean release build can take several minutes.

### Tester

```powershell
npm run build:tester
```

Outputs:

```text
src-tauri\target\release\watch-dog-tester.exe
src-tauri\target\release\onnxruntime.dll
```

### Target UI

```powershell
npm run build:target
```

Output:

```text
src-tauri\target\release\watch-dog-target.exe
```

### Production application

```powershell
npm run tauri -- build --no-bundle
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\install-onnxruntime.ps1 -Profile release
```

Output:

```text
src-tauri\target\release\watch-dog.exe
```

### Build all three release executables

```powershell
npm run build:target
npm run build:tester
npm run tauri -- build --no-bundle
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\install-onnxruntime.ps1 -Profile release
```

Final files:

```text
src-tauri\target\release\watch-dog.exe
src-tauri\target\release\watch-dog-tester.exe
src-tauri\target\release\watch-dog-target.exe
src-tauri\target\release\onnxruntime.dll
src-tauri\target\release\DirectML.dll
```

Keep the DLLs installed by `install-onnxruntime.ps1` beside `watch-dog.exe` and `watch-dog-tester.exe`; OCR cannot initialize without them. The script installs the DirectML-enabled ONNX Runtime package, which supports both CPU inference and GPU inference on DirectX 12-capable NVIDIA, AMD, and Intel hardware. Select the OCR device under Settings → Debug Settings. GPU initialization automatically falls back to CPU when DirectML is unavailable.

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

The standalone executable workflow above is the supported development workflow. Before distributing an installer, verify that `onnxruntime.dll` and the required application resources are included in the installed application directory.

## Why the final Rust link is slow

Output similar to this is normal:

```text
Building [=======================> ] 658/661: watch-dog(bin)
```

The final executable links Tauri, WebView2 bindings, capture libraries, image processing, OCR/ONNX Runtime bindings, and the embedded frontend. Release LTO makes this final link especially slow. The tester and target variants use Cargo features, so changing the feature set requires a separate final binary link even when most dependencies are already cached.

For normal UI work, prefer `npm run dev:target`, `npm run dev:tester`, or a debug executable build. Use release builds only for final verification or distribution.

## Rebuilding after frontend changes

An existing `.exe` contains the frontend generated at its build time. Running only `npm run build` updates `dist\` but does not update an already-built executable.

For example, after changing the Target UI:

```powershell
npm run build:target:debug
src-tauri\target\debug\watch-dog-target.exe
```

If old behavior remains, confirm that the executable being launched is from the expected `debug` or `release` directory.

## Pipeline files

Each pipeline uses two files with separate responsibilities:

- `<name>.md` is a Mermaid flowchart that declares node IDs and connects them.
- `<name>.meta.json` configures those IDs through `nodes` and `edges`: recognition, actions, parameters, timing, target overrides, and normal/error edge behavior.

The node tester reads the same `.meta.json` structure. Metadata files under `pipeline-nodes` normally contain one node and omit `edges`, which defaults to `{}`. VS Code automatically applies `schemas/watchdog-metadata.schema.json` to every metadata file.

## Project layout

```text
src/                         React and TypeScript frontend
src/api/                     Tauri command wrappers and frontend API types
src/modules/capture/         Capture, streaming, preview, and debug UI
src/modules/pipeline/        Pipeline UI, tester, and sample Target UI
src-tauri/src/               Rust backend and Tauri commands
src-tauri/resources/         Pipelines, node metadata, OCR models, and images
scripts/                     Windows build-artifact and runtime helpers
dist/                        Generated Vite frontend
src-tauri/target/            Generated Rust executables and bundles
```

Generated directories such as `dist\`, `node_modules\`, and `src-tauri\target\` should not be edited manually.

## Common problems

### `EBUSY: resource busy or locked` while copying an executable

Windows does not allow a running `.exe` to be overwritten. Close the corresponding application window, or stop its process, and then run the build again. For example, close every running `watch-dog-target.exe` before running:

```powershell
npm run build:target:debug
```

The Rust compilation has already succeeded when this error appears; only the final copy from `watch-dog.exe` to the feature-specific executable failed. The copy helper retries short-lived antivirus/indexer locks before displaying this error.

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

Use `npm run dev:target` for live development, or rebuild `watch-dog-target.exe`. Double-clicking an older executable does not load the current source files.
