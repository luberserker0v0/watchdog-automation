use chrono::Local;
use std::fs;
use std::fs::OpenOptions;

pub fn init() {
    let log_dir = crate::core::paths::logs_dir();
    fs::create_dir_all(&log_dir).ok();

    let log_path = log_dir.join(format!("{}.log", Local::now().format("%Y-%m-%d")));

    let file = match OpenOptions::new().create(true).append(true).open(&log_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to open log file {}: {}", log_path.display(), e);
            return;
        }
    };

    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}] [{}] {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Info)
        .chain(std::io::stdout())
        .chain(file)
        .apply()
        .ok();
}
