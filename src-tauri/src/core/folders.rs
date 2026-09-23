use std::path::PathBuf;
use tauri::{Manager, Runtime};
use tauri_plugin_opener::OpenerExt;

#[derive(Clone, serde::Serialize)]
pub struct RelatedFolder {
    pub key: &'static str,
    pub name: &'static str,
    pub purpose: &'static str,
    pub path: String,
}

fn known_folders<R: Runtime>(
    app: &tauri::AppHandle<R>,
) -> Vec<(&'static str, &'static str, &'static str, PathBuf)> {
    let visions = super::paths::visions_dir();
    let resources = super::paths::user_resources_dir();
    let downloads = app
        .path()
        .download_dir()
        .unwrap_or_else(|_| super::paths::user_data_dir());
    vec![
        (
            "data",
            ".watch_dog",
            "Application data root",
            super::paths::user_data_dir(),
        ),
        (
            "pipelines",
            "pipelines",
            "User-authored pipeline sources",
            super::paths::user_pipelines_dir(),
        ),
        (
            "pipeline_nodes",
            "pipeline-nodes",
            "Reusable node metadata",
            super::paths::user_pipeline_nodes_dir(),
        ),
        (
            "images",
            "images",
            "User recognition images and templates",
            super::paths::user_images_dir(),
        ),
        (
            "models",
            "models",
            "User-provided recognition models",
            super::paths::user_models_dir(),
        ),
        (
            "logs",
            "logs",
            "Application and execution logs",
            super::paths::logs_dir(),
        ),
        (
            "visions",
            "visions",
            "Generated captures and run evidence",
            visions.clone(),
        ),
        (
            "debug",
            "visions/debug",
            "Annotated recognition and action debug images",
            super::paths::debug_visions_dir(),
        ),
        (
            "patrol",
            "visions/patrol",
            "Per-run Patrol reference screenshots",
            visions.join("patrol"),
        ),
        (
            "downloads",
            "Downloads",
            "Action Recorder JSON exports",
            downloads,
        ),
        (
            "resources",
            "resources",
            "Root for user-editable pipelines, nodes, images, and models",
            resources,
        ),
    ]
}

#[tauri::command]
pub fn list_related_folders(app: tauri::AppHandle) -> Vec<RelatedFolder> {
    known_folders(&app)
        .into_iter()
        .map(|(key, name, purpose, path)| RelatedFolder {
            key,
            name,
            purpose,
            path: path.to_string_lossy().into_owned(),
        })
        .collect()
}

#[tauri::command]
pub fn open_related_folder(key: String, app: tauri::AppHandle) -> Result<(), String> {
    let (_, _, _, path) = known_folders(&app)
        .into_iter()
        .find(|(candidate, _, _, _)| *candidate == key)
        .ok_or_else(|| format!("Unknown related folder '{key}'"))?;
    std::fs::create_dir_all(&path)
        .map_err(|error| format!("Unable to create {}: {error}", path.display()))?;
    app.opener()
        .open_path(path.to_string_lossy(), None::<String>)
        .map_err(|error| format!("Unable to open folder: {error}"))
}
