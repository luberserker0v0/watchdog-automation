use std::collections::HashSet;
use std::sync::{mpsc, Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter};

use crate::modules::capture::repositories::window::{
    ensure_window_foreground, require_window_foreground,
};
use crate::modules::capture::services::bounds::{
    screen_capture_bounds, window_capture_bounds, CaptureBounds,
};
use crate::modules::pipeline::authoring::models::action::{Action, Target};

mod hooks;

use super::models::{
    RecordedActionDraft, RecorderState, RecorderStatus, RecordingConfig, RecordingSession,
    RecordingTarget,
};
#[cfg(target_os = "windows")]
use hooks::windows_hooks;

#[derive(Debug, Clone)]
pub(super) enum RawInput {
    MouseDown {
        x: i32,
        y: i32,
        at: Instant,
    },
    MouseUp {
        x: i32,
        y: i32,
        at: Instant,
    },
    Wheel {
        x: i32,
        y: i32,
        delta: i32,
        at: Instant,
    },
    KeyDown {
        code: u32,
        at: Instant,
    },
    KeyUp {
        code: u32,
    },
}

struct Inner {
    state: RecorderState,
    config: Option<RecordingConfig>,
    session: Option<RecordingSession>,
    started: Option<Instant>,
    reason: Option<String>,
    next_action_id: u64,
}

impl Default for Inner {
    fn default() -> Self {
        Self {
            state: RecorderState::Idle,
            config: None,
            session: None,
            started: None,
            reason: None,
            next_action_id: 1,
        }
    }
}

struct Controller {
    #[cfg(target_os = "windows")]
    thread_id: u32,
    hook_thread: Option<JoinHandle<()>>,
    processor_thread: Option<JoinHandle<()>>,
}

pub struct RecorderService {
    inner: Arc<Mutex<Inner>>,
    controller: Mutex<Option<Controller>>,
}

impl Default for RecorderService {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner::default())),
            controller: Mutex::new(None),
        }
    }
}

impl RecorderService {
    pub fn status(&self) -> RecorderStatus {
        status_from(&self.inner)
    }

    pub fn start(&self, config: RecordingConfig, app: AppHandle) -> Result<RecorderStatus, String> {
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (config, app);
            return Err("Action Recorder is not implemented on this platform".to_string());
        }

