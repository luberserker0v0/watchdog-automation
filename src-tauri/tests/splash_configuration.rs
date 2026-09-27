use serde_json::Value;

fn config_windows(config_source: &str) -> Vec<Value> {
    serde_json::from_str::<Value>(config_source).expect("Tauri config must be valid JSON")["app"]
        ["windows"]
        .as_array()
        .expect("Tauri config must declare windows")
        .clone()
}

fn capability(source: &str) -> Value {
    serde_json::from_str(source).expect("capability must be valid JSON")
}

#[test]
fn production_and_tester_start_with_a_splash_and_hidden_main() {
    for source in [
        include_str!("../tauri.conf.json"),
        include_str!("../tauri.conf.tester.json"),
    ] {
        let windows = config_windows(source);
        let main = windows
            .iter()
            .find(|window| window["label"] == "main")
            .expect("main window must be explicit");
        let splash = windows
            .iter()
            .find(|window| window["label"] == "splash")
            .expect("splash window must be configured");
        assert_eq!(main["visible"], false, "main must start hidden");
        assert_eq!(splash["url"], "/splash.html");
    }
}

#[test]
fn splash_has_store_read_access_without_filesystem_or_log_access() {
    let splash = capability(include_str!("../capabilities/splash.json"));
    assert_eq!(splash["windows"], serde_json::json!(["splash"]));
    let permissions = splash["permissions"].as_array().unwrap();
    assert!(permissions.iter().any(|item| item == "store:allow-load"));
    assert!(permissions.iter().any(|item| item == "store:allow-get"));
    assert!(!permissions.iter().any(|item| {
        item.as_str().is_some_and(|permission| {
            permission == "store:allow-set"
                || permission == "store:allow-save"
                || permission == "store:allow-delete"
        })
    }));
    assert!(!permissions.iter().any(|item| {
        item.as_str().is_some_and(|permission| {
            permission.starts_with("fs:") || permission.starts_with("log:")
        })
    }));
}

#[test]
fn main_windows_limit_fs_writes_to_recorder_downloads() {
    for source in [
        include_str!("../capabilities/default.json"),
        include_str!("../capabilities/tester.json"),
    ] {
        let main = capability(source);
        assert_eq!(main["windows"], serde_json::json!(["main"]));
        assert!(main["permissions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item == "store:default"));
        assert!(main["permissions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item == "log:default"));
        let permission = main["permissions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["identifier"] == "fs:allow-write-text-file")
            .expect("main must grant scoped recorder export");
        assert_eq!(
            permission["allow"],
            serde_json::json!([{ "path": "$DOWNLOAD/watchdog-action-recording-*.json" }])
        );
    }
}

#[test]
fn writable_paths_have_no_environment_or_working_directory_fallback() {
    let source = include_str!("../src/core/paths.rs");
    for resolver in [
        ".app_data_dir()",
        ".app_local_data_dir()",
        ".app_cache_dir()",
        ".app_log_dir()",
        ".download_dir()",
        ".resource_dir()",
    ] {
        assert!(source.contains(resolver), "missing {resolver}");
    }
    assert!(!source.contains("current_dir("));
    assert!(!source.contains("var_os("));
}

#[test]
fn sample_target_uses_no_plugin_capability_window_label() {
    let source = include_str!("../src/lib.rs");
    assert!(source.contains("_ => \"target\""));
    for capability_source in [
        include_str!("../capabilities/default.json"),
        include_str!("../capabilities/tester.json"),
        include_str!("../capabilities/splash.json"),
    ] {
        let value = capability(capability_source);
        assert!(!value["windows"]
            .as_array()
            .unwrap()
            .iter()
            .any(|window| window == "target"));
    }
}

#[test]
fn updater_access_is_exclusive_to_the_production_main_window() {
    let production = capability(include_str!("../capabilities/default.json"));
    let tester = capability(include_str!("../capabilities/tester.json"));
    let splash = capability(include_str!("../capabilities/splash.json"));
    let preview = capability(include_str!("../capabilities/preview.json"));
    let production_permissions = production["permissions"].as_array().unwrap();
    assert!(production_permissions
        .iter()
        .any(|item| item == "updater:default"));
    assert!(production_permissions
        .iter()
        .any(|item| item == "process:allow-restart"));
    for restricted in [tester, splash, preview] {
        assert!(!restricted["permissions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| {
                item.as_str().is_some_and(|permission| {
                    permission.starts_with("updater:") || permission.starts_with("process:")
                })
            }));
    }

    let production_config: Value =
        serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
    let tester_config: Value =
        serde_json::from_str(include_str!("../tauri.conf.tester.json")).unwrap();
    assert!(production_config["app"]["security"]["capabilities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item == "default"));
    assert!(!tester_config["app"]["security"]["capabilities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item == "default"));
}
