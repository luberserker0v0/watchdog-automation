use serde_json::Value;

#[test]
fn detached_preview_capability_matches_the_frontend_window_label() {
    let capability: Value = serde_json::from_str(include_str!("../capabilities/preview.json"))
        .expect("preview capability must be valid JSON");
    let windows = capability["windows"]
        .as_array()
        .expect("preview capability must declare window labels");

    assert!(
        windows.iter().any(|label| label == "capture-preview"),
        "the detached Preview uses the stable `capture-preview` label"
    );
}
