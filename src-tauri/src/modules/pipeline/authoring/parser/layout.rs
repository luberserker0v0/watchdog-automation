use std::collections::HashMap;
use std::sync::OnceLock;

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
    static VALIDATOR: OnceLock<Result<jsonschema::Validator, String>> = OnceLock::new();
    let validator = VALIDATOR
        .get_or_init(|| {
            let schema: serde_json::Value = serde_json::from_str(include_str!(
                "../../../../../../schemas/watchdog-layout.schema.json"
            ))
            .map_err(|error| error.to_string())?;
            jsonschema::draft202012::new(&schema).map_err(|error| error.to_string())
        })
        .as_ref()
        .map_err(|error| format!("Invalid bundled layout schema: {error}"))?;
    let value: serde_json::Value = serde_json::from_str(content)
        .map_err(|error| format!("Invalid pipeline layout JSON: {error}"))?;
    if let Some(error) = validator.iter_errors(&value).next() {
        return Err(format!(
            "Invalid pipeline layout at {}: {}",
            error.instance_path(),
            error
        ));
    }
    serde_json::from_value(value).map_err(|error| format!("Invalid pipeline layout JSON: {error}"))
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

    #[test]
    fn rejects_layout_missing_coordinate() {
        let error = parse(r#"{"nodes":{"A":{"x":5}}}"#).unwrap_err();
        assert!(error.contains("A"), "{error}");
    }
}
