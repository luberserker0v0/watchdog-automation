# Release readiness

This checklist prepares a WatchDog release candidate. It does not authorize or
perform a release. Every item must refer to one immutable commit.

## 1. Scope and documentation

- [ ] Release scope is frozen and documented in `CHANGELOG.md`.
- [ ] SRS audit basis names the candidate commit and no longer describes an
      uncommitted working tree.
- [ ] SDD matches the candidate implementation.
- [ ] Tutorials and examples match the bundled schemas and applications.
- [ ] All local Markdown links resolve.
- [ ] Public license is selected and a root `LICENSE` file exists.
- [ ] Package authors, description, repository, and version are accurate.

## 2. Automated verification

- [ ] `npm ci`
- [ ] `npm test`
- [ ] `npm run test:render`
- [ ] `npm run build`
- [ ] `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`
- [ ] `cargo check --manifest-path src-tauri/Cargo.toml`
- [ ] `cargo check --manifest-path src-tauri/Cargo.toml --features app-tester`
- [ ] `cargo check --manifest-path src-tauri/Cargo.toml --features app-target`
- [ ] `cargo test --no-run --manifest-path src-tauri/Cargo.toml`
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml`
- [ ] CI passes on the exact candidate commit.

Record test counts and the intentionally ignored OCR initialization test in
the candidate notes.

## 3. Native Windows acceptance

- [ ] Graphics, DXGI, and GDI capture smoke tests pass where supported.
- [ ] Screen and window pipeline targets pass.
- [ ] Foreground activation blocks input to the wrong window.
- [ ] Window placement restoration succeeds or reports a visible cleanup error.
- [ ] OCR CPU initialization passes from the packaged artifact.
- [ ] OCR DirectML initializes or visibly falls back to CPU.
- [ ] Pre and post freeze waits pass against a loading transition.
- [ ] Action Recorder native hooks, pause time, gesture conversion, and export
      timing pass.
- [ ] Pipeline and recorder notifications pass from an installed build.
- [ ] Patrol periodic State Matcher and Action Effect Verifier pass.
- [ ] Patrol transport retry, circuit opening, and terminal deadline pass.
- [ ] Patrol malformed JSON correction remains inconclusive after retry.
- [ ] Patrol failure handoff passes Task 2 location and Task 1 fallback cases.
- [ ] Hint-only uncertain location preserves the original failure.
- [ ] Accepted and rejected redirections enforce every engine guard.
- [ ] No Patrol report appears after terminal pipeline status.
- [ ] Startup cleanup stays inside the application data root.

## 4. Packaging

- [ ] `npm run verify:release-config` passes.
- [ ] `npm run build:all-targets` completes from a clean checkout.
- [ ] `npm run package:windows` creates a verified versioned directory and ZIP.
- [ ] Production, Tester, and Target executables have the intended identity.
- [ ] ONNX Runtime native DLLs and application resources are included in the
      portable package and installer.
- [ ] Installer contents include required OCR DLLs, resources, and the Sample
      Target companion required by bundled example pipelines.
- [ ] A fresh Windows machine or clean VM can install and start the application.
- [ ] OCR, capture, notifications, and bundled pipelines work after install.
- [ ] Uninstall leaves user-authored resources intact or clearly documents the
      selected data-removal policy.
- [ ] Artifact filenames contain version and architecture.
- [ ] SHA-256 checksums are generated for distributed artifacts.
- [ ] Windows code-signing policy is decided and documented.

## 5. Version and release candidate

- [ ] `package.json`, `src-tauri/Cargo.toml`, and
      `src-tauri/tauri.conf.json` use the same version.
- [ ] Lockfiles are updated by the normal package tools.
- [ ] Release notes identify Windows support and known limitations.
- [ ] The candidate is tagged only after all required checks pass.
- [ ] No credentials, generated captures, debug images, or local models are in
      the candidate commit.
- [ ] The working tree is clean.

## 6. Stable 1.0 approval

- [ ] A release candidate was exercised outside the development checkout.
- [ ] No unresolved severity-one or severity-two defects remain.
- [ ] File formats and IPC contracts intended for 1.x compatibility are listed.
- [ ] Product approval is recorded.
- [ ] Stable release authorization is explicit.
