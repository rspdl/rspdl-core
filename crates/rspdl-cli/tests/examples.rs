use std::path::PathBuf;
use std::process::Command;

use rspdl_domain::MAX_BOUNDED_SCOPE_PER_MODEL;

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
fn rejects_invalid_lifecycle_examples_with_expected_diagnostics() {
    let workspace = workspace_root();
    let cases = [
        (
            "unproduced-data-usage.rspdl",
            &["RSPDL-DATA-001", "RSPDL-DATA-002"][..],
        ),
        ("unproduced-calculation.rspdl", &["RSPDL-DATA-001"][..]),
        ("conflicting-action-results.rspdl", &["RSPDL-DATA-004"][..]),
    ];

    for (file_name, expected_rule_ids) in cases {
        let source = workspace.join("examples/rejected").join(file_name);
        let output = Command::new(env!("CARGO_BIN_EXE_rspdl"))
            .args([
                "compile",
                source.to_str().expect("example path should be valid UTF-8"),
                "--json",
            ])
            .output()
            .expect("rspdl command should run");

        assert_eq!(output.status.code(), Some(1), "{file_name}: {output:?}");
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let actual_rule_ids = report["diagnostics"]
            .as_array()
            .expect("compile report should contain diagnostics")
            .iter()
            .filter_map(|diagnostic| diagnostic["rule_id"].as_str())
            .collect::<Vec<_>>();

        for expected_rule_id in expected_rule_ids {
            assert!(
                actual_rule_ids.contains(expected_rule_id),
                "{file_name} should report {expected_rule_id}, got {actual_rule_ids:?}"
            );
        }
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

#[test]
fn distinguishes_bound_specific_unsat_from_a_larger_sat_scope() {
    let source = workspace_root().join("examples/project-assignment-bound.rspdl");

    let scope_one = Command::new(env!("CARGO_BIN_EXE_rspdl"))
        .args([
            "model",
            source.to_str().expect("example path should be valid UTF-8"),
            "--scope",
            "1",
            "--json",
        ])
        .output()
        .expect("rspdl command should run");
    assert_eq!(scope_one.status.code(), Some(2), "{scope_one:?}");
    let scope_one_report: serde_json::Value =
        serde_json::from_slice(&scope_one.stdout).expect("scope-1 report should be JSON");
    assert_eq!(scope_one_report["result"]["status"], "unsat_within_bound");
    assert!(
        !scope_one_report["result"]["core_rule_ids"]
            .as_array()
            .expect("UNSAT report should contain core rule IDs")
            .is_empty()
    );

    let scope_two = Command::new(env!("CARGO_BIN_EXE_rspdl"))
        .args([
            "model",
            source.to_str().expect("example path should be valid UTF-8"),
            "--scope",
            "2",
            "--json",
        ])
        .output()
        .expect("rspdl command should run");
    assert_eq!(scope_two.status.code(), Some(0), "{scope_two:?}");
    let scope_two_report: serde_json::Value =
        serde_json::from_slice(&scope_two.stdout).expect("scope-2 report should be JSON");
    assert_eq!(scope_two_report["result"]["status"], "sat");
    assert!(
        scope_two_report["result"]["witness"]["relation_tuples"]
            .as_array()
            .expect("SAT report should contain relation tuples")
            .len()
            >= 2
    );
}

#[test]
fn out_of_range_model_scope_is_a_structured_error() {
    let source = workspace_root().join("examples/project-ownership.rspdl");
    let invalid_scope = (MAX_BOUNDED_SCOPE_PER_MODEL + 1).to_string();
    let output = Command::new(env!("CARGO_BIN_EXE_rspdl"))
        .arg("model")
        .arg(source)
        .arg("--scope")
        .arg(invalid_scope)
        .arg("--json")
        .output()
        .expect("rspdl command should run");

    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["failure"]["rule_id"], "RSPDL-MODEL-001");
    assert_eq!(
        report["failure"]["message_key"],
        "model_finding.configuration_error"
    );
    assert!(report.get("result").is_none());
}
