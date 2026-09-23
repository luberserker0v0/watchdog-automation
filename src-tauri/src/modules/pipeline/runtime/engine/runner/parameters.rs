use crate::modules::pipeline::authoring::models::action::Action;
use std::collections::HashMap;

pub(crate) fn substitute_actions(
    actions: &[Action],
    params: &HashMap<String, serde_json::Value>,
) -> Vec<Action> {
    if params.is_empty() {
        return actions.to_vec();
    }
    actions
        .iter()
        .map(|action| substitute_action(action, params))
        .collect()
}

fn substitute_action(action: &Action, params: &HashMap<String, serde_json::Value>) -> Action {
    match action {
        Action::InputText { input_text } => Action::InputText {
            input_text: substitute_string(input_text, params),
        },
        Action::ClickKey { key } => Action::ClickKey {
            key: key.iter().map(|v| substitute_json(v, params)).collect(),
        },
        Action::Custom { action, param } => {
            let new_param = substitute_json(param, params);
            Action::Custom {
                action: action.clone(),
                param: new_param,
            }
        }
        _ => action.clone(),
    }
}

fn substitute_string(s: &str, params: &HashMap<String, serde_json::Value>) -> String {
    let mut result = s.to_string();
    for (key, value) in params {
        let placeholder = format!("{{{{{}}}}}", key);
        let replacement = match value {
            serde_json::Value::String(v) => v.clone(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Null => "null".to_string(),
            _ => value.to_string(),
        };
        result = result.replace(&placeholder, &replacement);
    }
    result
}

fn substitute_json(
    value: &serde_json::Value,
    params: &HashMap<String, serde_json::Value>,
) -> serde_json::Value {
    match value {
        serde_json::Value::String(s) => serde_json::Value::String(substitute_string(s, params)),
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(|v| substitute_json(v, params)).collect())
        }
        serde_json::Value::Object(map) => serde_json::Value::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), substitute_json(v, params)))
                .collect(),
        ),
        _ => value.clone(),
    }
}
