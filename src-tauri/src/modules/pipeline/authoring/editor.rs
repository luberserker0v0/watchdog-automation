mod validation;

pub use validation::{
    format_validation_errors, validate_layout_nodes, validate_pipeline_graph,
    validate_pipeline_sources,
};

use crate::core::paths;
use crate::modules::pipeline::authoring::models::graph::PipelineGraph;
use crate::modules::pipeline::authoring::parser;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LoadResult {
    pub graph: PipelineGraph,
    pub mermaid: String,
    pub meta: String,
    pub layout: Option<parser::layout::PipelineLayout>,
    pub layout_writable: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ValidationError {
    pub error_type: String,
    pub message: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PipelineInfo {
    pub name: String,
    pub path: String,
    pub source: String,
}

#[tauri::command]
pub fn list_pipelines() -> Result<Vec<PipelineInfo>, String> {
    let mut pipelines = Vec::new();

    let built_in = paths::built_in_pipelines_dir();
    log::info!("Scanning built-in pipelines: {}", built_in.display());
    scan_pipelines_dir(&built_in, "built-in", &mut pipelines);

    let user = paths::user_pipelines_dir();
    log::info!("Scanning user pipelines: {}", user.display());
    scan_pipelines_dir(&user, "custom", &mut pipelines);

    log::info!("Found {} pipeline(s)", pipelines.len());
    Ok(pipelines)
}

#[tauri::command]
pub fn load_pipeline(path: String) -> Result<LoadResult, String> {
    log::info!("Loading pipeline: {}", path);
    let base_path = std::path::Path::new(&path);
    let parent = base_path.parent().ok_or_else(|| {
        let msg = "Invalid path".to_string();
        log::error!("load_pipeline: {}", msg);
        msg
    })?;
    let stem = base_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| {
            let msg = "Invalid filename".to_string();
            log::error!("load_pipeline: {}", msg);
            msg
        })?;

    let mermaid_path = parent.join(format!("{}.md", stem));
    let meta_path = parent.join(format!("{}.meta.json", stem));
    let layout_path = parent.join(format!("{}.layout.json", stem));

    let mermaid_content = std::fs::read_to_string(&mermaid_path).map_err(|e| {
        let msg = format!("Failed to read {}: {}", mermaid_path.display(), e);
        log::error!("load_pipeline: {}", msg);
        msg
    })?;
    let meta_content = std::fs::read_to_string(&meta_path).map_err(|e| {
        let msg = format!("Failed to read {}: {}", meta_path.display(), e);
        log::error!("load_pipeline: {}", msg);
        msg
    })?;

    let mermaid_graph = parser::mermaid::parse(&mermaid_content).map_err(|e| {
        let msg = format!("Failed to parse Mermaid: {}", e);
        log::error!("load_pipeline: {}", msg);
        msg
    })?;
    let metadata = parser::meta::parse(&meta_content).map_err(|e| {
        let msg = format!("Failed to parse {}: {}", meta_path.display(), e);
        log::error!("load_pipeline: {}", msg);
        msg
    })?;
    let layout = if layout_path.is_file() {
        let parsed =
            parser::layout::parse(&std::fs::read_to_string(&layout_path).map_err(|e| {
                let msg = format!("Failed to read {}: {}", layout_path.display(), e);
                log::error!("load_pipeline: {}", msg);
                msg
            })?)
            .map_err(|error| format!("Invalid {}: {error}", layout_path.display()))?;
        validate_layout_nodes(&parsed, &mermaid_graph)
            .map_err(|error| format!("Invalid {}: {error}", layout_path.display()))?;
        Some(parsed)
    } else {
        None
    };

    let graph = parser::build_graph(&mermaid_graph, &metadata);
    let mut validation = validate_pipeline_sources(&mermaid_graph, &metadata);
    validation.extend(validate_pipeline_graph(&graph));
    if !validation.is_empty() {
        let msg = format!(
            "Invalid pipeline: {}",
            format_validation_errors(&validation)
        );
        log::error!("load_pipeline: {}", msg);
        return Err(msg);
    }

    let layout_writable = mermaid_path
        .canonicalize()
        .ok()
        .zip(paths::user_pipelines_dir().canonicalize().ok())
        .is_some_and(|(pipeline, custom_root)| pipeline.starts_with(custom_root));

    log::info!("Successfully loaded pipeline: {}", path);
    Ok(LoadResult {
        graph,
        mermaid: mermaid_content,
        meta: meta_content,
        layout,
        layout_writable,
    })
}

#[tauri::command]
pub fn save_pipeline_layout(
    path: String,
    positions: std::collections::HashMap<String, parser::layout::NodePosition>,
) -> Result<bool, String> {
    let base_path = std::path::Path::new(&path);
    let parent = base_path.parent().ok_or("Invalid pipeline path")?;
    let stem = base_path
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or("Invalid pipeline filename")?;
    let meta_path = parent.join(format!("{}.meta.json", stem));
    let canonical_meta = meta_path
        .canonicalize()
        .map_err(|error| format!("Failed to resolve {}: {}", meta_path.display(), error))?;
    let custom_root = paths::user_pipelines_dir()
        .canonicalize()
        .map_err(|error| format!("Failed to resolve custom pipeline directory: {}", error))?;
    if !canonical_meta.starts_with(&custom_root) {
        return Err("Built-in pipeline layouts are read-only. Copy the pipeline into the custom resources directory before saving a shareable layout.".to_string());
    }
    let layout_path = canonical_meta
        .parent()
        .ok_or("Invalid resolved pipeline path")?
        .join(format!("{}.layout.json", stem));
    if let Ok(metadata) = std::fs::symlink_metadata(&layout_path) {
        if metadata.file_type().is_symlink() {
            return Err("Pipeline layout destination cannot be a symlink".into());
        }
    }

    let content = std::fs::read_to_string(&canonical_meta)
        .map_err(|error| format!("Failed to read {}: {}", canonical_meta.display(), error))?;
    parser::meta::parse(&content)
        .map_err(|error| format!("Invalid {}: {error}", canonical_meta.display()))?;
    let mermaid_path = canonical_meta
        .parent()
        .ok_or("Invalid pipeline directory")?
        .join(format!("{stem}.md"));
    let mermaid = std::fs::read_to_string(&mermaid_path)
        .map_err(|error| format!("Failed to read {}: {error}", mermaid_path.display()))?;
    let topology = parser::mermaid::parse(&mermaid)?;
    validate_layout_nodes(
        &parser::layout::PipelineLayout {
            nodes: positions.clone(),
        },
        &topology,
    )?;
    let serialized = parser::layout::to_json(&parser::layout::PipelineLayout { nodes: positions })
        .map_err(|error| format!("Failed to serialize pipeline layout: {}", error))?;
    parser::layout::parse(&serialized)?;
    std::fs::write(&layout_path, serialized)
        .map_err(|error| format!("Failed to write {}: {}", layout_path.display(), error))?;
    Ok(true)
}

fn save_source_paths(
    root: &std::path::Path,
    path: &std::path::Path,
) -> Result<(std::path::PathBuf, std::path::PathBuf), String> {
    if !path.is_absolute() || path.extension().and_then(|value| value.to_str()) != Some("md") {
        return Err(
            "Pipeline path must be an absolute .md file under the custom pipeline directory".into(),
        );
    }
    if path
        .components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
        || !path.starts_with(root)
    {
        return Err("Pipeline path must stay within the custom pipeline directory".into());
    }
    std::fs::create_dir_all(root)
        .map_err(|error| format!("Failed to create custom pipeline directory: {error}"))?;
    let canonical_root = root.canonicalize().map_err(|error| error.to_string())?;
    let parent = path.parent().ok_or("Invalid pipeline path")?;
    let relative_parent = parent
        .strip_prefix(root)
        .map_err(|error| error.to_string())?;
    let mut canonical_parent = canonical_root.clone();
    for component in relative_parent.components() {
        let std::path::Component::Normal(name) = component else {
            return Err("Invalid pipeline directory component".into());
        };
        let next = canonical_parent.join(name);
        if next.exists() {
            canonical_parent = next.canonicalize().map_err(|error| error.to_string())?;
            if !canonical_parent.starts_with(&canonical_root) {
                return Err("Pipeline path resolves outside the custom pipeline directory".into());
            }
        } else {
            std::fs::create_dir(&next)
                .map_err(|error| format!("Failed to create pipeline directory: {error}"))?;
            canonical_parent = next;
        }
    }
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or("Invalid pipeline filename")?;
    let mermaid = canonical_parent.join(format!("{stem}.md"));
    let metadata = canonical_parent.join(format!("{stem}.meta.json"));
    for destination in [&mermaid, &metadata] {
        if let Ok(metadata) = std::fs::symlink_metadata(destination) {
            if metadata.file_type().is_symlink()
                || !destination
                    .canonicalize()
                    .map_err(|error| error.to_string())?
                    .starts_with(&canonical_root)
            {
                return Err(format!("Pipeline destination is a symlink or resolves outside the custom directory: {}", destination.display()));
            }
        }
    }
    Ok((mermaid, metadata))
}

#[tauri::command]
pub fn save_pipeline(
    path: String,
    mermaid: Option<String>,
    meta: Option<String>,
) -> Result<bool, String> {
    let mermaid = mermaid.ok_or(
        "save_pipeline requires Mermaid and metadata sources; graph-only saves are not supported",
    )?;
    let meta = meta.ok_or(
        "save_pipeline requires Mermaid and metadata sources; graph-only saves are not supported",
    )?;
    save_pipeline_sources(
        &paths::user_pipelines_dir(),
        std::path::Path::new(&path),
        &mermaid,
        &meta,
    )
}

fn save_pipeline_sources(
    root: &std::path::Path,
    path: &std::path::Path,
    mermaid: &str,
    meta: &str,
) -> Result<bool, String> {
    let topology = parser::mermaid::parse(&mermaid)
        .map_err(|error| format!("Invalid Mermaid source: {error}"))?;
    let metadata = parser::meta::parse(&meta).map_err(|error| {
        format!(
            "Invalid {}: {error}",
            path.with_extension("meta.json").display()
        )
    })?;
    let graph = parser::build_graph(&topology, &metadata);
    let mut errors = validate_pipeline_sources(&topology, &metadata);
    errors.extend(validate_pipeline_graph(&graph));
    if !errors.is_empty() {
        return Err(format!(
            "Invalid pipeline: {}",
            format_validation_errors(&errors)
        ));
    }
    let (mermaid_path, meta_path) = save_source_paths(root, path)?;
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or("Invalid pipeline filename")?;
    let layout_path = mermaid_path
        .parent()
        .ok_or("Invalid pipeline directory")?
        .join(format!("{stem}.layout.json"));
    if layout_path.exists() {
        let layout = parser::layout::parse(
            &std::fs::read_to_string(&layout_path)
                .map_err(|error| format!("Failed to read {}: {error}", layout_path.display()))?,
        )
        .map_err(|error| format!("Invalid {}: {error}", layout_path.display()))?;
        validate_layout_nodes(&layout, &topology)
            .map_err(|error| format!("Invalid {}: {error}", layout_path.display()))?;
    }
    std::fs::write(&mermaid_path, mermaid)
        .map_err(|error| format!("Failed to write {}: {error}", mermaid_path.display()))?;
    std::fs::write(&meta_path, meta)
        .map_err(|error| format!("Failed to write {}: {error}", meta_path.display()))?;
    Ok(true)
}

#[tauri::command]
pub fn validate_pipeline(graph: PipelineGraph) -> Result<Vec<ValidationError>, String> {
    Ok(validate_pipeline_graph(&graph))
}

fn scan_pipelines_dir(dir: &std::path::Path, source: &str, out: &mut Vec<PipelineInfo>) {
    if !dir.exists() {
        log::warn!(
            "Pipeline directory does not exist: {} ({})",
            dir.display(),
            source
        );
        return;
    }
    let Ok(read_dir) = std::fs::read_dir(dir) else {
        log::warn!(
            "Failed to read pipeline directory: {} ({})",
            dir.display(),
            source
        );
        return;
    };
    for entry in read_dir.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let mermaid_file = std::fs::read_dir(&path).ok().and_then(|rd| {
            rd.flatten().find(|entry| {
                entry
                    .path()
                    .extension()
                    .map(|ext| ext == "md")
                    .unwrap_or(false)
            })
        });
        if let Some(mermaid_entry) = mermaid_file {
            let stem = mermaid_entry
                .path()
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            let pipeline_path = path.join(&stem);
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            log::info!("Found pipeline '{}' at {}", name, pipeline_path.display());
            out.push(PipelineInfo {
                name,
                path: pipeline_path.to_string_lossy().to_string(),
                source: source.to_string(),
            });
        }
    }
}

