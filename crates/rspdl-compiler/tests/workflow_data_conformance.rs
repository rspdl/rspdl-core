use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use rspdl_compiler::compile_ko;
use serde::Deserialize;

#[derive(Deserialize)]
struct Case {
    spec_version: String,
    locale: String,
    category: String,
    expected_module: bool,
    expected_diagnostics: Vec<ExpectedDiagnostic>,
}

#[derive(Deserialize)]
struct ExpectedDiagnostic {
    rule_id: String,
    severity: String,
    message_key: String,
}

#[test]
fn workflow_data_conformance_suite() {
    let root = repository_root().join("conformance/ko-KR/workflow-data");
    let mut dirs = fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    dirs.sort();
    let mut categories = BTreeSet::new();
    for dir in dirs {
        let name = dir.file_name().unwrap().to_string_lossy();
        let case: Case =
            serde_json::from_str(&fs::read_to_string(dir.join("case.json")).unwrap()).unwrap();
        assert_eq!(case.spec_version, "0.6.0", "{name}");
        assert_eq!(case.locale, "ko-KR", "{name}");
        categories.insert(case.category.clone());
        let source = fs::read_to_string(dir.join("input.rspdl")).unwrap();
        let compiled = compile_ko(&source);
        assert_eq!(compiled, compile_ko(&source), "{name}: deterministic");
        assert_eq!(
            compiled.module.is_some(),
            case.expected_module,
            "{name}: module"
        );
        let actual = compiled
            .diagnostics
            .iter()
            .map(|diagnostic| {
                (
                    diagnostic.rule_id.as_str(),
                    serde_json::to_value(diagnostic.severity)
                        .unwrap()
                        .as_str()
                        .unwrap()
                        .to_owned(),
                    diagnostic.message_key.as_str(),
                )
            })
            .collect::<Vec<_>>();
        let expected = case
            .expected_diagnostics
            .iter()
            .map(|diagnostic| {
                (
                    diagnostic.rule_id.as_str(),
                    diagnostic.severity.clone(),
                    diagnostic.message_key.as_str(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(actual, expected, "{name}: diagnostics");

        if case.expected_module {
            let document = rspdl_ko::parse(&source).document.expect("valid source");
            let formatted = rspdl_ko::format_document(&document).expect("formats");
            assert_eq!(
                rspdl_ko::format_document(&rspdl_ko::parse(&formatted).document.unwrap()).unwrap(),
                formatted,
                "{name}: idempotent"
            );
            assert_eq!(
                strip_spans(&compile_ko(&formatted).module),
                strip_spans(&compiled.module),
                "{name}: semantic roundtrip"
            );
        }
    }
    assert_eq!(
        categories,
        ["boundary", "failure", "false_positive", "normal"]
            .into_iter()
            .map(str::to_owned)
            .collect()
    );
}

fn strip_spans<T: serde::Serialize>(value: &T) -> serde_json::Value {
    fn strip(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(object) => {
                object.remove("span");
                object.values_mut().for_each(strip);
            }
            serde_json::Value::Array(values) => values.iter_mut().for_each(strip),
            _ => {}
        }
    }
    let mut value = serde_json::to_value(value).unwrap();
    strip(&mut value);
    value
}

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .to_path_buf()
}
