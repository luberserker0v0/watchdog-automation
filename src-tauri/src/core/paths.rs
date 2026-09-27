use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::Manager;

pub const LEGACY_USER_DATA_DIR_NAME: &str = ".watch_dog";

#[derive(Debug, Clone)]
struct AppPaths {
    app_data: PathBuf,
    app_local_data: PathBuf,
    app_cache: PathBuf,
    app_log: PathBuf,
    downloads: PathBuf,
    #[allow(dead_code)]
    resource: PathBuf,
    legacy_root: PathBuf,
}

static PATHS: OnceLock<AppPaths> = OnceLock::new();

pub fn init(app: &tauri::AppHandle) -> Result<(), String> {
    let resolver = app.path();
    let paths = AppPaths {
        app_data: resolver.app_data_dir().map_err(path_error("app data"))?,
        app_local_data: resolver
            .app_local_data_dir()
            .map_err(path_error("app local data"))?,
        app_cache: resolver.app_cache_dir().map_err(path_error("app cache"))?,
        app_log: resolver.app_log_dir().map_err(path_error("app log"))?,
        downloads: resolver.download_dir().map_err(path_error("downloads"))?,
        resource: resolver.resource_dir().map_err(path_error("resources"))?,
        legacy_root: resolver
            .home_dir()
            .map_err(path_error("home"))?
            .join(LEGACY_USER_DATA_DIR_NAME),
    };
    PATHS
        .set(paths)
        .map_err(|_| "Application paths were already initialized".to_string())?;
    ensure_user_dirs()
}

fn path_error(label: &'static str) -> impl FnOnce(tauri::Error) -> String {
    move |error| format!("Unable to resolve {label} directory: {error}")
}

fn paths() -> &'static AppPaths {
    #[cfg(test)]
    return PATHS.get_or_init(test_paths);

    #[cfg(not(test))]
    PATHS
        .get()
        .expect("application paths must be initialized during Tauri setup")
}

#[cfg(test)]
fn test_paths() -> AppPaths {
    let root = tempfile::tempdir()
        .expect("test application directory")
        .keep();
    let paths = AppPaths {
        app_data: root.join("data"),
        app_local_data: root.join("local"),
        app_cache: root.join("cache"),
        app_log: root.join("logs"),
        downloads: root.join("downloads"),
        resource: PathBuf::from(env!("CARGO_MANIFEST_DIR")),
        legacy_root: root.join("legacy"),
    };
    for directory in [
        &paths.app_data,
        &paths.app_local_data,
        &paths.app_cache,
        &paths.app_log,
        &paths.downloads,
    ] {
        std::fs::create_dir_all(directory).expect("test application directory");
    }
    paths
}

pub fn settings_store_path() -> PathBuf {
    paths().app_data.join("watchdog.settings.json")
}

pub fn user_data_dir() -> PathBuf {
    paths().app_local_data.clone()
}

pub fn user_resources_dir() -> PathBuf {
    paths().app_local_data.join("resources")
}

pub fn user_pipelines_dir() -> PathBuf {
    user_resources_dir().join("pipelines")
}

pub fn user_pipeline_nodes_dir() -> PathBuf {
    user_resources_dir().join("pipeline-nodes")
}

pub fn user_images_dir() -> PathBuf {
    user_resources_dir().join("images")
}

pub fn user_models_dir() -> PathBuf {
    user_resources_dir().join("models")
}

pub fn logs_dir() -> PathBuf {
    paths().app_log.clone()
}

pub fn visions_dir() -> PathBuf {
    paths().app_cache.join("visions")
}

pub fn vision_cleanup_marker() -> PathBuf {
    paths().app_cache.join(".visions-last-cleanup")
}

pub fn debug_visions_dir() -> PathBuf {
    visions_dir().join("debug")
}

pub fn downloads_dir() -> PathBuf {
    paths().downloads.clone()
}

#[cfg(debug_assertions)]
fn bundled_resources_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources")
}

