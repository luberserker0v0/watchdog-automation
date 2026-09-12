use crate::modules::pipeline::models::edge::EdgeType;
use crate::modules::pipeline::models::graph::PipelineGraph;
use crate::modules::pipeline::parser;
use crate::utils::paths;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LoadResult {
    pub graph: PipelineGraph,
    pub mermaid: String,
    pub meta: String,
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

    let mermaid_content = std::fs::read_to_string(&mermaid_path)
        .map_err(|e| format!("Failed to read {}: {}", mermaid_path.display(), e))?;
    let meta_content = std::fs::read_to_string(&meta_path)
        .map_err(|e| format!("Failed to read {}: {}", meta_path.display(), e))?;

    let mermaid_graph = parser::mermaid::parse(&mermaid_content)
        .map_err(|e| format!("Failed to parse Mermaid: {}", e))?;
    let metadata =
        parser::meta::parse(&meta_content).map_err(|e| format!("Failed to parse meta: {}", e))?;

    let graph = parser::build_graph(&mermaid_graph, &metadata);

    Ok(LoadResult {
        graph,
        mermaid: mermaid_content,
        meta: meta_content,
    })
}

#[tauri::command]
pub fn save_pipeline(graph: PipelineGraph, path: String) -> Result<bool, String> {
    let base_path = std::path::Path::new(&path);
    let parent = base_path.parent().ok_or("Invalid path")?;
    let stem = base_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("Invalid filename")?;

    std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;

    let mut mermaid = String::from("```mermaid\nflowchart TD\n");
    for node in &graph.nodes {
        mermaid.push_str(&format!("    {}[\"{}\"]\n", node.id, node.id));
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
    let mut errors = Vec::new();

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

    if has_cycle(&graph) {
        errors.push(ValidationError {
            error_type: "cycle_detected".to_string(),
            message: "Pipeline contains a cycle".to_string(),
        });
    }

    Ok(errors)
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
