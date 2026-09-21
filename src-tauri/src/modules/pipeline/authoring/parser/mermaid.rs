#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EdgeType {
    Next,
    Error,
}

#[derive(Debug, Clone)]
pub struct MermaidNode {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone)]
pub struct MermaidEdge {
    pub from: String,
    pub to: String,
    pub edge_type: EdgeType,
}

#[derive(Debug, Clone)]
pub struct MermaidGraph {
    pub nodes: Vec<MermaidNode>,
    pub edges: Vec<MermaidEdge>,
}

fn ensure_node(nodes: &mut Vec<MermaidNode>, id: &str) {
    if !nodes.iter().any(|node| node.id == id) {
        nodes.push(MermaidNode {
            id: id.to_string(),
            label: id.to_string(),
        });
    }
}

fn valid_id(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some(first) if first.is_ascii_alphabetic() || first == '_')
        && chars.all(|character| {
            character.is_ascii_alphanumeric() || character == '_' || character == '-'
        })
}

fn parse_edge(line: &str) -> Option<(String, String, EdgeType)> {
    let (from, remainder, dashed_error) = if let Some((from, to)) = line.split_once("-.->") {
        (from, to, true)
    } else {
        let (from, to) = line.split_once("-->")?;
        (from, to, false)
    };
    let from = from.trim();
    let remainder = remainder.trim();
    let (edge_type, to) = if dashed_error {
        (EdgeType::Error, remainder)
    } else if let Some(to) = remainder.strip_prefix("|error|") {
        (EdgeType::Error, to.trim())
    } else {
        (EdgeType::Next, remainder)
    };
    if valid_id(from) && valid_id(to) {
        Some((from.to_string(), to.to_string(), edge_type))
    } else {
        None
    }
}

fn parse_node(line: &str) -> Option<MermaidNode> {
    let open = line.find('[')?;
    let close = line.rfind(']')?;
    if close <= open {
        return None;
    }
    let id = line[..open].trim();
    if !valid_id(id) {
        return None;
    }
    let label = line[open + 1..close]
        .trim()
        .trim_matches(|character| character == '"' || character == '\'')
        .trim();
    Some(MermaidNode {
        id: id.to_string(),
        label: if label.is_empty() { id } else { label }.to_string(),
    })
}

pub fn parse(content: &str) -> Result<MermaidGraph, String> {
    let mut found_header = false;
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty()
            || line.starts_with("```")
            || line.starts_with("%%")
            || line.starts_with('#')
        {
            continue;
        }
        if line.starts_with("flowchart ") || line.starts_with("graph ") {
            found_header = true;
            continue;
        }
        if !found_header {
            continue;
        }

        if let Some((from, to, edge_type)) = parse_edge(line) {
            ensure_node(&mut nodes, &from);
            ensure_node(&mut nodes, &to);
            edges.push(MermaidEdge {
                from,
                to,
                edge_type,
            });
            continue;
        }
        if let Some(node) = parse_node(line) {
            if let Some(existing) = nodes.iter_mut().find(|existing| existing.id == node.id) {
                existing.label = node.label;
            } else {
                nodes.push(node);
            }
        }
    }

    if !found_header {
        return Err("Missing Mermaid flowchart header".to_string());
    }

    Ok(MermaidGraph { nodes, edges })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_markdown_fence_nodes_and_edges() {
        let graph =
            parse("```mermaid\nflowchart TD\nA[\"Start\"]\nA --> B\nA -->|error| Error\n```")
                .unwrap();
        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.nodes[0].label, "Start");
        assert_eq!(graph.edges.len(), 2);
        assert_eq!(graph.edges[1].edge_type, EdgeType::Error);
    }

    #[test]
    fn requires_a_flowchart_header() {
        assert!(parse("A --> B").is_err());
    }
}
