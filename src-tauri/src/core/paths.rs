use std::path::PathBuf;
use std::sync::OnceLock;
use tauri::Manager;

pub const USER_DATA_DIR_NAME: &str = ".watch_dog";

static RESOURCE_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn init_resource_dir(app: &tauri::AppHandle) {
    match app.path().resource_dir() {
        Ok(dir) => {
            let _ = RESOURCE_DIR.set(dir);
        }
        Err(e) => {
            log::error!("Cannot get resource dir: {}", e);
            let _ =
                RESOURCE_DIR.set(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        }
    }
}

pub fn user_data_dir() -> PathBuf {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .to_string_lossy()
                .into_owned()
        });
    PathBuf::from(home).join(USER_DATA_DIR_NAME)
}

pub fn user_resources_dir() -> PathBuf {
    user_data_dir().join("resources")
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

#[cfg(debug_assertions)]
fn bundled_resources_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources")
}

#[cfg(not(debug_assertions))]
fn bundled_resources_dir() -> PathBuf {
    RESOURCE_DIR
        .get()
        .cloned()
        .unwrap_or_else(|| {
            log::warn!("Resource dir not initialized, falling back to current dir");
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        })
        .join("resources")
}

pub fn built_in_pipeline_nodes_dir() -> PathBuf {
    bundled_resources_dir().join("pipeline-nodes")
}

pub fn logs_dir() -> PathBuf {
    user_data_dir().join("logs")
}

pub fn visions_dir() -> PathBuf {
    user_data_dir().join("visions")
}

pub fn debug_visions_dir() -> PathBuf {
    visions_dir().join("debug")
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

fn migrate_legacy_user_pipelines(base: &std::path::Path) {
    let legacy = base.join("pipelines");
    let destination = base.join("resources").join("pipelines");
    if !legacy.is_dir() {
        return;
    }

    if let Err(error) = std::fs::create_dir_all(&destination) {
        log::warn!(
            "Unable to create user resources directory for pipeline migration: {}",
            error
        );
        return;
    }

    let Ok(entries) = std::fs::read_dir(&legacy) else {
        log::warn!(
            "Unable to read legacy pipeline directory: {}",
            legacy.display()
        );
        return;
    };
    for entry in entries.flatten() {
        let source = entry.path();
        let target = destination.join(entry.file_name());
        if target.exists() {
            log::warn!(
                "Keeping legacy pipeline because its destination already exists: {}",
                source.display()
            );
            continue;
        }
        match std::fs::rename(&source, &target) {
            Ok(()) => log::info!(
                "Migrated user pipeline from {} to {}",
                source.display(),
                target.display()
            ),
            Err(error) => log::warn!(
                "Unable to migrate user pipeline from {} to {}: {}",
                source.display(),
                target.display(),
                error
            ),
        }
    }
    let _ = std::fs::remove_dir(&legacy);
}

pub fn ensure_user_dirs() {
    let base = user_data_dir();
    migrate_legacy_user_pipelines(&base);
    std::fs::create_dir_all(user_pipelines_dir()).ok();
    std::fs::create_dir_all(user_pipeline_nodes_dir()).ok();
    std::fs::create_dir_all(user_images_dir()).ok();
    std::fs::create_dir_all(user_models_dir()).ok();
    std::fs::create_dir_all(base.join("logs")).ok();
    std::fs::create_dir_all(base.join("visions")).ok();
    std::fs::create_dir_all(base.join("visions").join("debug")).ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary_user_root(test_name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "watch-dog-path-test-{}-{}-{}",
            test_name,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn built_in_pipelines_use_the_bundled_resource_root() {
        let path = built_in_pipelines_dir();
        assert!(path.ends_with(PathBuf::from("resources/pipelines")));
        assert!(!path.to_string_lossy().contains("build-in"));
    }

    #[test]
    fn user_resources_are_nested_below_the_user_resource_tree() {
        let resources = user_resources_dir();
        assert!(user_pipelines_dir().starts_with(&resources));
        assert!(user_pipeline_nodes_dir().starts_with(&resources));
        assert!(user_images_dir().starts_with(&resources));
        assert!(user_models_dir().starts_with(&resources));
        assert!(!user_pipelines_dir().ends_with(PathBuf::from(".watch_dog/pipelines")));
    }

    #[test]
    fn legacy_user_pipelines_move_below_resources_without_overwriting() {
        let base = temporary_user_root("migration");
        let legacy = base.join("pipelines");
        let destination = base.join("resources").join("pipelines");
        std::fs::create_dir_all(legacy.join("LegacyFlow")).unwrap();
        std::fs::write(
            legacy.join("LegacyFlow").join("LegacyFlow.md"),
            "flowchart TD",
        )
        .unwrap();

        migrate_legacy_user_pipelines(&base);

        assert!(destination
            .join("LegacyFlow")
            .join("LegacyFlow.md")
            .is_file());
        assert!(!legacy.exists());
        std::fs::remove_dir_all(base).unwrap();
    }
}
