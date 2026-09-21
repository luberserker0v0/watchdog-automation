//! Startup retention for generated evidence. User resources are never touched.

use chrono::{Local, NaiveDate};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MAX_LOG_FILES: usize = 30;
const VISION_CLEANUP_INTERVAL: Duration = Duration::from_secs(7 * 24 * 60 * 60);
const VISION_CLEANUP_MARKER: &str = ".visions-last-cleanup";

pub fn cleanup_on_startup() {
    let base = crate::core::paths::user_data_dir();
    let now = SystemTime::now();
    match retain_recent_logs(&base, now) {
        Ok(removed) if removed > 0 => log::info!("[maintenance] removed {removed} old log files"),
        Ok(_) => {}
        Err(error) => log::warn!("[maintenance] log retention failed: {error}"),
    }
    match clean_visions_if_due(&base, now) {
        Ok(Some(removed)) => {
            log::info!("[maintenance] cleared {removed} vision entries; next check in 7 days")
        }
        Ok(None) => {}
        Err(error) => log::warn!("[maintenance] vision cleanup failed: {error}"),
    }
}

fn owned_directory(base: &Path, name: &str) -> Result<(PathBuf, PathBuf), String> {
    let base = base
        .canonicalize()
        .map_err(|error| format!("Cannot resolve {}: {error}", base.display()))?;
    let directory = base.join(name);
    let metadata = fs::symlink_metadata(&directory)
        .map_err(|error| format!("Cannot inspect {}: {error}", directory.display()))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(format!(
            "Expected a regular directory: {}",
            directory.display()
        ));
    }
    let resolved = directory
        .canonicalize()
        .map_err(|error| format!("Cannot resolve {}: {error}", directory.display()))?;
    if resolved != directory {
        return Err(format!(
            "Directory escapes its user-data location: {}",
            directory.display()
        ));
    }
    Ok((directory, resolved))
}

fn retain_recent_logs(base: &Path, now: SystemTime) -> Result<usize, String> {
    let (directory, resolved) = owned_directory(base, "logs")?;
    let current_name = format!(
        "{}.log",
        chrono::DateTime::<Local>::from(now).format("%Y-%m-%d")
    );
    let mut files = Vec::<(NaiveDate, PathBuf)>::new();
    for entry in fs::read_dir(&directory)
        .map_err(|error| format!("Cannot list {}: {error}", directory.display()))?
    {
        let entry = entry.map_err(|error| format!("Cannot read log entry: {error}"))?;
        let path = entry.path();
        let kind = entry
            .file_type()
            .map_err(|error| format!("Cannot inspect {}: {error}", path.display()))?;
        if !kind.is_file() || kind.is_symlink() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let Some(stem) = name.strip_suffix(".log") else {
            continue;
        };
        let Ok(date) = NaiveDate::parse_from_str(stem, "%Y-%m-%d") else {
            continue;
        };
        files.push((date, path));
    }
    files.sort_by(|left, right| right.0.cmp(&left.0));
    let active_exists = files.iter().any(|(_, path)| {
        path.file_name()
            .is_some_and(|name| name == current_name.as_str())
    });
    let mut other_slots = MAX_LOG_FILES - usize::from(active_exists);
    let mut removed = 0;
    for (_, path) in files {
        if path
            .file_name()
            .is_some_and(|name| name == current_name.as_str())
        {
            continue;
        }
        if other_slots > 0 {
            other_slots -= 1;
            continue;
        }
        let actual = path
            .canonicalize()
            .map_err(|error| format!("Cannot resolve {}: {error}", path.display()))?;
        if !actual.starts_with(&resolved) || actual.parent() != Some(resolved.as_path()) {
            return Err(format!(
                "Log path escapes its directory: {}",
                path.display()
            ));
        }
        fs::remove_file(&path)
            .map_err(|error| format!("Cannot remove {}: {error}", path.display()))?;
        removed += 1;
    }
    Ok(removed)
}

fn clean_visions_if_due(base: &Path, now: SystemTime) -> Result<Option<usize>, String> {
    let (directory, resolved) = owned_directory(base, "visions")?;
    let marker = base.join(VISION_CLEANUP_MARKER);
    if let Ok(metadata) = fs::symlink_metadata(&marker) {
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err(format!(
                "Invalid vision cleanup marker: {}",
                marker.display()
            ));
        }
    }
    let now_seconds = now
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("Invalid system clock: {error}"))?
        .as_secs();
    if let Ok(last_seconds) = fs::read_to_string(&marker)
        .unwrap_or_default()
        .trim()
        .parse::<u64>()
    {
        if Duration::from_secs(now_seconds.saturating_sub(last_seconds)) < VISION_CLEANUP_INTERVAL {
            return Ok(None);
        }
    }

    let mut removed = 0;
    for entry in fs::read_dir(&directory)
        .map_err(|error| format!("Cannot list {}: {error}", directory.display()))?
    {
        let entry = entry.map_err(|error| format!("Cannot read vision entry: {error}"))?;
        let path = entry.path();
        if remove_vision_entry(&path, &resolved)? {
            removed += 1;
        }
    }
    let debug = directory.join("debug");
    if debug.exists() {
        let kind = fs::symlink_metadata(&debug)
            .map_err(|error| format!("Cannot inspect {}: {error}", debug.display()))?;
        if kind.file_type().is_symlink() || !kind.is_dir() {
            return Err(format!(
                "Invalid vision debug directory: {}",
                debug.display()
            ));
        }
    } else {
        fs::create_dir(&debug)
            .map_err(|error| format!("Cannot recreate vision debug directory: {error}"))?;
    }
    fs::write(&marker, now_seconds.to_string())
        .map_err(|error| format!("Cannot update {}: {error}", marker.display()))?;
    Ok(Some(removed))
}