#[cfg(not(debug_assertions))]
fn bundled_resources_dir() -> PathBuf {
    paths().resource.join("resources")
}

pub fn built_in_pipeline_nodes_dir() -> PathBuf {
    bundled_resources_dir().join("pipeline-nodes")
}

pub fn built_in_pipelines_dir() -> PathBuf {
    bundled_resources_dir().join("pipelines")
}

pub fn built_in_images_dir() -> PathBuf {
    bundled_resources_dir().join("images")
}

pub fn built_in_models_dir() -> PathBuf {
    bundled_resources_dir().join("models")
}

pub fn bundled_native_dir() -> PathBuf {
    #[cfg(debug_assertions)]
    {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("native")
            .join("windows-x64")
    }
    #[cfg(not(debug_assertions))]
    {
        paths().resource.join("native").join("windows-x64")
    }
}

pub fn ensure_user_dirs() -> Result<(), String> {
    for directory in [
        paths().app_data.clone(),
        paths().app_local_data.clone(),
        paths().app_cache.clone(),
        logs_dir(),
        user_pipelines_dir(),
        user_pipeline_nodes_dir(),
        user_images_dir(),
        user_models_dir(),
        visions_dir(),
        debug_visions_dir(),
    ] {
        std::fs::create_dir_all(&directory)
            .map_err(|error| format!("Unable to create {}: {error}", directory.display()))?;
    }
    Ok(())
}

/// Moves the pre-1.1 home-directory layout into Tauri application folders.
/// Existing destinations win; skipped or failed sources remain for recovery.
pub fn migrate_legacy_data() -> Vec<String> {
    migrate_legacy_tree(
        &paths().legacy_root,
        &user_resources_dir(),
        &visions_dir(),
        &logs_dir(),
        &vision_cleanup_marker(),
    )
}

fn migrate_legacy_tree(
    legacy_root: &Path,
    resources: &Path,
    visions: &Path,
    logs: &Path,
    cleanup_marker: &Path,
) -> Vec<String> {
    if !legacy_root.exists() {
        return Vec::new();
    }
    let mut warnings = Vec::new();
    for (source, destination) in [
        (legacy_root.join("resources"), resources.to_path_buf()),
        (legacy_root.join("pipelines"), resources.join("pipelines")),
        (legacy_root.join("visions"), visions.to_path_buf()),
        (legacy_root.join("logs"), logs.to_path_buf()),
    ] {
        move_entry_contents(&source, &destination, &mut warnings);
    }
    move_file(
        &legacy_root.join(".visions-last-cleanup"),
        cleanup_marker,
        &mut warnings,
    );
    remove_if_empty(legacy_root);
    warnings
}

fn move_entry_contents(source: &Path, destination: &Path, warnings: &mut Vec<String>) {
    if !source.exists() {
        return;
    }
    let Ok(metadata) = std::fs::symlink_metadata(source) else {
        warnings.push(format!(
            "Unable to inspect legacy path {}",
            source.display()
        ));
        return;
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        warnings.push(format!(
            "Skipped unsafe legacy directory {}",
            source.display()
        ));
        return;
    }
    if destination.exists() {
        match std::fs::symlink_metadata(destination) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
            Ok(_) => {
                warnings.push(format!(
                    "Skipped unsafe migration destination {}",
                    destination.display()
                ));
                return;
            }
            Err(error) => {
                warnings.push(format!(
                    "Unable to inspect {}: {error}",
                    destination.display()
                ));
                return;
            }
        }
    } else if let Err(error) = std::fs::create_dir_all(destination) {
        warnings.push(format!(
            "Unable to create migration destination {}: {error}",
            destination.display()
        ));
        return;
    }
    let entries = match std::fs::read_dir(source) {
        Ok(entries) => entries,
        Err(error) => {
            warnings.push(format!("Unable to read {}: {error}", source.display()));
            return;
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                warnings.push(format!("Unable to read legacy entry: {error}"));
                continue;
            }
        };
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let Ok(kind) = entry.file_type() else {
            warnings.push(format!("Unable to inspect {}", source_path.display()));
            continue;
        };
        if kind.is_symlink() {
            warnings.push(format!("Skipped legacy symlink {}", source_path.display()));
        } else if kind.is_dir() {
            move_entry_contents(&source_path, &destination_path, warnings);
        } else if kind.is_file() {
            move_file(&source_path, &destination_path, warnings);
        }
    }
    remove_if_empty(source);
}

