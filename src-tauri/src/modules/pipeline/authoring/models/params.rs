use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamDef {
    pub r#type: String,
    pub key: String,
    pub label: String,
    #[serde(default)]
    pub placeholder: String,
    #[serde(default)]
    pub default: serde_json::Value,
    #[serde(default)]
    pub options: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_param_def_deserialize() {
        let json = r#"{
            "type": "text",
            "key": "account",
            "label": "Account",
            "placeholder": "Enter account",
            "default": "admin"
        }"#;
        let param: ParamDef = serde_json::from_str(json).unwrap();
        assert_eq!(param.r#type, "text");
        assert_eq!(param.key, "account");
        assert_eq!(param.label, "Account");
        assert_eq!(param.placeholder, "Enter account");
        assert_eq!(
            param.default,
            serde_json::Value::String("admin".to_string())
        );
        assert!(param.options.is_empty());
    }

    #[test]
    fn test_param_def_dropdown() {
        let json = r#"{
            "type": "dropdown",
            "key": "mode",
            "label": "Mode",
            "options": ["fast", "slow"],
            "default": "fast"
        }"#;
        let param: ParamDef = serde_json::from_str(json).unwrap();
        assert_eq!(param.r#type, "dropdown");
        assert_eq!(param.options, vec!["fast", "slow"]);
    }

    #[test]
    fn test_param_def_switch() {
        let json = r#"{
            "type": "switch",
            "key": "verbose",
            "label": "Verbose",
            "default": true
        }"#;
        let param: ParamDef = serde_json::from_str(json).unwrap();
        assert_eq!(param.r#type, "switch");
        assert_eq!(param.default, serde_json::Value::Bool(true));
    }

    #[test]
    fn test_param_def_defaults() {
        let json = r#"{
            "type": "number",
            "key": "count",
            "label": "Count"
        }"#;
        let param: ParamDef = serde_json::from_str(json).unwrap();
        assert_eq!(param.placeholder, "");
        assert_eq!(param.default, serde_json::Value::Null);
        assert!(param.options.is_empty());
    }
}
