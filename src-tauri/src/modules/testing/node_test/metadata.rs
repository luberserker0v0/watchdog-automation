use super::NodeMetaInfo;
use crate::core::paths;

pub(super) const NODE_META_SUFFIX: &str = ".meta.json";

pub(super) fn node_meta_file_name(path: &std::path::Path) -> Option<String> {
    path.file_name()?
        .to_str()?
        .strip_suffix(NODE_META_SUFFIX)
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
}

pub(super) fn read_node_metadata(
    path: &std::path::Path,
) -> Result<crate::modules::pipeline::authoring::parser::meta::MetadataFile, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    crate::modules::pipeline::authoring::parser::meta::parse(&content)
        .map_err(|e| format!("Failed to deserialize {}: {}", path.display(), e))
}

#[tauri::command]
pub(super) fn discover_node_metas() -> Result<Vec<NodeMetaInfo>, String> {
    let built_in_dir = paths::built_in_pipeline_nodes_dir();
    let custom_dir = paths::user_pipeline_nodes_dir();
    log::info!(
        "[node_test] listing node metas from {} and {}",
        built_in_dir.display(),
        custom_dir.display()
    );
    let mut nodes = Vec::new();
    append_node_metas(&built_in_dir, "built-in", &mut nodes)?;
    append_node_metas(&custom_dir, "custom", &mut nodes)?;
    nodes.sort_by(|a, b| (&a.source, &a.id).cmp(&(&b.source, &b.id)));
    log::info!("[node_test] found {} node metas", nodes.len());
    Ok(nodes)
}

fn append_node_metas(
    dir: &std::path::Path,
    source: &str,
    nodes: &mut Vec<NodeMetaInfo>,
) -> Result<(), String> {
    if !dir.exists() {
        return Ok(());
    }

    let entries = std::fs::read_dir(&dir).map_err(|e| {
        let msg = format!("Failed to read node metas dir: {}", e);
        log::warn!("[node_test] {}", msg);
        msg
    })?;

    for entry in entries.flatten() {
        let path = entry.path();
        let Some(file_name) = node_meta_file_name(&path) else {
            continue;
        };

        let metadata = read_node_metadata(&path)?;
        let default_target = metadata.target;
        nodes.extend(metadata.nodes.into_iter().map(|(id, mut config)| {
            if config.target.is_none() {
                config.target = default_target.clone();
            }
            NodeMetaInfo {
                id,
                file_name: file_name.clone(),
                source: source.to_string(),
                config,
                params: Vec::new(),
            }
        }));
    }
    Ok(())
}
