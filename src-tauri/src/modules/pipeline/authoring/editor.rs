use crate::core::paths;
use crate::modules::pipeline::authoring::models::edge::EdgeType;
use crate::modules::pipeline::authoring::models::graph::{PipelineGraph, PipelineTarget};
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

pub fn validate_pipeline_sources(
    mermaid: &parser::mermaid::MermaidGraph,
    metadata: &parser::meta::MetadataFile,
) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    // Validate instance references point to existing node templates
    if let Some(instances) = &metadata.instances {
        for (instance_id, instance) in instances {
            if !metadata.nodes.contains_key(&instance.node) {
                errors.push(ValidationError {
                    error_type: "invalid_instance_reference".to_string(),
                    message: format!(
                        "Instance '{}' references non-existent node '{}'",
                        instance_id, instance.node
                    ),
                });
            }
        }
    }

    // Check that every mermaid node has metadata (either via instance or node lookup)
    let flow_ids: std::collections::HashSet<&str> =
        mermaid.nodes.iter().map(|node| node.id.as_str()).collect();
    let instance_ids: std::collections::HashSet<&str> = metadata
        .instances
        .as_ref()
        .map(|instances| instances.keys().map(String::as_str).collect())
        .unwrap_or_default();
    let metadata_keys: std::collections::HashSet<&str> =
        metadata.nodes.keys().map(String::as_str).collect();

    for id in flow_ids.difference(&instance_ids) {
        // Node ID not in instances, check if label is in nodes
        let node = mermaid.nodes.iter().find(|n| n.id == *id).unwrap();
        if !metadata_keys.contains(node.label.as_str()) && !metadata_keys.contains(node.id.as_str())
        {
            errors.push(ValidationError {
                error_type: "missing_node_metadata".to_string(),
                message: format!("Mermaid node '{}' has no metadata", id),
            });
        }
    }

    errors
}

pub fn format_validation_errors(errors: &[ValidationError]) -> String {
    errors
        .iter()
        .map(|error| format!("{}: {}", error.error_type, error.message))
        .collect::<Vec<_>>()
        .join("; ")
}

