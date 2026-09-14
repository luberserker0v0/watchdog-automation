use crate::modules::pipeline::models::edge::EdgeType;
use crate::modules::pipeline::models::graph::PipelineGraph;
use crate::modules::pipeline::parser;
use crate::utils::paths;

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
    let flow_ids: std::collections::HashSet<&str> =
        mermaid.nodes.iter().map(|node| node.id.as_str()).collect();
    let metadata_ids: std::collections::HashSet<&str> =
        metadata.nodes.keys().map(String::as_str).collect();
    for id in flow_ids.difference(&metadata_ids) {
        errors.push(ValidationError {
            error_type: "missing_node_metadata".to_string(),
            message: format!("Mermaid node '{}' has no metadata", id),
        });
    }
    for id in metadata_ids.difference(&flow_ids) {
        errors.push(ValidationError {
            error_type: "orphan_node_metadata".to_string(),
            message: format!("Metadata node '{}' is not present in Mermaid", id),
        });
    }
    let flow_edges: std::collections::HashSet<String> = mermaid
        .edges
        .iter()
        .map(|edge| format!("{}->{}", edge.from, edge.to))
        .collect();
    for key in metadata.edges.keys() {
        if !flow_edges.contains(key) {
            errors.push(ValidationError {
                error_type: "orphan_edge_metadata".to_string(),
                message: format!("Metadata edge '{}' is not present in Mermaid", key),
            });
        }
    }
    errors
}

pub fn format_validation_errors(errors: &[ValidationError]) -> String {
    errors
        .iter()
        .map(|error| error.message.as_str())
        .collect::<Vec<_>>()
        .join("; ")
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
    scan_pipelines_dir(&built_in, "built-in", &mut pipelines);

    let user = paths::user_pipelines_dir();
    scan_pipelines_dir(&user, "custom", &mut pipelines);

    Ok(pipelines)
}

#[tauri::command]
pub fn load_pipeline(path: String) -> Result<LoadResult, String> {
    let base_path = std::path::Path::new(&path);
    let parent = base_path.parent().ok_or("Invalid path")?;
    let stem = base_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("Invalid filename")?;

    let mermaid_path = parent.join(format!("{}.md", stem));
    let meta_path = parent.join(format!("{}.meta.json", stem));
    let layout_path = parent.join(format!("{}.layout.json", stem));

    let mermaid_content = std::fs::read_to_string(&mermaid_path)
        .map_err(|e| format!("Failed to read {}: {}", mermaid_path.display(), e))?;
    let meta_content = std::fs::read_to_string(&meta_path)
        .map_err(|e| format!("Failed to read {}: {}", meta_path.display(), e))?;

    let mermaid_graph = parser::mermaid::parse(&mermaid_content)
        .map_err(|e| format!("Failed to parse Mermaid: {}", e))?;
    let metadata =
        parser::meta::parse(&meta_content).map_err(|e| format!("Failed to parse meta: {}", e))?;
    let layout = if layout_path.is_file() {
        let parsed = parser::layout::parse(
            &std::fs::read_to_string(&layout_path)
                .map_err(|e| format!("Failed to read {}: {}", layout_path.display(), e))?,
        )?;
        if let Some(node_id) = parsed
            .nodes
            .keys()
            .find(|node_id| !metadata.nodes.contains_key(*node_id))
        {
            return Err(format!(
                "Invalid pipeline layout: node '{}' is not present in metadata",
                node_id
            ));
        }
        Some(parsed)
    } else {
        None
    };

    let graph = parser::build_graph(&mermaid_graph, &metadata);
    let mut validation = validate_pipeline_sources(&mermaid_graph, &metadata);
    validation.extend(validate_pipeline_graph(&graph));
    if !validation.is_empty() {
        return Err(format!(
            "Invalid pipeline: {}",
            format_validation_errors(&validation)
        ));
    }

    let layout_writable = mermaid_path
        .canonicalize()
        .ok()
        .zip(paths::user_pipelines_dir().canonicalize().ok())
        .is_some_and(|(pipeline, custom_root)| pipeline.starts_with(custom_root));

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

    let content = std::fs::read_to_string(&canonical_meta)
        .map_err(|error| format!("Failed to read {}: {}", canonical_meta.display(), error))?;
    let metadata = parser::meta::parse(&content)?;
    if let Some(node_id) = positions
        .keys()
        .find(|node_id| !metadata.nodes.contains_key(*node_id))
    {
        return Err(format!("Layout contains unknown node '{}'", node_id));
    }
    let serialized = parser::layout::to_json(&parser::layout::PipelineLayout { nodes: positions })
        .map_err(|error| format!("Failed to serialize pipeline layout: {}", error))?;
    std::fs::write(&layout_path, serialized)
        .map_err(|error| format!("Failed to write {}: {}", layout_path.display(), error))?;
    Ok(true)
}

