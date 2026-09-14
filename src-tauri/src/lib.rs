mod commands;
mod error;
mod models;
pub mod modules;
mod repositories;
mod services;
pub mod utils;

use commands::stream::StreamState;
use services::stream::StreamService;
use std::sync::{Arc, Mutex};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

pub static STREAM_FRAME: Mutex<Option<Vec<u8>>> = Mutex::new(None);
pub static FRAME_CACHE: Mutex<Option<services::stream::StreamFrame>> = Mutex::new(None);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    run_app(tauri::generate_context!())
}

pub fn run_tester() {
    log::info!("[exe] Running as TESTER");
    run_app(tauri::generate_context!())
}

pub fn run_target() {
    log::info!("[exe] Running as TARGET");
    utils::paths::ensure_user_dirs();
    utils::logger::init();

    // Production opens the combined Target UI. The node tester can request an
    // isolated page by launching this same executable with a window label.
    let target_label = match std::env::args().nth(1).as_deref() {
        Some("login") => "login",
        Some("dashboard") => "dashboard",
        Some("settings") => "settings",
        _ => "main",
    };
    log::info!("[exe] Starting target window, label={}", target_label);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            let (title, url, disable_password_manager) = match target_label {
                "login" => ("Target UI - Login", "index-login.html", true),
                "dashboard" => ("Target UI - Dashboard", "index-dashboard.html", false),
                "settings" => ("Target UI - Settings", "index-settings.html", false),
                _ => ("Target UI", "index-target.html", true),
            };

            let test_window_height = match target_label {
                "dashboard" | "settings" => 820.0,
                _ => 600.0,
            };
            let mut builder =
                WebviewWindowBuilder::new(app, target_label, WebviewUrl::App(url.into()))
                    .title(title)
                    .inner_size(500.0, test_window_height);

            if disable_password_manager {
                builder = builder.additional_browser_args(
                    "--disable-features=PasswordManager,PasswordManagerOnboarding",
                );
            }

            // Create only the requested window. Defining all variants in the
            // config made WebView2 show them briefly before setup could close them.
            let window = builder.build()?;
            let app_handle = app.handle().clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { .. } = event {
                    app_handle.exit(0);
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![])
        .run(tauri::generate_context!())
        .expect("error while running target application");
}

fn run_app(context: tauri::Context<tauri::Wry>) {
    utils::paths::ensure_user_dirs();
    utils::logger::init();
    log::info!("Starting watch-dog application");

    let stream_service = Arc::new(StreamService::new());
    let stream_service_for_setup = stream_service.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .register_uri_scheme_protocol("watchdog", |_ctx, request| {
            let path = request.uri().path();
            if path == "/frame" {
                let frame = STREAM_FRAME.lock().unwrap_or_else(|e| e.into_inner());
                match frame.as_ref() {
                    Some(jpeg) => tauri::http::Response::builder()
                        .header("Content-Type", "image/jpeg")
                        .header("Cache-Control", "no-cache, no-store, must-revalidate")
                        .body(jpeg.clone())
                        .unwrap(),
                    None => tauri::http::Response::builder()
                        .status(404)
                        .body(vec![])
                        .unwrap(),
                }
            } else {
                tauri::http::Response::builder()
                    .status(404)
                    .body(vec![])
                    .unwrap()
            }
        })
        .setup(move |app| {
            utils::paths::init_resource_dir(app.handle());

            repositories::virtual_desktop::get_desktop_manager();

            if let Some(window) = app.get_webview_window("main") {
                let svc = stream_service_for_setup.clone();
                let app_handle = app.handle().clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { .. } = event {
                        svc.stop_sync();
                        for (name, w) in app_handle.webview_windows() {
                            if name != "main" {
                                let _ = w.close();
                            }
                        }
                        app_handle.exit(0);
                    }
                });
            }

            Ok(())
        })
        .manage(StreamState(stream_service))
        .invoke_handler(tauri::generate_handler![
            commands::screen::get_screens,
            commands::screen::capture_screen,
            commands::screen::save_capture,
            commands::screen::save_region_capture,
            commands::window::get_windows,
            commands::window::capture_window_by_hwnd,
            commands::window::get_window_details,
            commands::window::resize_window_client,
            modules::pipeline::commands::editor::load_pipeline,
            modules::pipeline::commands::editor::save_pipeline,
            modules::pipeline::commands::editor::save_pipeline_layout,
            modules::pipeline::commands::editor::validate_pipeline,
            modules::pipeline::commands::editor::list_pipelines,
            modules::pipeline::commands::runner::start_pipeline,
            modules::pipeline::commands::runner::stop_pipeline,
            modules::pipeline::commands::runner::pause_pipeline,
            modules::pipeline::commands::runner::get_pipeline_status,
            modules::pipeline::commands::runner::list_running_pipelines,
            modules::pipeline::commands::runner::test_action,
            modules::pipeline::commands::runner::test_recognition,
            modules::pipeline::commands::runner::set_recognition_device,
            commands::stream::start_stream,
            commands::stream::stop_stream,
            commands::stream::get_stream_status,
            commands::desktop::get_desktops,
            commands::desktop::get_current_desktop,
            commands::target::open_target_window,
            commands::node_test::list_node_metas,
            commands::node_test::open_window,
            commands::node_test::test_node,
        ])
        .run(context)
        .expect("error while running tauri application");
}
