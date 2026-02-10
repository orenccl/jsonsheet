#[test]
#[ignore]
fn ui_e2e_smoke() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script = manifest_dir.join("tests").join("ui_e2e").join("run.mjs");

    let status = std::process::Command::new("node")
        .arg(script)
        .env("JSONSHEET_ROOT", manifest_dir)
        .status()
        .expect("failed to launch node for UI E2E");

    assert!(status.success(), "UI E2E script failed");
}
