use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use rspdl_compiler::compile_ko;
use rspdl_domain::DerivationExpression;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Case {
    spec_version: String,
    locale: String,
    category: String,
    expected_module: bool,
    expected_diagnostics: Vec<ExpectedDiagnostic>,
    expected_derivation: Option<ExpectedDerivation>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct ExpectedDiagnostic {
    rule_id: String,
    severity: String,
    #[serde(default)]
    span: Option<ExpectedSpan>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct ExpectedSpan {
    start: usize,
    end: usize,
}

#[derive(Debug, Deserialize, PartialEq)]
struct ExpectedDerivation {
    target_field_id: String,
    source_field_id: String,
    recalculate_when_changed_field_ids: Vec<String>,
}

#[test]
fn sentence_shaped_data_usage_conformance_suite() {
    let root = repository_root().join("conformance/ko-KR/data-usage");
    let mut directories = fs::read_dir(&root)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", root.display()))
        .map(|entry| entry.expect("conformance entry should be readable").path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    directories.sort();

    let mut categories = BTreeSet::new();
    for directory in directories {
        let name = directory.file_name().unwrap().to_string_lossy();
        let case = read_case(&directory);
        assert_eq!(case.spec_version, "0.2.0", "case {name}");
        assert_eq!(case.locale, "ko-KR", "case {name}");
        categories.insert(case.category.clone());
        let input = fs::read_to_string(directory.join("input.rspdl")).unwrap();
        let compilation = compile_ko(&input);
        assert_eq!(compilation, compile_ko(&input), "case {name} determinism");
        assert_eq!(
            compilation.module.is_some(),
            case.expected_module,
            "case {name}"
        );
        assert_eq!(
            compilation.diagnostics.len(),
            case.expected_diagnostics.len(),
            "case {name} diagnostic count"
        );
        for (actual, expected) in compilation
            .diagnostics
            .iter()
            .zip(&case.expected_diagnostics)
        {
            assert_eq!(actual.rule_id, expected.rule_id, "case {name}");
            assert_eq!(
                serde_json::to_value(&actual.severity)
                    .unwrap()
                    .as_str()
                    .unwrap(),
                expected.severity,
                "case {name}"
            );
            if let Some(span) = &expected.span {
                assert_eq!(actual.span.start, span.start, "case {name} span start");
                assert_eq!(actual.span.end, span.end, "case {name} span end");
            }
        }

        assert_eq!(
            compilation
                .module
                .as_ref()
                .map_or(0, |module| module.derivations.len()),
            usize::from(case.expected_derivation.is_some()),
            "case {name} derivation count"
        );

        let derivation = compilation.module.as_ref().and_then(|module| {
            module.derivations.first().map(|derivation| {
                let DerivationExpression::Sum { source_field_id } = &derivation.expression;
                ExpectedDerivation {
                    target_field_id: derivation.target_field_id.to_string(),
                    source_field_id: source_field_id.to_string(),
                    recalculate_when_changed_field_ids: derivation
                        .recalculate_when_changed_field_ids
                        .iter()
                        .map(ToString::to_string)
                        .collect(),
                }
            })
        });
        assert_eq!(derivation, case.expected_derivation, "case {name}");
    }

    assert_eq!(
        categories,
        ["boundary", "failure", "false_positive", "normal"]
            .into_iter()
            .map(str::to_owned)
            .collect()
    );
}

fn read_case(directory: &Path) -> Case {
    let path = directory.join("case.json");
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .to_path_buf()
}
