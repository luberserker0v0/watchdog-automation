use super::*;
use serde_json;

#[test]
fn test_do_nothing_serde() {
    let action = Action::DoNothing;
    let json = serde_json::to_string(&action).unwrap();
    assert!(json.contains("\"type\":\"DoNothing\""));

    let deserialized: Action = serde_json::from_str(&json).unwrap();
    assert!(matches!(deserialized, Action::DoNothing));
}

#[test]
fn test_click_serde() {
    let action = Action::Click {
        target: Target::recognized(),
        target_offset: None,
    };

    let json = serde_json::to_string(&action).unwrap();
    assert!(json.contains("\"type\":\"Click\""));

    let deserialized: Action = serde_json::from_str(&json).unwrap();
    match deserialized {
        Action::Click {
            target,
            target_offset,
        } => {
            assert!(matches!(target, Target::RecognizedBool(true)));
            assert!(target_offset.is_none());
        }
        _ => panic!("Expected Click action"),
    }
}

#[test]
fn test_click_with_coordinate() {
    let action = Action::Click {
        target: Target::Coordinate([100, 200]),
        target_offset: None,
    };

    let json = serde_json::to_string(&action).unwrap();
    assert!(json.contains("\"type\":\"Click\""));

    let deserialized: Action = serde_json::from_str(&json).unwrap();
    match deserialized {
        Action::Click { target, .. } => match target {
            Target::Coordinate([x, y]) => {
                assert_eq!(x, 100);
                assert_eq!(y, 200);
            }
            _ => panic!("Expected Coordinate target"),
        },
        _ => panic!("Expected Click action"),
    }
}

#[test]
fn test_click_with_node_reference() {
    let action = Action::Click {
        target: Target::Node("LoginButton".to_string()),
        target_offset: None,
    };

    let json = serde_json::to_string(&action).unwrap();
    let deserialized: Action = serde_json::from_str(&json).unwrap();
    match deserialized {
        Action::Click { target, .. } => match target {
            Target::Node(name) => assert_eq!(name, "LoginButton"),
            _ => panic!("Expected Node target"),
        },
        _ => panic!("Expected Click action"),
    }
}

#[test]
fn test_input_text_serde() {
    let action = Action::InputText {
        input_text: "Hello World".to_string(),
    };

    let json = serde_json::to_string(&action).unwrap();
    assert!(json.contains("\"type\":\"InputText\""));
    assert!(json.contains("\"input_text\":\"Hello World\""));

    let deserialized: Action = serde_json::from_str(&json).unwrap();
    match deserialized {
        Action::InputText { input_text } => {
            assert_eq!(input_text, "Hello World");
        }
        _ => panic!("Expected InputText action"),
    }
}

#[test]
fn test_click_key_serde() {
    let action = Action::ClickKey {
        key: vec![serde_json::json!(0x0D), serde_json::json!(0x09)],
    };

    let json = serde_json::to_string(&action).unwrap();
    assert!(json.contains("\"type\":\"ClickKey\""));

    let deserialized: Action = serde_json::from_str(&json).unwrap();
    match deserialized {
        Action::ClickKey { key } => {
            assert_eq!(key, vec![serde_json::json!(0x0D), serde_json::json!(0x09)]);
        }
        _ => panic!("Expected ClickKey action"),
    }
}

#[test]
fn test_click_key_from_string_numeric() {
    let json = r#"{"type":"ClickKey","key":["27"]}"#;
    let deserialized: Action = serde_json::from_str(json).unwrap();
    match deserialized {
        Action::ClickKey { key } => {
            assert_eq!(key, vec![serde_json::json!("27")]);
        }
        _ => panic!("Expected ClickKey action"),
    }
}

#[test]
fn test_click_key_from_template_placeholder() {
    let json = r#"{"type":"ClickKey","key":["{{key}}"]}"#;
    let deserialized: Action = serde_json::from_str(json).unwrap();
    match deserialized {
        Action::ClickKey { key } => {
            assert_eq!(key, vec![serde_json::json!("{{key}}")]);
        }
        _ => panic!("Expected ClickKey action"),
    }
}

#[test]
fn test_stop_task_serde() {
    let action = Action::StopTask;
    let json = serde_json::to_string(&action).unwrap();
    assert!(json.contains("\"type\":\"StopTask\""));

    let deserialized: Action = serde_json::from_str(&json).unwrap();
    assert!(matches!(deserialized, Action::StopTask));
}
