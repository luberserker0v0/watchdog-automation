# Chapter 0: Install and build WatchDog

This chapter prepares a Windows development environment and builds the three
WatchDog executables. Use debug builds while developing and release builds
when you need optimized standalone programs.

## 0.1 Install prerequisites

WatchDog is currently developed and tested on Windows x64. Install:

- Node.js LTS with npm;
- Rust stable with the MSVC toolchain;
- Microsoft C++ Build Tools with **Desktop development with C++**;
- Microsoft Edge WebView2 Runtime;
- PowerShell.

Follow the official
[Tauri Windows prerequisites](https://v2.tauri.app/start/prerequisites/)
when installing the Microsoft toolchain.

Verify the command-line tools:

```powershell
node --version
npm --version
rustc --version
cargo --version
```

## 0.2 Install project dependencies

Open PowerShell in the repository root:

```powershell
cd D:\program\Projects\WatchDog\watch-dog
npm install
```

The first Rust build downloads and compiles dependencies. Later incremental
builds are substantially faster.

## 0.3 Run in development mode

Run one Tauri development application at a time because they share Vite port
`12420`:

```powershell
# Production application
npm run tauri -- dev

# Pipeline and node tester
npm run dev:tester

# Sample Target UI
npm run dev:target
```

`npm run dev` starts only the frontend. Native capture, filesystem, window,
notification, and input commands are unavailable in a normal browser.

## 0.4 Build debug executables

Close all running WatchDog executables before building. Windows cannot replace
an executable while its process is running.

Build all applications:

```powershell
npm run build:all-targets:debug
```

Outputs:

```text
src-tauri\target\debug\watch-dog.exe
src-tauri\target\debug\watch-dog-tester.exe
src-tauri\target\debug\watch-dog-target.exe
```

You can also build one application:

| Application | Command |
| --- | --- |
| Production | `npm run build:app:debug` |
| Tester | `npm run build:tester:debug` |
| Sample Target | `npm run build:target:debug` |

The all-targets command builds Tester, Target, and production in that order.
Production runs last so Cargo's final `watch-dog.exe` is the production app.

## 0.5 Build release executables

Build all optimized standalone executables without installers:

```powershell
npm run build:all-targets
```

Outputs:

```text
src-tauri\target\release\watch-dog.exe
src-tauri\target\release\watch-dog-tester.exe
src-tauri\target\release\watch-dog-target.exe
```

Individual release commands are:

| Application | Command |
| --- | --- |
| Production | `npm run build:app` |
| Tester | `npm run build:tester` |
| Sample Target | `npm run build:target` |

Release builds enable optimization and can take several minutes when the
release cache is empty.

## 0.6 Keep runtime DLLs with the executable

The production and Tester build scripts install ONNX Runtime libraries into
the matching output directory. Keep `onnxruntime.dll` and the installed
DirectML libraries beside `watch-dog.exe` and `watch-dog-tester.exe`. OCR
cannot initialize without ONNX Runtime.

If the DLL is missing from a debug build, reinstall it with:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\install-onnxruntime.ps1 -Profile debug
```

Use `-Profile release` for the release output directory.

## 0.7 Create a portable Windows package

Build all three release applications, collect their runtime files, and verify
the package:

```powershell
npm run package:windows
```

The command creates a directory, ZIP archive, and archive checksum under
`artifacts\`. The name includes the application version and `windows-x64`. The package contains the
three executables, ONNX Runtime DLLs, built-in resources, JSON schemas, legal
files, a manifest, and SHA-256 checksums.

To verify an existing release configuration without compiling applications:

```powershell
npm run verify:release-config
```

For a local package assembled from existing debug binaries:

```powershell
npm run package:windows:debug -- -SkipBuild
```

Portable packages are release candidates until their Windows acceptance tests
in `docs\release-readiness.md` pass.

Ordinary local and CI packages are intentionally unsigned. A trusted release
operator can create signed artifacts with:

```powershell
$env:WINDOWS_CERTIFICATE_BASE64 = '<base64 PFX from a protected secret>'
$env:WINDOWS_CERTIFICATE_PASSWORD = '<PFX password>'
$env:WINDOWS_TIMESTAMP_URL = 'https://<trusted RFC 3161 timestamp service>'
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content "$env:USERPROFILE\.tauri\watchdog-updater.key" -Raw
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = '<updater key password>'
npm run package:windows:signed
```

The script imports the certificate temporarily into `Cert:\CurrentUser\My`,
signs and verifies all distributed executables and the NSIS installer, verifies
the Tauri update signature with Minisign, packages only verified artifacts,
then removes certificate and updater-key temporary material in a `finally`
block. A matching `v*` tag publishes the feed; manual workflow dispatch creates
a non-publishing signed candidate. Generate and recover the updater key using
[`docs/updater-signing.md`](../updater-signing.md).

## 0.8 Build Windows installers

Build the production NSIS installer and its Sample Target companion:

```powershell
npm run build:installer
```

Tauri writes enabled bundles below:

```text
src-tauri\target\release\bundle\msi\
src-tauri\target\release\bundle\nsis\
```

The production bundle stages application resources, native OCR DLLs, and
`watch-dog-target.exe` before Tauri builds the installer. The companion makes
the bundled `sample` and `patrol-redirection` pipelines runnable. Node Tester
remains a separate developer executable and is not installed. Test the
installed application on a clean Windows machine before distribution.

## 0.9 Verify the build

Run the automated checks:

```powershell
npm test
npm run test:render
npm run build
cargo test --manifest-path src-tauri\Cargo.toml
cargo fmt --manifest-path src-tauri\Cargo.toml -- --check
```

`npm run build` validates TypeScript and builds browser assets under `dist`; it
does not rebuild an executable.

Start each generated executable and confirm its identity:

- `watch-dog.exe` opens the production application;
- `watch-dog-tester.exe` opens Pipeline and Node Test tools;
- `watch-dog-target.exe` opens the sample Target UI.

## 0.10 Fix common build failures

### The executable is locked

Close the corresponding WatchDog process and rerun the command. A failure while
copying a feature-specific executable often means compilation succeeded but
Windows prevented replacement of the old file.

### Port 12420 is already in use

Stop the other Tauri or Vite development process. Only one development target
can use the configured port at a time.

### OCR cannot load ONNX Runtime

Run the install script from section 0.6 and keep the installed DLLs beside the
executable.

For detailed resource paths, build internals, and additional diagnostics, see
the [development guide](../development.md).

Next: [Create your first pipeline](01-first-pipeline.md).
