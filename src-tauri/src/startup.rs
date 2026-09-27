use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

const STARTUP_EVENT: &str = "startup-progress";
const MIN_SPLASH_DISPLAY: std::time::Duration = std::time::Duration::from_millis(1500);

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StartupProgress {
    pub percent: u8,
    pub stage: String,
    pub detail: Option<String>,
    pub complete: bool,
}

struct StartupInner {
    progress: StartupProgress,
    main_ready: bool,
    initialization_complete: bool,
    revealed: bool,
}

pub struct StartupState {
    inner: Mutex<StartupInner>,
    gate: tokio::sync::Mutex<()>,
    clean_generated_data: bool,
    started_at: std::time::Instant,
    minimum_display: std::time::Duration,
}

impl StartupState {
    pub fn new(clean_generated_data: bool) -> Self {
        Self::with_minimum_display(clean_generated_data, MIN_SPLASH_DISPLAY)
    }

    fn with_minimum_display(
        clean_generated_data: bool,
        minimum_display: std::time::Duration,
    ) -> Self {
        Self {
            inner: Mutex::new(StartupInner {
                progress: StartupProgress {
                    percent: 4,
                    stage: "preparing".into(),
                    detail: None,
                    complete: false,
                },
                main_ready: false,
                initialization_complete: false,
                revealed: false,
            }),
            gate: tokio::sync::Mutex::new(()),
            clean_generated_data,
            started_at: std::time::Instant::now(),
            minimum_display,
        }
    }

    fn snapshot(&self) -> StartupProgress {
        self.inner
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .progress
            .clone()
    }

    fn update(&self, app: &AppHandle, percent: u8, stage: &str, detail: Option<String>) {
        let progress = {
            let mut inner = self.inner.lock().unwrap_or_else(|error| error.into_inner());
            if !inner.revealed {
                inner.progress = StartupProgress {
                    percent: percent.max(inner.progress.percent).min(100),
                    stage: stage.into(),
                    detail,
                    complete: false,
                };
            }
            inner.progress.clone()
        };
        let _ = app.emit_to("splash", STARTUP_EVENT, progress);
    }

    fn mark_initialization_complete(&self) {
        self.inner
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .initialization_complete = true;
    }

    fn mark_main_ready(&self) {
        self.inner
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .main_ready = true;
    }

    fn claim_reveal(&self) -> Option<std::time::Duration> {
        let mut inner = self.inner.lock().unwrap_or_else(|error| error.into_inner());
        if inner.revealed || !inner.main_ready || !inner.initialization_complete {
            return None;
        }
        inner.revealed = true;
        inner.progress = StartupProgress {
            percent: 100,
            stage: "ready".into(),
            detail: None,
            complete: true,
        };
        Some(
            self.minimum_display
                .saturating_sub(self.started_at.elapsed()),
        )
    }
}

fn reveal_if_ready(app: &AppHandle, state: &StartupState) {
    let Some(remaining_display) = state.claim_reveal() else {
        return;
    };
    let progress = state.snapshot();
    let _ = app.emit_to("splash", STARTUP_EVENT, progress);
    let main = app.get_webview_window("main");
    let splash = app.get_webview_window("splash");
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(remaining_display).await;
        if let Some(main) = main {
            let _ = main.show();
            let _ = main.set_focus();
        }
        if let Some(splash) = splash {
            tokio::time::sleep(std::time::Duration::from_millis(220)).await;
            let _ = splash.close();
        }
    });
}

#[tauri::command]
pub fn get_startup_progress(state: State<'_, StartupState>) -> StartupProgress {
    state.snapshot()
}

#[tauri::command]
pub fn mark_main_window_ready(app: AppHandle, state: State<'_, StartupState>) {
    state.mark_main_ready();
    state.update(&app, 94, "interface", None);
    reveal_if_ready(&app, &state);
}

