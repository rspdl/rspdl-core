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
fn compiles_every_rspdl_example() {
    let examples = workspace_root().join("examples");
    let mut sources = std::fs::read_dir(&examples)
        .expect("examples directory should be readable")
        .map(|entry| entry.expect("example entry should be readable").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "rspdl")
        })
        .collect::<Vec<_>>();
    sources.sort();
    assert!(!sources.is_empty(), "at least one example should exist");

    for source in sources {
        let output = Command::new(env!("CARGO_BIN_EXE_rspdl"))
            .args([
                "compile",
                source.to_str().expect("example path should be valid UTF-8"),
                "--json",
            ])
            .output()
            .expect("rspdl command should run");

        assert_eq!(
            output.status.code(),
            Some(0),
            "example {} failed: {}",
            source.display(),
            String::from_utf8_lossy(&output.stderr)
        );
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert!(report["module"].is_object(), "example {}", source.display());
    }
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

#[test]
fn finds_a_virtual_model_without_runtime_data() {
    let source = workspace_root().join("examples/project-ownership.rspdl");
    let output = Command::new(env!("CARGO_BIN_EXE_rspdl"))
        .args([
            "model",
            source.to_str().expect("example path should be valid UTF-8"),
            "--scope",
            "2",
            "--json",
        ])
        .output()
        .expect("rspdl command should run");

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["result"]["status"], "sat");
    assert!(
        !report["result"]["witness"]["relation_tuples"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}
