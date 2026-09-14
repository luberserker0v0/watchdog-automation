use std::collections::HashMap;

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PipelineLayout {
    #[serde(default)]
    pub nodes: HashMap<String, NodePosition>,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodePosition {
    pub x: i32,
    pub y: i32,
}

pub fn parse(content: &str) -> Result<PipelineLayout, String> {
    serde_json::from_str(content)
        .map_err(|error| format!("Invalid pipeline layout JSON: {}", error))
}

pub fn to_json(layout: &PipelineLayout) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(layout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_shareable_node_positions() {
        let layout = parse(r#"{"nodes":{"Login":{"x":120,"y":80}}}"#).unwrap();
        assert_eq!(layout.nodes["Login"], NodePosition { x: 120, y: 80 });
        assert!(to_json(&layout).unwrap().contains("\"Login\""));
    }
}