/// Remove a generated entry without traversing links, including links nested
/// inside a run directory. A directory containing a skipped link is retained.
fn remove_vision_entry(path: &Path, root: &Path) -> Result<bool, String> {
    let kind = fs::symlink_metadata(path)
        .map_err(|error| format!("Cannot inspect {}: {error}", path.display()))?;
    if kind.file_type().is_symlink() {
        log::warn!("[maintenance] skipped vision symlink: {}", path.display());
        return Ok(false);
    }
    let actual = path
        .canonicalize()
        .map_err(|error| format!("Cannot resolve {}: {error}", path.display()))?;
    if !actual.starts_with(root) || actual == root {
        return Err(format!(
            "Vision path escapes its directory: {}",
            path.display()
        ));
    }
    if kind.is_dir() {
        let mut can_remove = true;
        for entry in fs::read_dir(path)
            .map_err(|error| format!("Cannot list {}: {error}", path.display()))?
        {
            let entry = entry.map_err(|error| format!("Cannot read vision entry: {error}"))?;
            can_remove &= remove_vision_entry(&entry.path(), root)?;
        }
        if !can_remove {
            return Ok(false);
        }
        fs::remove_dir(path)
            .map_err(|error| format!("Cannot remove {}: {error}", path.display()))?;
    } else if kind.is_file() {
        fs::remove_file(path)
            .map_err(|error| format!("Cannot remove {}: {error}", path.display()))?;
    } else {
        return Ok(false);
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_current_and_newest_dated_logs() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir(temp.path().join("logs")).unwrap();
        let now = UNIX_EPOCH + Duration::from_secs(1_780_000_000);
        let current = format!(
            "{}.log",
            chrono::DateTime::<Local>::from(now).format("%Y-%m-%d")
        );
        fs::write(temp.path().join("logs").join(&current), "active").unwrap();
        let first = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        for offset in 0..35 {
            let name = format!("{}.log", first + chrono::Duration::days(offset));
            fs::write(temp.path().join("logs").join(name), "old").unwrap();
        }
        fs::write(temp.path().join("logs").join("notes.txt"), "keep").unwrap();
        let removed = retain_recent_logs(temp.path(), now).unwrap();
        assert_eq!(removed, 6);
        assert!(temp.path().join("logs").join(current).exists());
        assert!(temp.path().join("logs").join("notes.txt").exists());
    }

    #[test]
    fn clears_visions_only_when_interval_is_due() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir(temp.path().join("visions")).unwrap();
        fs::write(temp.path().join("visions").join("capture.jpg"), "image").unwrap();
        let now = UNIX_EPOCH + Duration::from_secs(1_780_000_000);
        assert_eq!(clean_visions_if_due(temp.path(), now).unwrap(), Some(1));
        assert!(!temp.path().join("visions").join("capture.jpg").exists());
        assert!(temp.path().join("visions").join("debug").is_dir());
        fs::write(temp.path().join("visions").join("new.jpg"), "image").unwrap();
        assert_eq!(
            clean_visions_if_due(temp.path(), now + Duration::from_secs(6 * 24 * 60 * 60)).unwrap(),
            None
        );
        assert!(temp.path().join("visions").join("new.jpg").exists());
        assert_eq!(
            clean_visions_if_due(temp.path(), now + VISION_CLEANUP_INTERVAL).unwrap(),
            Some(2)
        );
        assert!(!temp.path().join("visions").join("new.jpg").exists());
    }

    #[test]
    fn vision_cleanup_never_follows_a_nested_link() {
        let temp = tempfile::tempdir().unwrap();
        let visions = temp.path().join("visions");
        let run = visions.join("patrol").join("run");
        fs::create_dir_all(&run).unwrap();
        let outside = temp.path().join("outside.txt");
        fs::write(&outside, "keep").unwrap();
        let link = run.join("external.txt");
        #[cfg(windows)]
        if let Err(error) = std::os::windows::fs::symlink_file(&outside, &link) {
            if error.kind() == std::io::ErrorKind::PermissionDenied
                || error.raw_os_error() == Some(1314)
            {
                return;
            }
            panic!("Cannot create test symlink: {error}");
        }
        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, &link).unwrap();

        let now = UNIX_EPOCH + Duration::from_secs(1_780_000_000);
        clean_visions_if_due(temp.path(), now).unwrap();
        assert_eq!(fs::read_to_string(outside).unwrap(), "keep");
        assert!(link.exists());
    }
}
