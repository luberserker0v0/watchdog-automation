use chrono::Local;
use tauri::Runtime;
use tauri_plugin_log::{RotationStrategy, Target, TargetKind, TimezoneStrategy};

const MAX_LOG_FILE_SIZE: u128 = 10 * 1024 * 1024;

pub fn plugin<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    let dated_name = Local::now().format("%Y-%m-%d").to_string();
    tauri_plugin_log::Builder::new()
        .clear_targets()
        .targets([
            Target::new(TargetKind::Stdout),
            Target::new(TargetKind::LogDir {
                file_name: Some(dated_name),
            }),
        ])
        .level(log::LevelFilter::Info)
        .max_file_size(MAX_LOG_FILE_SIZE)
        .rotation_strategy(RotationStrategy::KeepAll)
        .timezone_strategy(TimezoneStrategy::UseLocal)
        .build()
}
