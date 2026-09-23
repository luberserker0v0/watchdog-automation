use crate::modules::pipeline::authoring::models::edge::EdgeType;
use crate::modules::pipeline::authoring::models::graph::{PipelineGraph, PipelineTarget};
use crate::modules::pipeline::authoring::parser;

use super::ValidationError;
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