#[tauri::command]
pub async fn initialize_app(
    recognition_device: String,
    app: AppHandle,
    state: State<'_, StartupState>,
) -> Result<StartupProgress, String> {
    let _gate = state.gate.lock().await;
    if state
        .inner
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .initialization_complete
    {
        reveal_if_ready(&app, &state);
        return Ok(state.snapshot());
    }

    state.update(&app, 10, "storage", None);
    let migration_warnings = tokio::task::spawn_blocking(crate::core::paths::migrate_legacy_data)
        .await
        .map_err(|error| format!("Legacy data migration worker failed: {error}"))?;
    for warning in &migration_warnings {
        log::warn!("[migration] {warning}");
    }
    let migration_detail = (!migration_warnings.is_empty()).then(|| {
        format!(
            "{} legacy item(s) require attention; original files were retained",
            migration_warnings.len()
        )
    });
    state.update(&app, 18, "migration", migration_detail);
    if state.clean_generated_data {
        let maintenance = tokio::task::spawn_blocking(crate::core::maintenance::cleanup_on_startup)
            .await
            .map_err(|error| format!("Startup maintenance worker failed: {error}"));
        if let Err(error) = maintenance {
            log::warn!("[startup] {error}");
            state.update(&app, 28, "maintenance", Some(error));
        } else {
            state.update(&app, 28, "maintenance", None);
        }
    } else {
        state.update(&app, 28, "maintenance", None);
    }

    let desktop = tokio::task::spawn_blocking(|| {
        crate::modules::capture::repositories::virtual_desktop::get_desktop_manager();
    })
    .await;
    let desktop_detail = desktop
        .err()
        .map(|error| format!("Desktop service preload failed: {error}"));
    if let Some(error) = &desktop_detail {
        log::warn!("[startup] {error}");
    }
    state.update(&app, 42, "desktop", desktop_detail);

    let schema_result = tokio::task::spawn_blocking(|| {
        crate::modules::pipeline::authoring::parser::meta::parse(r#"{"nodes":{}}"#)?;
        crate::modules::pipeline::authoring::parser::layout::parse(r#"{"nodes":{}}"#)?;
        Ok::<(), String>(())
    })
    .await
    .map_err(|error| format!("Schema preload worker failed: {error}"))
    .and_then(|result| result);
    let schema_detail = schema_result.err();
    if let Some(error) = &schema_detail {
        log::warn!("[startup] {error}");
    }
    state.update(&app, 56, "schemas", schema_detail);

    let catalog_result =
        tokio::task::spawn_blocking(crate::modules::pipeline::authoring::editor::list_pipelines)
            .await
            .map_err(|error| format!("Pipeline catalog worker failed: {error}"))
            .and_then(|result| result.map(|pipelines| pipelines.len()));
    let catalog_detail = match catalog_result {
        Ok(count) => Some(format!("{count}")),
        Err(error) => {
            log::warn!("[startup] {error}");
            Some(error)
        }
    };
    state.update(&app, 68, "pipelines", catalog_detail);

    state.update(&app, 74, "ocr", None);
    let requested_device = recognition_device.clone();
    let ocr_result = tokio::task::spawn_blocking(move || {
        crate::modules::pipeline::plugins::recognizers::ocr::set_device(&requested_device)
    })
    .await
    .map_err(|error| format!("OCR preload worker failed: {error}"))
    .and_then(|result| result);
    let ocr_detail = match ocr_result {
        Ok(active) => Some(active),
        Err(error) => {
            log::warn!("[startup] OCR preload unavailable: {error}");
            Some(error)
        }
    };
    state.update(&app, 90, "ocr_ready", ocr_detail);

    state.mark_initialization_complete();
    state.update(&app, 94, "interface", None);
    reveal_if_ready(&app, &state);
    Ok(state.snapshot())
}

pub fn arm_fallback(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        let Some(state) = app.try_state::<StartupState>() else {
            return;
        };
        reveal_if_ready(&app, &state);
        let already_revealed = state
            .inner
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .revealed;
        if already_revealed {
            return;
        }
        let progress = {
            let mut inner = state
                .inner
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            inner.revealed = true;
            inner.progress = StartupProgress {
                percent: 100,
                stage: "ready".into(),
                detail: Some("Startup coordination timed out".into()),
                complete: true,
            };
            inner.progress.clone()
        };
        log::warn!("[startup] splash timeout; revealing the main window");
        let _ = app.emit_to("splash", STARTUP_EVENT, progress);
        if let Some(main) = app.get_webview_window("main") {
            let _ = main.show();
            let _ = main.set_focus();
        }
        if let Some(splash) = app.get_webview_window("splash") {
            let _ = splash.close();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reveal_requires_both_backend_and_interface_readiness() {
        let minimum = std::time::Duration::from_millis(1500);
        let state = StartupState::with_minimum_display(true, minimum);
        assert!(state.claim_reveal().is_none());
        state.mark_main_ready();
        assert!(state.claim_reveal().is_none());
        state.mark_initialization_complete();
        let remaining = state.claim_reveal().expect("both readiness gates reveal");
        assert!(remaining > std::time::Duration::from_millis(1400));
        assert!(remaining <= minimum);
        assert!(state.claim_reveal().is_none());
        assert_eq!(state.snapshot().percent, 100);
        assert!(state.snapshot().complete);
    }
}