        #[cfg(target_os = "windows")]
        {
            let mut inner = self.inner.lock().unwrap_or_else(|error| error.into_inner());
            if inner.state != RecorderState::Idle {
                return Ok(status_from_locked(&inner));
            }
            validate_target(&config.target)?;
            let started_unix_ms = unix_ms();
            let session_id = format!("recording-{started_unix_ms}");
            inner.state = RecorderState::Recording;
            inner.started = Some(Instant::now());
            inner.reason = None;
            inner.next_action_id = 1;
            inner.config = Some(config.clone());
            inner.session = Some(RecordingSession {
                id: session_id,
                started_unix_ms,
                duration_ms: 0,
                target: config.target.clone(),
                actions: Vec::new(),
            });
            drop(inner);

            let (sender, receiver) = mpsc::channel::<RawInput>();
            let (ready_sender, ready_receiver) = mpsc::channel::<Result<u32, String>>();
            let hook_thread = std::thread::spawn(move || windows_hooks::run(sender, ready_sender));
            let thread_id = match ready_receiver.recv_timeout(Duration::from_secs(3)) {
                Ok(Ok(thread_id)) => thread_id,
                Ok(Err(error)) => {
                    let _ = hook_thread.join();
                    self.reset_after_start_error();
                    return Err(error);
                }
                Err(_) => {
                    self.reset_after_start_error();
                    return Err("Timed out while starting Windows input hooks".to_string());
                }
            };

            let processor_inner = self.inner.clone();
            let processor_app = app.clone();
            let processor_thread = std::thread::spawn(move || {
                process_events(receiver, processor_inner, processor_app)
            });
            *self
                .controller
                .lock()
                .unwrap_or_else(|error| error.into_inner()) = Some(Controller {
                thread_id,
                hook_thread: Some(hook_thread),
                processor_thread: Some(processor_thread),
            });
            let status = self.status();
            let _ = app.emit("recorder-status", &status);
            Ok(status)
        }
    }

    pub fn pause(&self, app: AppHandle) -> Result<RecorderStatus, String> {
        let mut inner = self.inner.lock().unwrap_or_else(|error| error.into_inner());
        match inner.state {
            RecorderState::Recording => {
                inner.state = RecorderState::Paused;
                inner.reason = Some("Paused by user".to_string());
            }
            RecorderState::Paused => {}
            _ => return Err("No active recording can be paused".to_string()),
        }
        let status = status_from_locked(&inner);
        drop(inner);
        let _ = app.emit("recorder-status", &status);
        Ok(status)
    }

    pub fn resume(&self, app: AppHandle) -> Result<RecorderStatus, String> {
        let mut inner = self.inner.lock().unwrap_or_else(|error| error.into_inner());
        match inner.state {
            RecorderState::Paused => {
                inner.state = RecorderState::Recording;
                inner.reason = None;
            }
            RecorderState::Recording => {}
            _ => return Err("No paused recording can be resumed".to_string()),
        }
        let status = status_from_locked(&inner);
        drop(inner);
        let _ = app.emit("recorder-status", &status);
        Ok(status)
    }

    pub fn stop(&self, app: AppHandle) -> Result<RecordingSession, String> {
        {
            let mut inner = self.inner.lock().unwrap_or_else(|error| error.into_inner());
            if inner.state == RecorderState::Idle {
                return inner
                    .session
                    .clone()
                    .ok_or_else(|| "No recording session is available".to_string());
            }
            inner.state = RecorderState::Stopping;
            if let (Some(started), Some(session)) = (inner.started, inner.session.as_mut()) {
                session.duration_ms = started.elapsed().as_millis().min(u64::MAX as u128) as u64;
            }
        }
        let stopping = self.status();
        let _ = app.emit("recorder-status", &stopping);

        #[cfg(target_os = "windows")]
        if let Some(mut controller) = self
            .controller
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take()
        {
            windows_hooks::stop(controller.thread_id);
            if let Some(thread) = controller.hook_thread.take() {
                let _ = thread.join();
            }
            if let Some(thread) = controller.processor_thread.take() {
                let _ = thread.join();
            }
        }

        let mut inner = self.inner.lock().unwrap_or_else(|error| error.into_inner());
        inner.state = RecorderState::Idle;
        inner.config = None;
        inner.started = None;
        inner.reason = None;
        let session = inner
            .session
            .clone()
            .ok_or_else(|| "Recording stopped without a session".to_string())?;
        let status = status_from_locked(&inner);
        drop(inner);
        let _ = app.emit("recorder-status", &status);
        Ok(session)
    }

    pub fn clear(&self) -> Result<(), String> {
        let mut inner = self.inner.lock().unwrap_or_else(|error| error.into_inner());
        if inner.state != RecorderState::Idle {
            return Err("Stop the active recording before clearing it".to_string());
        }
        inner.session = None;
        inner.next_action_id = 1;
        Ok(())
    }

    #[cfg(target_os = "windows")]
    fn reset_after_start_error(&self) {
        let mut inner = self.inner.lock().unwrap_or_else(|error| error.into_inner());
        inner.state = RecorderState::Idle;
        inner.config = None;
        inner.session = None;
        inner.started = None;
    }
}

fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u64::MAX as u128) as u64
}

fn status_from(inner: &Arc<Mutex<Inner>>) -> RecorderStatus {
    let inner = inner.lock().unwrap_or_else(|error| error.into_inner());
    status_from_locked(&inner)
}

fn status_from_locked(inner: &Inner) -> RecorderStatus {
    let elapsed_ms = inner
        .started
        .map(|started| started.elapsed().as_millis().min(u64::MAX as u128) as u64)
        .or_else(|| inner.session.as_ref().map(|session| session.duration_ms))
        .unwrap_or(0);
    RecorderStatus {
        state: inner.state,
        session_id: inner.session.as_ref().map(|session| session.id.clone()),
        elapsed_ms,
        event_count: inner
            .session
            .as_ref()
            .map(|session| session.actions.len())
            .unwrap_or(0),
        target_label: inner.session.as_ref().map(|session| session.target.label()),
        reason: inner.reason.clone(),
        supported: cfg!(target_os = "windows"),
    }
}

fn target_bounds(target: &RecordingTarget) -> Result<CaptureBounds, String> {
    match target {
        RecordingTarget::Screen { index, .. } => screen_capture_bounds(*index),
        RecordingTarget::Window {
            hwnd,
            capture_area,
            frame_width,
            frame_height,
            ..
        } => {
            require_window_foreground(*hwnd)?;
            window_capture_bounds(*hwnd, *capture_area, *frame_width, *frame_height)
        }
    }
}

fn validate_target(target: &RecordingTarget) -> Result<(), String> {
    if let RecordingTarget::Window { hwnd, .. } = target {
        ensure_window_foreground(*hwnd)?;
    }
    let bounds = target_bounds(target)?;
    if bounds.width == 0 || bounds.height == 0 {
        Err("The recording target has no capture area".to_string())
    } else {
        Ok(())
    }
}