fn move_file(source: &Path, destination: &Path, warnings: &mut Vec<String>) {
    if !source.exists() {
        return;
    }
    let metadata = match std::fs::symlink_metadata(source) {
        Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => metadata,
        Ok(_) => {
            warnings.push(format!("Skipped unsafe legacy file {}", source.display()));
            return;
        }
        Err(error) => {
            warnings.push(format!("Unable to inspect {}: {error}", source.display()));
            return;
        }
    };
    if destination.exists() {
        warnings.push(format!(
            "Kept legacy file because destination exists: {}",
            source.display()
        ));
        return;
    }
    let Some(parent) = destination.parent() else {
        warnings.push(format!(
            "Invalid migration destination {}",
            destination.display()
        ));
        return;
    };
    if let Err(error) = std::fs::create_dir_all(parent) {
        warnings.push(format!("Unable to create {}: {error}", parent.display()));
        return;
    }
    match std::fs::symlink_metadata(parent) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
        Ok(_) => {
            warnings.push(format!(
                "Skipped unsafe migration destination {}",
                parent.display()
            ));
            return;
        }
        Err(error) => {
            warnings.push(format!("Unable to inspect {}: {error}", parent.display()));
            return;
        }
    }
    remove_stale_migration_temps(parent, warnings);
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temporary = parent.join(format!(
        ".watchdog-migrate-{}-{nonce}.tmp",
        std::process::id()
    ));
    let result = (|| -> Result<(), String> {
        let copied = std::fs::copy(source, &temporary)
            .map_err(|error| format!("Unable to copy {}: {error}", source.display()))?;
        if copied != metadata.len() {
            return Err(format!(
                "Migration size mismatch for {}: expected {}, copied {copied}",
                source.display(),
                metadata.len()
            ));
        }
        std::fs::rename(&temporary, destination).map_err(|error| {
            format!(
                "Unable to finalize migration to {}: {error}",
                destination.display()
            )
        })?;
        std::fs::remove_file(source).map_err(|error| {
            format!("Copied but could not remove {}: {error}", source.display())
        })?;
        Ok(())
    })();
    if let Err(error) = result {
        let _ = std::fs::remove_file(&temporary);
        warnings.push(error);
    }
}

fn remove_stale_migration_temps(directory: &Path, warnings: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if !name.starts_with(".watchdog-migrate-") || !name.ends_with(".tmp") {
            continue;
        }
        let path = entry.path();
        match entry.file_type() {
            Ok(kind) if kind.is_file() && !kind.is_symlink() => {
                if let Err(error) = std::fs::remove_file(&path) {
                    warnings.push(format!(
                        "Unable to remove interrupted migration file {}: {error}",
                        path.display()
                    ));
                }
            }
            _ => warnings.push(format!(
                "Skipped unsafe interrupted migration entry {}",
                path.display()
            )),
        }
    }
}

