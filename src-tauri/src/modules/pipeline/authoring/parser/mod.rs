pub mod layout;
pub mod mermaid;
pub mod meta;

use crate::modules::pipeline::authoring::models::edge::{Edge, EdgeType};
use crate::modules::pipeline::authoring::models::graph::PipelineGraph;
use crate::modules::pipeline::authoring::models::node::{Node, NodeConfig};

/// Merge instance config with node template.
/// Instance overrides take precedence over template defaults.
fn merge_instance_config(mut template: NodeConfig, instance: &meta::InstanceConfig) -> NodeConfig {
    // Override optional fields
    if let Some(v) = instance.kind {
        template.kind = v;
    }
    if let Some(v) = instance.desc.clone() {
        template.desc = v;
    }
    if let Some(v) = instance.timeout {
        template.timeout = v;
    }
    if let Some(v) = instance.error_timeout {
        template.error_timeout = v;
    }
    if let Some(v) = instance.rate_limit {
        template.rate_limit = v;
    }
    if let Some(v) = instance.pre_delay {
        template.pre_delay = v;
    }
    if let Some(v) = instance.pre_wait_freezes.clone() {
        template.pre_wait_freezes = v;
    }
    if let Some(v) = instance.post_delay {
        template.post_delay = v;
    }
    if let Some(v) = instance.post_wait_freezes.clone() {
        template.post_wait_freezes = v;
    }
    if let Some(v) = instance.enabled {
        template.enabled = v;
    }
    if let Some(v) = instance.inverse {
        template.inverse = v;
    }
    if let Some(v) = instance.max_hit {
        template.max_hit = v;
    }
    if let Some(v) = instance.repeat {
        template.repeat = v;
    }
    if let Some(v) = instance.repeat_delay {
        template.repeat_delay = v;
    }
    template
}

pub fn build_graph(
    mermaid: &mermaid::MermaidGraph,
    metadata: &meta::MetadataFile,
) -> PipelineGraph {
    let nodes = mermaid
        .nodes
        .iter()
        .map(|flow_node| {
            // Check if node ID exists in instances
            let (config, params) = if let Some(instances) = &metadata.instances {
                if let Some(instance) = instances.get(&flow_node.id) {
                    // Resolve node template
                    let template = metadata
                        .nodes
                        .get(&instance.node)
                        .cloned()
                        .unwrap_or_default();
                    // Merge instance overrides with template
                    let config = merge_instance_config(template, instance);
                    let mut params = instance.params.clone();
                    for param in &mut params {
                        if let Some(value) = instance.param_values.get(&param.key) {
                            param.default = value.clone();
                        }
                    }
                    (config, params)
                } else {
                    // Fall back to label-based lookup
                    let config = metadata
                        .nodes
                        .get(&flow_node.label)
                        .or_else(|| metadata.nodes.get(&flow_node.id))
                        .cloned()
                        .unwrap_or_default();
                    (config, Vec::new())
                }
            } else {
                // No instances field, use label-based lookup
                let config = metadata
                    .nodes
                    .get(&flow_node.label)
                    .or_else(|| metadata.nodes.get(&flow_node.id))
                    .cloned()
                    .unwrap_or_default();
                (config, Vec::new())
            };
            Node {
                id: flow_node.id.clone(),
                label: flow_node.label.clone(),
                config,
                params,
            }
        })
        .collect();

    let edges = mermaid
        .edges
        .iter()
        .map(|edge| Edge {
            from: edge.from.clone(),
            to: edge.to.clone(),
            edge_type: match edge.edge_type {
                mermaid::EdgeType::Next => EdgeType::Next,
                mermaid::EdgeType::Error => EdgeType::Error,
            },
        })
        .collect();

    PipelineGraph {
        version: "1.0".to_string(),
        target: metadata.target.clone(),
        launch: metadata.launch.clone(),
        nodes,
        edges,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instance_values_become_visible_parameter_defaults() {
        let mermaid = mermaid::parse("flowchart TD\nA[\"A\"]").unwrap();
        let metadata = meta::parse(r#"{"nodes":{"T":{"kind":"Entry"}},"instances":{"A":{"node":"T","params":[{"type":"text","key":"name","label":"Name","default":"Default"}],"param_values":{"name":"Alice"}}}}"#).unwrap();
        let graph = build_graph(&mermaid, &metadata);
        assert_eq!(graph.nodes[0].params[0].default, serde_json::json!("Alice"));
    }

    #[test]
    fn metadata_top_level_target_becomes_the_graph_target() {
        let mermaid = mermaid::parse("flowchart TD\nA").unwrap();
        let metadata = meta::parse(
            r#"{"target":{"type":"screen","index":2},"nodes":{"A":{"recognition":{"type":"DirectHit"}}}}"#,
        )
        .unwrap();
        let graph = build_graph(&mermaid, &metadata);
        assert!(matches!(
            graph.target,
            Some(
                crate::modules::pipeline::authoring::models::graph::PipelineTarget::Screen {
                    index: 2
                }
            )
        ));
    }

    #[test]
    fn shipped_sample_pipeline_is_valid_and_has_entry() {
        let mermaid_content = std::fs::read_to_string("resources/pipelines/sample/sample.md")
            .expect("shipped sample.md must exist");
        let meta_content = std::fs::read_to_string("resources/pipelines/sample/sample.meta.json")
            .expect("shipped sample.meta.json must exist");
        let mermaid_graph = mermaid::parse(&mermaid_content).unwrap();
        let metadata = meta::parse(&meta_content).unwrap();
        let graph = build_graph(&mermaid_graph, &metadata);
        let validation =
            crate::modules::pipeline::authoring::editor::validate_pipeline_graph(&graph);
        assert!(
            validation.is_empty(),
            "shipped sample must validate: {validation:?}"
        );
        let plan = crate::modules::pipeline::runtime::plan::ExecutionPlan::compile(graph).unwrap();
        assert_eq!(plan.entry_node_id(), "Start");
    }

    #[test]
    fn shipped_patrol_redirection_pipeline_is_valid() {
        let mermaid_content =
            std::fs::read_to_string("resources/pipelines/patrol-redirection/patrol-redirection.md")
                .expect("shipped patrol redirection pipeline must exist");
        let meta_content = std::fs::read_to_string(
            "resources/pipelines/patrol-redirection/patrol-redirection.meta.json",
        )
        .expect("shipped patrol redirection metadata must exist");
        let mermaid_graph = mermaid::parse(&mermaid_content).unwrap();
        let metadata = meta::parse(&meta_content).unwrap();
        let graph = build_graph(&mermaid_graph, &metadata);
        let validation =
            crate::modules::pipeline::authoring::editor::validate_pipeline_graph(&graph);
        assert!(
            validation.is_empty(),
            "shipped patrol redirection pipeline must validate: {validation:?}"
        );
        let plan = crate::modules::pipeline::runtime::plan::ExecutionPlan::compile(graph).unwrap();
        assert_eq!(plan.entry_node_id(), "Start");
    }
}