#[tauri::command]
pub fn save_pipeline(graph: PipelineGraph, path: String) -> Result<bool, String> {
    let validation = validate_pipeline_graph(&graph);
    if !validation.is_empty() {
        return Err(format!(
            "Invalid pipeline: {}",
            format_validation_errors(&validation)
        ));
    }
    let base_path = std::path::Path::new(&path);
    let parent = base_path.parent().ok_or("Invalid path")?;
    let stem = base_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("Invalid filename")?;

    std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;

    let mut mermaid = String::from("```mermaid\nflowchart TD\n");
    for node in &graph.nodes {
        let label = if node.label.is_empty() {
            &node.id
        } else {
            &node.label
        };
        mermaid.push_str(&format!(
            "    {}[\"{}\"]\n",
            node.id,
            label.replace('"', "&quot;")
        ));
    }
    mermaid.push('\n');
    for edge in &graph.edges {
        match edge.edge_type {
            EdgeType::Next => mermaid.push_str(&format!("    {} --> {}\n", edge.from, edge.to)),
            EdgeType::Error => {
                mermaid.push_str(&format!("    {} -->|error| {}\n", edge.from, edge.to))
            }
        }
    }
    mermaid.push_str("```\n");
    let mermaid_path = parent.join(format!("{}.md", stem));
    std::fs::write(&mermaid_path, &mermaid)
        .map_err(|e| format!("Failed to write {}: {}", mermaid_path.display(), e))?;

    let mut meta_nodes = std::collections::HashMap::new();
    for node in &graph.nodes {
        meta_nodes.insert(node.id.clone(), node.config.clone());
    }

    let meta_edges = graph
        .edges
        .iter()
        .map(|edge| {
            (
                format!("{}->{}", edge.from, edge.to),
                parser::meta::EdgeConfig {
                    edge_type: edge.edge_type.clone(),
                },
            )
        })
        .collect();

    let metadata = parser::meta::MetadataFile {
        schema: None,
        target: graph.target.clone(),
        nodes: meta_nodes,
        edges: meta_edges,
    };

    let meta_content =
        parser::meta::to_json(&metadata).map_err(|e| format!("Failed to serialize meta: {}", e))?;
    let meta_path = parent.join(format!("{}.meta.json", stem));
    std::fs::write(&meta_path, &meta_content)
        .map_err(|e| format!("Failed to write {}: {}", meta_path.display(), e))?;

    Ok(true)
}

#[tauri::command]
pub fn validate_pipeline(graph: PipelineGraph) -> Result<Vec<ValidationError>, String> {
    Ok(validate_pipeline_graph(&graph))
}

pub fn validate_pipeline_graph(graph: &PipelineGraph) -> Vec<ValidationError> {
    let mut errors = Vec::new();

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
        for edge_type in [EdgeType::Next, EdgeType::Error] {
            let count = graph
                .edges
                .iter()
                .filter(|edge| edge.from == node.id && edge.edge_type == edge_type)
                .count();
            if count > 1 {
                errors.push(ValidationError {
                    error_type: "ambiguous_transition".to_string(),
                    message: format!(
                        "Node '{}' has {} {:?} edges; only one is supported",
                        node.id, count, edge_type
                    ),
                });
            }
        }
    }

    let incoming: std::collections::HashSet<&str> =
        graph.edges.iter().map(|edge| edge.to.as_str()).collect();
    let entries: Vec<&str> = graph
        .nodes
        .iter()
        .map(|node| node.id.as_str())
        .filter(|id| !incoming.contains(id))
        .collect();
    if !graph.nodes.is_empty() && entries.len() != 1 {
        errors.push(ValidationError {
            error_type: "invalid_entry_count".to_string(),
            message: format!(
                "Pipeline must have exactly one entry node; found {}",
                entries.len()
            ),
        });
    }

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
        return;
    }
    let Ok(read_dir) = std::fs::read_dir(dir) else {
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
    fn rejects_missing_and_orphan_node_metadata() {
        let (flow, metadata, _) = parse_graph(
            "flowchart TD\nA[\"A\"]",
            r#"{"nodes":{"B":{"recognition":{"type":"DirectHit"}}}}"#,
        );
        let errors = validate_pipeline_sources(&flow, &metadata);
        assert!(errors
            .iter()
            .any(|error| error.error_type == "missing_node_metadata"));
        assert!(errors
            .iter()
            .any(|error| error.error_type == "orphan_node_metadata"));
    }

    #[test]
    fn rejects_multiple_entries_and_unreachable_nodes() {
        let (_, _, graph) = parse_graph(
            "flowchart TD\nA[\"A\"]\nB[\"B\"]",
            r#"{"nodes":{"A":{"recognition":{"type":"DirectHit"}},"B":{"recognition":{"type":"DirectHit"}}}}"#,
        );
        let errors = validate_pipeline_graph(&graph);
        assert!(errors
            .iter()
            .any(|error| error.error_type == "invalid_entry_count"));
    }

    #[test]
    fn rejects_ambiguous_transitions_and_zero_repeat() {
        let (_, _, graph) = parse_graph(
            "flowchart TD\nA[\"A\"]\nB[\"B\"]\nC[\"C\"]\nA --> B\nA --> C",
            r#"{"nodes":{"A":{"recognition":{"type":"DirectHit"},"repeat":0},"B":{"recognition":{"type":"DirectHit"}},"C":{"recognition":{"type":"DirectHit"}}}}"#,
        );
        let errors = validate_pipeline_graph(&graph);
        assert!(errors
            .iter()
            .any(|error| error.error_type == "ambiguous_transition"));
        assert!(errors
            .iter()
            .any(|error| error.error_type == "invalid_repeat"));
    }

    #[test]
    fn validates_window_size_only_on_the_top_level_target() {
        let (_, _, graph) = parse_graph(
            "flowchart TD\nA[\"A\"]",
            r#"{"target":{"type":"window","title":"Target","window_size":[0,720]},"nodes":{"A":{"target":{"type":"window","capture_area":"client","window_size":[1280,720]},"recognition":{"type":"DirectHit"}}}}"#,
        );
        let errors = validate_pipeline_graph(&graph);
        assert!(errors
            .iter()
            .any(|error| error.error_type == "invalid_window_size"));
        assert!(errors
            .iter()
            .any(|error| error.error_type == "node_window_size_not_supported"));
    }
}