fn remove_if_empty(directory: &Path) {
    let is_empty = std::fs::read_dir(directory)
        .ok()
        .is_some_and(|mut entries| entries.next().is_none());
    if is_empty {
        let _ = std::fs::remove_dir(directory);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_moves_known_data_without_overwriting_conflicts() {
        let temp = tempfile::tempdir().unwrap();
        let legacy = temp.path().join("legacy");
        let resources = temp.path().join("local/resources");
        let visions = temp.path().join("cache/visions");
        let logs = temp.path().join("logs");
        let marker = temp.path().join("cache/marker");
        std::fs::create_dir_all(legacy.join("resources/pipelines/demo")).unwrap();
        std::fs::create_dir_all(legacy.join("logs")).unwrap();
        std::fs::create_dir_all(&logs).unwrap();
        std::fs::create_dir_all(marker.parent().unwrap()).unwrap();
        std::fs::write(legacy.join("resources/pipelines/demo/demo.md"), "new").unwrap();
        std::fs::write(legacy.join("logs/old.log"), "legacy").unwrap();
        std::fs::write(&marker, "current").unwrap();
        std::fs::write(legacy.join(".visions-last-cleanup"), "legacy").unwrap();

        let warnings = migrate_legacy_tree(&legacy, &resources, &visions, &logs, &marker);

        assert_eq!(
            std::fs::read_to_string(resources.join("pipelines/demo/demo.md")).unwrap(),
            "new"
        );
        assert_eq!(
            std::fs::read_to_string(logs.join("old.log")).unwrap(),
            "legacy"
        );
        assert_eq!(std::fs::read_to_string(&marker).unwrap(), "current");
        assert!(legacy.join(".visions-last-cleanup").exists());
        assert!(warnings
            .iter()
            .any(|warning| warning.contains("destination exists")));
    }

    #[test]
    fn migration_is_idempotent_after_success() {
        let temp = tempfile::tempdir().unwrap();
        let legacy = temp.path().join("legacy");
        let resources = temp.path().join("resources");
        std::fs::create_dir_all(legacy.join("pipelines/flow")).unwrap();
        std::fs::write(legacy.join("pipelines/flow/flow.md"), "flowchart TD").unwrap();
        assert!(migrate_legacy_tree(
            &legacy,
            &resources,
            &temp.path().join("visions"),
            &temp.path().join("logs"),
            &temp.path().join("marker")
        )
        .is_empty());
        assert!(migrate_legacy_tree(
            &legacy,
            &resources,
            &temp.path().join("visions"),
            &temp.path().join("logs"),
            &temp.path().join("marker")
        )
        .is_empty());
    }

    #[test]
    fn migration_skips_symlinks() {
        let temp = tempfile::tempdir().unwrap();
        let legacy = temp.path().join("legacy");
        std::fs::create_dir_all(legacy.join("visions")).unwrap();
        let outside = temp.path().join("outside.txt");
        std::fs::write(&outside, "keep").unwrap();
        let link = legacy.join("visions/link.txt");
        #[cfg(windows)]
        if std::os::windows::fs::symlink_file(&outside, &link).is_err() {
            return;
        }
        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, &link).unwrap();
        let warnings = migrate_legacy_tree(
            &legacy,
            &temp.path().join("resources"),
            &temp.path().join("visions"),
            &temp.path().join("logs"),
            &temp.path().join("marker"),
        );
        assert_eq!(std::fs::read_to_string(outside).unwrap(), "keep");
        assert!(warnings.iter().any(|warning| warning.contains("symlink")));
    }

    #[test]
    fn migration_recovers_from_an_interrupted_temp_copy_and_preserves_binary_data() {
        let temp = tempfile::tempdir().unwrap();
        let legacy = temp.path().join("legacy");
        let resources = temp.path().join("resources");
        std::fs::create_dir_all(legacy.join("resources/images")).unwrap();
        std::fs::create_dir_all(resources.join("images")).unwrap();
        let stale = resources.join("images/.watchdog-migrate-1-1.tmp");
        std::fs::write(&stale, b"partial").unwrap();
        let bytes = [0, 159, 146, 150, 255];
        std::fs::write(legacy.join("resources/images/raw.bin"), bytes).unwrap();

        let warnings = migrate_legacy_tree(
            &legacy,
            &resources,
            &temp.path().join("visions"),
            &temp.path().join("logs"),
            &temp.path().join("marker"),
        );

        assert!(warnings.is_empty());
        assert!(!stale.exists());
        assert_eq!(
            std::fs::read(resources.join("images/raw.bin")).unwrap(),
            bytes
        );
    }
}
