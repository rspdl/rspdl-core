use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("CLI crate should be nested below the workspace root")
        .to_owned()
}

#[test]
fn checks_the_expense_approval_example_with_runtime_fixture_data() {
    let workspace = workspace_root();
    let source = workspace.join("examples/expense-approval.rspdl");
    let data =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/expense-approval-data.json");

    let output = Command::new(env!("CARGO_BIN_EXE_rspdl"))
        .args([
            "check",
            source.to_str().expect("example path should be valid UTF-8"),
            "--data",
            data.to_str().expect("fixture path should be valid UTF-8"),
            "--json",
        ])
        .output()
        .expect("rspdl command should run");

    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(report["runtime_diagnostics"].as_array().unwrap().is_empty());
    assert!(
        !report["constraint_violations"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}
