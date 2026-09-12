use crate::modules::pipeline::models::constants::USER_DATA_DIR_NAME;
use std::path::PathBuf;
use std::sync::OnceLock;
use tauri::Manager;

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

pub fn user_pipelines_dir() -> PathBuf {
    user_data_dir().join("pipelines")
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
    bundled_resources_dir()
        .join("build-in")
        .join("pipeline-nodes")
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
    bundled_resources_dir().join("build-in").join("pipelines")
}

pub fn resource_images_dir() -> PathBuf {
    bundled_resources_dir().join("images")
}

pub fn resource_models_dir() -> PathBuf {
    bundled_resources_dir().join("models")
}

pub fn ensure_user_dirs() {
    let base = user_data_dir();
    std::fs::create_dir_all(base.join("pipelines")).ok();
    std::fs::create_dir_all(base.join("logs")).ok();
    std::fs::create_dir_all(base.join("visions")).ok();
    std::fs::create_dir_all(base.join("visions").join("debug")).ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_pipelines_only_use_the_build_in_resource_tree() {
        let path = built_in_pipelines_dir();
        assert!(path.ends_with(PathBuf::from("resources/build-in/pipelines")));
        assert!(!path.ends_with(PathBuf::from("resources/pipelines")));
    }
}