pub fn validate_layout_nodes(
    layout: &parser::layout::PipelineLayout,
    topology: &parser::mermaid::MermaidGraph,
) -> Result<(), String> {
    if let Some(node_id) = layout
        .nodes
        .keys()
        .find(|node_id| !topology.nodes.iter().any(|node| node.id == **node_id))
    {
        return Err(format!(
            "Invalid pipeline layout: node '{node_id}' is not in Mermaid topology"
        ));
    }
    Ok(())
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

pub fn validate_pipeline_graph(graph: &PipelineGraph) -> Vec<ValidationError> {
    use crate::modules::pipeline::authoring::models::node::NodeKind;
    use crate::modules::pipeline::authoring::models::recognizer::Recognition;
    let mut errors = Vec::new();

    if let Some(launch) = &graph.launch {
        match (&launch.exe, &launch.cmd) {
            (Some(exe), None) if !exe.trim().is_empty() => {}
            (None, Some(cmd)) if !cmd.trim().is_empty() => {}
            (Some(_), Some(_)) => errors.push(ValidationError {
                error_type: "invalid_launch".to_string(),
                message: "launch must specify exactly one of exe or cmd, not both".to_string(),
            }),
            _ => errors.push(ValidationError {
                error_type: "invalid_launch".to_string(),
                message: "launch must specify a non-empty exe or cmd".to_string(),
            }),
        }
        if let Some(PipelineTarget::Window { .. }) = graph.target.as_ref() {
            if !graph
                .target
                .as_ref()
                .is_some_and(PipelineTarget::has_window_filters)
            {
                errors.push(ValidationError {
                    error_type: "launch_requires_window_filter".to_string(),
                    message: "launch with a window target requires process, class, or title so an existing target can be reused and a launched target can be verified".to_string(),
                });
            }
        }
    }

    if let Some(size) = graph
        .target
        .as_ref()
        .and_then(|target| target.window_size())
    {
        if size[0] == 0 || size[1] == 0 {
            errors.push(ValidationError {
                error_type: "invalid_window_size".to_string(),
                message: "Top-level target.window_size dimensions must both be positive"
                    .to_string(),
            });
        }
    }

    if graph.nodes.is_empty() {
        errors.push(ValidationError {
            error_type: "empty_graph".to_string(),
            message: "Pipeline has no nodes".to_string(),
        });
    }

    let mut seen_ids = std::collections::HashSet::new();
    for node in &graph.nodes {
        if !seen_ids.insert(&node.id) {
            errors.push(ValidationError {
                error_type: "duplicate_node".to_string(),
                message: format!("Duplicate node ID: {}", node.id),
            });
        }
        if node.repeat == 0 {
            errors.push(ValidationError {
                error_type: "invalid_repeat".to_string(),
                message: format!("Node '{}' repeat must be at least 1", node.id),
            });
        }
        if node
            .target
            .as_ref()
            .and_then(|target| target.window_size())
            .is_some()
        {
            errors.push(ValidationError {
                error_type: "node_window_size_not_supported".to_string(),
                message: format!(
                    "Node '{}' cannot declare window_size; place it on the top-level target",
                    node.id
                ),
            });
        }
    }

    let node_ids: std::collections::HashSet<&str> =
        graph.nodes.iter().map(|n| n.id.as_str()).collect();
    for node in &graph.nodes {
        if let Some(
            crate::modules::pipeline::authoring::models::recognizer::RecognitionRoi::Node(id),
        ) = node.recognition.roi.as_ref()
        {
            if id == &node.id || !node_ids.contains(id.as_str()) {
                errors.push(ValidationError {
                    error_type: "invalid_roi_reference".to_string(),
                    message: format!(
                        "Node '{}' ROI must reference a different existing node; received '{}'",
                        node.id, id
                    ),
                });
            }
        }
    }
    for edge in &graph.edges {
        if !node_ids.contains(edge.from.as_str()) {
            errors.push(ValidationError {
                error_type: "invalid_edge".to_string(),
                message: format!("Edge references non-existent 'from' node: {}", edge.from),
            });
        }
        if !node_ids.contains(edge.to.as_str()) {
            errors.push(ValidationError {
                error_type: "invalid_edge".to_string(),
                message: format!("Edge references non-existent 'to' node: {}", edge.to),
            });
        }
    }

    for node in &graph.nodes {
        let error_count = graph
            .edges
            .iter()
            .filter(|edge| edge.from == node.id && edge.edge_type == EdgeType::Error)
            .count();
        if error_count > 1 {
            errors.push(ValidationError {
                error_type: "ambiguous_transition".to_string(),
                message: format!(
                    "Node '{}' has {} error edges; only one is supported",
                    node.id, error_count
                ),
            });
        }
        // Entry nodes carry no recognition or actions: the engine skips
        // straight to their children. Declared actions would never run.
        if node.kind == crate::modules::pipeline::authoring::models::node::NodeKind::Entry
            && !node.actions.is_empty()
        {
            errors.push(ValidationError {
                error_type: "entry_node_has_actions".to_string(),
                message: format!(
                    "Entry node '{}' must not declare actions; they never execute",
                    node.id
                ),
            });
        }
        if node.kind == NodeKind::Endpoint {
            if !node.actions.is_empty() {
                errors.push(ValidationError {
                    error_type: "endpoint_node_has_actions".to_string(),
                    message: format!("Endpoint node '{}' must not declare actions", node.id),
                });
            }
            if !matches!(node.recognition.kind, Recognition::DirectHit)
                || node.recognition.roi.is_some()
                || node.recognition.roi_offset.is_some()
                || node.recognition.candidate_selection.is_some()
            {
                errors.push(ValidationError {
                    error_type: "endpoint_node_has_recognition".to_string(),
                    message: format!(
                        "Endpoint node '{}' must not declare recognition settings",
                        node.id
                    ),
                });
            }
            if graph.edges.iter().any(|edge| edge.from == node.id) {
                errors.push(ValidationError {
                    error_type: "endpoint_node_has_outgoing_edges".to_string(),
                    message: format!("Endpoint node '{}' must not have outgoing edges", node.id),
                });
            }
        }
    }

    if !graph
        .nodes
        .iter()
        .any(|node| node.kind == NodeKind::Endpoint)
    {
        errors.push(ValidationError {
            error_type: "missing_endpoint_node".to_string(),
            message: "Pipeline must declare at least one Endpoint node (node kind \"Endpoint\")"
                .to_string(),
        });
    }

    // Exactly one explicit Entry-kind node is required, mirroring plan
    // entry resolution. Missing marker and duplicate markers are distinct
    // error codes.
    let explicit_entries: Vec<&str> = graph
        .nodes
        .iter()
        .filter(|node| {
            node.kind == crate::modules::pipeline::authoring::models::node::NodeKind::Entry
        })
        .map(|node| node.id.as_str())
        .collect();
    let entries: Vec<&str> = if explicit_entries.is_empty() {
        if !graph.nodes.is_empty() {
            errors.push(ValidationError {
                error_type: "missing_entry_node".to_string(),
                message:
                    "Pipeline must declare exactly one Entry node (node kind \"Entry\"); found none"
                        .to_string(),
            });
        }
        Vec::new()
    } else {
        if explicit_entries.len() != 1 {
            errors.push(ValidationError {
                error_type: "invalid_entry_count".to_string(),
                message: format!(
                    "Pipeline must have exactly one Entry node; found {}",
                    explicit_entries.len()
                ),
            });
        }
        explicit_entries
    };

    if let Some(entry) = entries.first() {
        let mut reachable = std::collections::HashSet::new();
        let mut pending = vec![*entry];
        while let Some(id) = pending.pop() {
            if !reachable.insert(id) {
                continue;
            }
            pending.extend(
                graph
                    .edges
                    .iter()
                    .filter(|edge| edge.from == id)
                    .map(|edge| edge.to.as_str()),
            );
        }
        for node in &graph.nodes {
            if !reachable.contains(node.id.as_str()) {
                errors.push(ValidationError {
                    error_type: "unreachable_node".to_string(),
                    message: format!("Node '{}' is unreachable from entry '{}'", node.id, entry),
                });
            }
        }
    }

    if has_cycle(&graph) {
        errors.push(ValidationError {
            error_type: "cycle_detected".to_string(),
            message: "Pipeline contains a cycle".to_string(),
        });
    }

    errors
}

fn has_cycle(graph: &PipelineGraph) -> bool {
    use std::collections::{HashMap, HashSet};

    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for edge in &graph.edges {
        adj.entry(&edge.from).or_default().push(&edge.to);
    }

    let mut visited = HashSet::new();
    let mut rec_stack = HashSet::new();

    fn dfs<'a>(
        node: &'a str,
        adj: &HashMap<&'a str, Vec<&'a str>>,
        visited: &mut HashSet<&'a str>,
        rec_stack: &mut HashSet<&'a str>,
    ) -> bool {
        visited.insert(node);
        rec_stack.insert(node);

        if let Some(neighbors) = adj.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    if dfs(neighbor, adj, visited, rec_stack) {
                        return true;
                    }
                } else if rec_stack.contains(neighbor) {
                    return true;
                }
            }
        }

        rec_stack.remove(node);
        false
    }

    for node in graph.nodes.iter().map(|n| n.id.as_str()) {
        if !visited.contains(node) && dfs(node, &adj, &mut visited, &mut rec_stack) {
            return true;
        }
    }

    false
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
