pub mod mermaid;
pub mod meta;

use crate::modules::pipeline::models::edge::{Edge, EdgeType};
use crate::modules::pipeline::models::graph::PipelineGraph;
use crate::modules::pipeline::models::node::{Node, NodeConfig};
use crate::modules::pipeline::models::recognizer::{Recognition, RecognitionConfig};

pub fn build_graph(
    mermaid: &mermaid::MermaidGraph,
    metadata: &meta::MetadataFile,
) -> PipelineGraph {
    let nodes = mermaid
        .nodes
        .iter()
        .map(|flow_node| {
            let config = metadata
                .nodes
                .get(&flow_node.id)
                .cloned()
                .unwrap_or_else(|| NodeConfig {
                    recognition: RecognitionConfig::from(Recognition::TemplateMatch {
                        template: format!(
                            "{}.png",
                            flow_node.label.to_lowercase().replace(' ', "_")
                        ),
                        threshold: None,
                    }),
                    ..NodeConfig::default()
                });
            Node {
                id: flow_node.id.clone(),
                config,
            }
        })
        .collect();

    let edges = mermaid
        .edges
        .iter()
        .map(|edge| Edge {
            from: edge.from.clone(),
            to: edge.to.clone(),
            edge_type: metadata
                .edges
                .get(&format!("{}->{}", edge.from, edge.to))
                .map(|config| config.edge_type.clone())
                .unwrap_or_else(|| match edge.edge_type {
                    mermaid::EdgeType::Next => EdgeType::Next,
                    mermaid::EdgeType::Error => EdgeType::Error,
                }),
        })
        .collect();

    PipelineGraph {
        version: "1.0".to_string(),
        desktop: None,
        target: None,
        nodes,
        edges,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_overrides_mermaid_edge_behavior() {
        let mermaid = mermaid::parse("flowchart TD\nA --> B").unwrap();
        let metadata =
            meta::parse(r#"{"nodes":{},"edges":{"A->B":{"edge_type":"error"}}}"#).unwrap();
        let graph = build_graph(&mermaid, &metadata);
        assert!(matches!(graph.edges[0].edge_type, EdgeType::Error));
    }
}