fn relative_point(bounds: CaptureBounds, x: i32, y: i32) -> Option<[i32; 2]> {
    let right = bounds
        .left
        .saturating_add(bounds.width.min(i32::MAX as u32) as i32);
    let bottom = bounds
        .top
        .saturating_add(bounds.height.min(i32::MAX as u32) as i32);
    (x >= bounds.left && x < right && y >= bounds.top && y < bottom)
        .then_some([x - bounds.left, y - bounds.top])
}

fn pointer_action(begin: [i32; 2], end: [i32; 2], duration: u64) -> Action {
    let distance = (end[0] - begin[0]).abs().max((end[1] - begin[1]).abs());
    if distance >= 8 {
        Action::Swipe {
            begin: Target::Coordinate(begin),
            begin_offset: None,
            end: Target::Coordinate(end),
            end_offset: None,
            duration: duration.max(1),
        }
    } else if duration >= 600 {
        Action::LongPress {
            target: Target::Coordinate(end),
            target_offset: None,
            duration,
        }
    } else {
        Action::Click {
            target: Target::Coordinate(end),
            target_offset: None,
        }
    }
}

fn append_action(
    inner: &Arc<Mutex<Inner>>,
    app: &AppHandle,
    action: Action,
    source: &str,
    warning: Option<String>,
    at: Instant,
) {
    let mut guard = inner.lock().unwrap_or_else(|error| error.into_inner());
    if guard.state != RecorderState::Recording {
        return;
    }
    let offset_ms = guard
        .started
        .map(|started| {
            at.saturating_duration_since(started)
                .as_millis()
                .min(u64::MAX as u128) as u64
        })
        .unwrap_or(0);
    let draft = RecordedActionDraft {
        id: guard.next_action_id,
        offset_ms,
        action,
        source: source.to_string(),
        warning,
    };
    guard.next_action_id += 1;
    if let Some(session) = guard.session.as_mut() {
        session.actions.push(draft.clone());
    }
    let status = status_from_locked(&guard);
    drop(guard);
    let _ = app.emit("recorder-action", &draft);
    let _ = app.emit("recorder-status", &status);
}

fn append_text_action(
    inner: &Arc<Mutex<Inner>>,
    app: &AppHandle,
    text: &str,
    capture_text: bool,
    at: Instant,
) {
    let mut guard = inner.lock().unwrap_or_else(|error| error.into_inner());
    if guard.state != RecorderState::Recording {
        return;
    }
    let offset_ms = guard
        .started
        .map(|started| {
            at.saturating_duration_since(started)
                .as_millis()
                .min(u64::MAX as u128) as u64
        })
        .unwrap_or(0);
    let warning = if capture_text {
        Some("Review recorded text before saving; keyboard layout and IME composition are not reconstructed.".to_string())
    } else {
        Some("Printable text was masked because typed-text capture is disabled.".to_string())
    };
    let mut updated = None;
    if let Some(session) = guard.session.as_mut() {
        if let Some(last) = session.actions.last_mut() {
            if last.source == "keyboard" && offset_ms.saturating_sub(last.offset_ms) <= 750 {
                if let Action::InputText { input_text } = &mut last.action {
                    if capture_text {
                        input_text.push_str(text);
                    }
                    last.warning = warning.clone();
                    updated = Some(last.clone());
                }
            }
        }
    }
    let draft = if let Some(draft) = updated {
        draft
    } else {
        let draft = RecordedActionDraft {
            id: guard.next_action_id,
            offset_ms,
            action: Action::InputText {
                input_text: if capture_text {
                    text.to_string()
                } else {
                    "{{recorded_text}}".to_string()
                },
            },
            source: "keyboard".to_string(),
            warning,
        };
        guard.next_action_id += 1;
        if let Some(session) = guard.session.as_mut() {
            session.actions.push(draft.clone());
        }
        draft
    };
    let status = status_from_locked(&guard);
    drop(guard);
    let _ = app.emit("recorder-action", &draft);
    let _ = app.emit("recorder-status", &status);
}

fn pause_for_target(inner: &Arc<Mutex<Inner>>, app: &AppHandle, error: String) {
    let mut guard = inner.lock().unwrap_or_else(|poison| poison.into_inner());
    if guard.state != RecorderState::Recording {
        return;
    }
    guard.state = RecorderState::Paused;
    guard.reason = Some(format!("Target unavailable: {error}"));
    let status = status_from_locked(&guard);
    drop(guard);
    let _ = app.emit("recorder-status", &status);
    let _ = app.emit("recorder-error", error);
}

