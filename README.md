# WatchDog

WatchDog is a Windows desktop automation and visual pipeline application built with Tauri 2, React, TypeScript, and Rust. It captures screens or windows, recognizes content with OCR, template matching, and other nodes, then executes mouse or keyboard actions.

The repository produces three applications from one Rust binary target:

| Executable | Purpose | Cargo feature |
| --- | --- | --- |
| `watch-dog.exe` | Production pipeline and capture application | none |
| `watch-dog-tester.exe` | Developer UI for testing pipelines and individual nodes | `app-tester` |
| `watch-dog-target.exe` | Sample Target UI used by automated node tests | `app-target` |

## Requirements

WatchDog is currently developed and tested on Windows x64. Install:

- Node.js LTS and npm
- Rust stable with the MSVC toolchain
- Microsoft C++ Build Tools with **Desktop development with C++**
- Microsoft Edge WebView2 Runtime
- PowerShell

See the official [Tauri Windows prerequisites](https://v2.tauri.app/start/prerequisites/) for setup details.

## Setup

Install JavaScript dependencies from the repository root:

```powershell
npm install
```

The first Rust build downloads and compiles Cargo dependencies, so it takes substantially longer than later incremental builds.

## Development

Run one Tauri development command at a time because each command uses Vite port `12420`:

```powershell
# Production application
npm run tauri -- dev

# Pipeline and node tester
npm run dev:tester

# Sample Target UI
npm run dev:target
```

For frontend-only work, run `npm run dev`. Native Tauri commands such as capture and filesystem access are unavailable in a normal browser.

## Verify changes

```powershell
# Frontend tests
npm test

# Render-isolation tests
npm run test:render

# TypeScript and every Vite entrypoint
npm run build

# Rust tests and formatting
cargo test --manifest-path src-tauri\Cargo.toml
cargo fmt --manifest-path src-tauri\Cargo.toml -- --check
```

`npm run build` updates `dist\` only; it does not rebuild an executable.

## Build executables

Close every running WatchDog executable before building. Windows cannot replace a running `.exe`, and the build intentionally fails instead of retaining a stale artifact.

Fast development build:

```powershell
npm run build:all-targets:debug
```

Outputs:

```text
src-tauri\target\debug\watch-dog.exe
src-tauri\target\debug\watch-dog-tester.exe
src-tauri\target\debug\watch-dog-target.exe
```

Optimized standalone release build without installers:

```powershell
npm run build:all-targets
```

Outputs use the same names under `src-tauri\target\release\`. Both commands build tester, Target, and production in that order so the final `watch-dog.exe` is always the production application.

## Documentation

- [Development, builds, resources, and troubleshooting](docs/development.md)
- [Pipeline Mermaid and metadata format](docs/pipeline-meta.md)