#[cfg(test)]
mod validation_tests {
    use super::*;

    #[test]
    fn save_preserves_metadata_sources_and_rejects_escape() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("pipelines");
        let path = root.join("example.md");
        let mermaid = "flowchart TD\nStart[\"Start\"]\nDone[\"Done\"]\nStart --> Done";
        let meta = r#"{"launch":{"exe":"demo.exe"},"nodes":{"Template":{"kind":"Entry"},"Done":{"kind":"Endpoint"}},"instances":{"Start":{"node":"Template","param_values":{"name":"Alice"}}}}"#;
        save_pipeline_sources(&root, &path, mermaid, meta).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), mermaid);
        assert_eq!(
            std::fs::read_to_string(root.join("example.meta.json")).unwrap(),
            meta
        );
        let loaded = load_pipeline(path.to_string_lossy().to_string()).unwrap();
        assert_eq!(loaded.mermaid, mermaid);
        assert_eq!(loaded.meta, meta);
        assert!(save_source_paths(&root, &root.join("../escape.md")).is_err());
        assert!(save_source_paths(&root, &temp.path().join("outside.md")).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn save_rejects_symlink_directory_escape() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("pipelines");
        let outside = temp.path().join("outside");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        let link = root.join("linked");
        if let Err(error) = std::os::windows::fs::symlink_dir(&outside, &link) {
            if error.kind() == std::io::ErrorKind::PermissionDenied
                || error.raw_os_error() == Some(1314)
            {
                return;
            }
            panic!("Failed to create test symlink: {error}");
        }
        assert!(save_source_paths(&root, &link.join("escape.md")).is_err());
    }

    fn parse_graph(
        flow: &str,
        meta: &str,
    ) -> (
        parser::mermaid::MermaidGraph,
        parser::meta::MetadataFile,
        PipelineGraph,
    ) {
        let mermaid = parser::mermaid::parse(flow).unwrap();
        let metadata = parser::meta::parse(meta).unwrap();
        let graph = parser::build_graph(&mermaid, &metadata);
        (mermaid, metadata, graph)
    }

    #[test]
    fn rejects_missing_node_metadata() {
        let (flow, metadata, _) = parse_graph(
            "flowchart TD\nA[\"A\"]",
            r#"{"nodes":{"B":{"recognition":{"type":"DirectHit"}}}}"#,
        );
        let errors = validate_pipeline_sources(&flow, &metadata);
        assert!(errors
            .iter()
            .any(|error| error.error_type == "missing_node_metadata"));
        assert!(!errors
            .iter()
            .any(|error| error.error_type == "orphan_node_metadata"));
    }

    #[test]
    fn resolves_id_metadata_when_labeled_node_has_no_label_template() {
        let (flow, metadata, graph) = parse_graph(
            "flowchart TD\nA[\"Readable label\"]\nDone[\"Done\"]\nA --> Done",
            r#"{"nodes":{"A":{"kind":"Entry","desc":"ID template"},"Done":{"kind":"Endpoint"}}}"#,
        );
        assert!(validate_pipeline_sources(&flow, &metadata).is_empty());
        assert_eq!(graph.nodes[0].desc, "ID template");
        assert!(validate_pipeline_graph(&graph).is_empty());
    }

    #[test]
    fn requires_endpoint_and_accepts_multiple_endpoints() {
        let (_, _, missing) = parse_graph(
            "flowchart TD\nStart --> Work",
            r#"{"nodes":{"Start":{"kind":"Entry"},"Work":{}}}"#,
        );
        assert!(validate_pipeline_graph(&missing)
            .iter()
            .any(|error| error.error_type == "missing_endpoint_node"));

        let (_, _, multiple) = parse_graph(
            "flowchart TD\nStart --> DoneA\nStart --> DoneB",
            r#"{"nodes":{"Start":{"kind":"Entry"},"DoneA":{"kind":"Endpoint"},"DoneB":{"kind":"Endpoint"}}}"#,
        );
        assert!(validate_pipeline_graph(&multiple).is_empty());
    }

    #[test]
    fn load_rejects_pipeline_without_endpoint() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("missing.md");
        std::fs::write(&path, "flowchart TD\nStart --> Work").unwrap();
        std::fs::write(
            temp.path().join("missing.meta.json"),
            r#"{"nodes":{"Start":{"kind":"Entry"},"Work":{}}}"#,
        )
        .unwrap();
        let error = load_pipeline(path.to_string_lossy().to_string()).unwrap_err();
        assert!(error.contains("missing_endpoint_node"), "{error}");
    }

    #[test]
    fn endpoint_rejects_recognition_actions_and_outgoing_edges() {
        let (_, _, graph) = parse_graph(
            "flowchart TD\nStart --> Done\nDone --> Work",
            r#"{"nodes":{"Start":{"kind":"Entry"},"Done":{"kind":"Endpoint","recognition":{"type":"OCR"},"actions":[{"type":"DoNothing"}]},"Work":{}}}"#,
        );
        let errors = validate_pipeline_graph(&graph);
        for code in [
            "endpoint_node_has_recognition",
            "endpoint_node_has_actions",
            "endpoint_node_has_outgoing_edges",
        ] {
            assert!(
                errors.iter().any(|error| error.error_type == code),
                "missing {code}: {errors:?}"
            );
        }
    }

    #[test]
    fn rejects_missing_entry_node() {
        let (_, _, graph) = parse_graph(
            "flowchart TD\nA[\"A\"]\nB[\"B\"]\nA --> B",
            r#"{"nodes":{"A":{"recognition":{"type":"DirectHit"}},"B":{"recognition":{"type":"DirectHit"}}}}"#,
        );
        let errors = validate_pipeline_graph(&graph);
        assert!(errors
            .iter()
            .any(|error| error.error_type == "missing_entry_node"));
    }

    #[test]
    fn rejects_actions_on_entry_node() {
        let (_, _, graph) = parse_graph(
            "flowchart TD\nA[\"A\"]\nB[\"B\"]\nA --> B",
            r#"{"nodes":{"A":{"kind":"Entry","actions":[{"type":"DoNothing"}]},"B":{"recognition":{"type":"DirectHit"}}}}"#,
        );
        let errors = validate_pipeline_graph(&graph);
        assert!(errors
            .iter()
            .any(|error| error.error_type == "entry_node_has_actions"));
    }

    #[test]
    fn rejects_multiple_entries_and_unreachable_nodes() {
        let (_, _, graph) = parse_graph(
            "flowchart TD\nA[\"A\"]\nB[\"B\"]",
            r#"{"nodes":{"A":{"kind":"Entry"},"B":{"kind":"Entry"}}}"#,
        );
        let errors = validate_pipeline_graph(&graph);
        assert!(errors
            .iter()
            .any(|error| error.error_type == "invalid_entry_count"));
    }

    #[test]
    fn rejects_zero_repeat() {
        let error = parser::meta::parse(
            r#"{"nodes":{"A":{"recognition":{"type":"DirectHit"},"repeat":0}}}"#,
        )
        .unwrap_err();
        assert!(error.contains("/nodes/A/repeat"), "{error}");
        assert!(error.contains("minimum of 1"), "{error}");
    }

    #[test]
    fn rejects_ambiguous_error_edges() {
        let (_, _, graph) = parse_graph(
            "flowchart TD\nA[\"A\"]\nB[\"B\"]\nC[\"C\"]\nA --> B\nA -->|error| C\nA -->|error| B",
            r#"{"nodes":{"A":{"recognition":{"type":"DirectHit"}},"B":{"recognition":{"type":"DirectHit"}},"C":{"recognition":{"type":"DirectHit"}}}}"#,
        );
        let errors = validate_pipeline_graph(&graph);
        assert!(errors
            .iter()
            .any(|error| error.error_type == "ambiguous_transition"));
    }

    #[test]
    fn validates_window_size_only_on_the_top_level_target() {
        let mermaid = parser::mermaid::parse("flowchart TD\nA[\"A\"]").unwrap();
        // Bypass the schema here to exercise the graph validator's defensive
        // checks. Normal file loading rejects the node-level field earlier.
        let metadata: parser::meta::MetadataFile = serde_json::from_str(
            r#"{"target":{"type":"window","title":"Target","window_size":[0,720]},"nodes":{"A":{"target":{"type":"window","capture_area":"client","window_size":[1280,720]},"recognition":{"type":"DirectHit"}}}}"#,
        )
        .unwrap();
        let graph = parser::build_graph(&mermaid, &metadata);
        let errors = validate_pipeline_graph(&graph);
        assert!(errors
            .iter()
            .any(|error| error.error_type == "invalid_window_size"));
        assert!(errors
            .iter()
            .any(|error| error.error_type == "node_window_size_not_supported"));
    }

    #[test]
    fn validates_launch_command_and_window_identity() {
        let (_, _, invalid_command) = parse_graph(
            "flowchart TD\nA[\"A\"]",
            r#"{"launch":{"exe":"a.exe","cmd":"a"},"nodes":{"A":{}}}"#,
        );
        assert!(validate_pipeline_graph(&invalid_command)
            .iter()
            .any(|error| error.error_type == "invalid_launch"));

        let (_, _, filterless_window) = parse_graph(
            "flowchart TD\nA[\"A\"]",
            r#"{"target":{"type":"window","capture_area":"client"},"launch":{"exe":"a.exe"},"nodes":{"A":{}}}"#,
        );
        assert!(validate_pipeline_graph(&filterless_window)
            .iter()
            .any(|error| error.error_type == "launch_requires_window_filter"));

        let (_, _, valid) = parse_graph(
            "flowchart TD\nA[\"A\"]",
            r#"{"target":{"type":"window","title":"^Target$"},"launch":{"exe":"a.exe","wait_for_window":true},"nodes":{"A":{}}}"#,
        );
        assert!(!validate_pipeline_graph(&valid)
            .iter()
            .any(|error| error.error_type.starts_with("invalid_launch")
                || error.error_type == "launch_requires_window_filter"));
    }

    #[test]
    fn allows_multiple_next_edges_for_children() {
        let (_, _, graph) = parse_graph(
            "flowchart TD\nA[\"A\"]\nB[\"B\"]\nC[\"C\"]\nA --> B\nA --> C",
            r#"{"nodes":{"A":{"recognition":{"type":"DirectHit"}},"B":{"recognition":{"type":"DirectHit"}},"C":{"recognition":{"type":"DirectHit"}}}}"#,
        );
        let errors = validate_pipeline_graph(&graph);
        assert!(!errors
            .iter()
            .any(|e| e.error_type == "ambiguous_transition"));
    }

    #[test]
    fn allows_children_with_error_edge() {
        let (_, _, graph) = parse_graph(
            "flowchart TD\nA[\"A\"]\nB[\"B\"]\nC[\"C\"]\nD[\"D\"]\nA --> B\nA --> C\nA -->|error| D",
            r#"{"nodes":{"A":{"recognition":{"type":"DirectHit"}},"B":{"recognition":{"type":"DirectHit"}},"C":{"recognition":{"type":"DirectHit"}},"D":{"recognition":{"type":"DirectHit"}}}}"#,
        );
        let errors = validate_pipeline_graph(&graph);
        assert!(!errors
            .iter()
            .any(|e| e.error_type == "ambiguous_transition"));
        assert_eq!(graph.edges.len(), 3);
    }
}