fn process_events(receiver: mpsc::Receiver<RawInput>, inner: Arc<Mutex<Inner>>, app: AppHandle) {
    let mut mouse_down: Option<(i32, i32, Instant)> = None;
    let mut pressed = HashSet::<u32>::new();
    while let Ok(event) = receiver.recv() {
        let (state, config) = {
            let guard = inner.lock().unwrap_or_else(|error| error.into_inner());
            (guard.state, guard.config.clone())
        };
        if state != RecorderState::Recording {
            mouse_down = None;
            pressed.clear();
            continue;
        }
        let Some(config) = config else { continue };
        match event {
            RawInput::MouseDown { x, y, at } => mouse_down = Some((x, y, at)),
            RawInput::MouseUp { x, y, at } => {
                let Some((start_x, start_y, started)) = mouse_down.take() else {
                    continue;
                };
                let bounds = match target_bounds(&config.target) {
                    Ok(bounds) => bounds,
                    Err(error) => {
                        pause_for_target(&inner, &app, error);
                        continue;
                    }
                };
                let (Some(begin), Some(end)) = (
                    relative_point(bounds, start_x, start_y),
                    relative_point(bounds, x, y),
                ) else {
                    continue;
                };
                let duration = at
                    .saturating_duration_since(started)
                    .as_millis()
                    .min(u64::MAX as u128) as u64;
                let action = pointer_action(begin, end, duration);
                append_action(&inner, &app, action, "mouse", None, at);
            }
            RawInput::Wheel { x, y, delta, at } => {
                let bounds = match target_bounds(&config.target) {
                    Ok(bounds) => bounds,
                    Err(error) => {
                        pause_for_target(&inner, &app, error);
                        continue;
                    }
                };
                let Some(point) = relative_point(bounds, x, y) else {
                    continue;
                };
                append_action(
                    &inner,
                    &app,
                    Action::Scroll {
                        target: Target::Coordinate(point),
                        target_offset: None,
                        dx: 0,
                        dy: delta,
                    },
                    "wheel",
                    None,
                    at,
                );
            }
            RawInput::KeyDown { code, at } => {
                if !pressed.insert(code)
                    || config.excluded_key_codes.contains(&code)
                    || is_modifier(code)
                {
                    continue;
                }
                if let RecordingTarget::Window { hwnd, .. } = config.target {
                    if let Err(error) = require_window_foreground(hwnd) {
                        pause_for_target(&inner, &app, error);
                        continue;
                    }
                }
                let mut modifiers = pressed
                    .iter()
                    .copied()
                    .filter(|value| is_modifier(*value))
                    .collect::<Vec<_>>();
                modifiers.sort_unstable();
                let shift = modifiers
                    .iter()
                    .any(|value| matches!(*value, 0x10 | 0xA0 | 0xA1));
                let has_command_modifier = modifiers
                    .iter()
                    .any(|value| !matches!(*value, 0x10 | 0xA0 | 0xA1));
                if let Some(text) = (!has_command_modifier)
                    .then(|| printable_key(code, shift))
                    .flatten()
                {
                    append_text_action(&inner, &app, &text.to_string(), config.capture_text, at);
                } else {
                    let mut keys = modifiers
                        .into_iter()
                        .map(serde_json::Value::from)
                        .collect::<Vec<_>>();
                    keys.push(serde_json::Value::from(code));
                    append_action(
                        &inner,
                        &app,
                        Action::ClickKey { key: keys },
                        "keyboard",
                        None,
                        at,
                    );
                }
            }
            RawInput::KeyUp { code } => {
                pressed.remove(&code);
            }
        }
    }
}

fn is_modifier(code: u32) -> bool {
    matches!(code, 0x10 | 0x11 | 0x12 | 0x5B | 0x5C | 0xA0..=0xA5)
}

fn printable_key(code: u32, shift: bool) -> Option<char> {
    match code {
        0x41..=0x5A => char::from_u32(if shift { code } else { code + 32 }),
        0x30..=0x39 => char::from_u32(code),
        0x20 => Some(' '),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_pointer_to_relative_capture_coordinates() {
        let bounds = CaptureBounds {
            left: -100,
            top: 20,
            width: 500,
            height: 300,
        };
        assert_eq!(relative_point(bounds, -50, 80), Some([50, 60]));
        assert_eq!(relative_point(bounds, 401, 80), None);
    }

    #[test]
    fn masks_printable_keys_by_default_contract() {
        assert_eq!(printable_key(0x41, false), Some('a'));
        assert_eq!(printable_key(0x41, true), Some('A'));
        assert_eq!(printable_key(0x70, false), None);
    }

    #[test]
    fn classifies_pointer_gestures_without_losing_duration() {
        assert!(matches!(
            pointer_action([10, 10], [12, 11], 100),
            Action::Click { .. }
        ));
        assert!(matches!(
            pointer_action([10, 10], [12, 11], 700),
            Action::LongPress { duration: 700, .. }
        ));
        assert!(matches!(
            pointer_action([10, 10], [30, 20], 250),
            Action::Swipe { duration: 250, .. }
        ));
    }
}
