use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use rspdl_compiler::compile_ko;
use rspdl_domain::{CreationDecision, ProductionCardinality};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Case {
    spec_version: String,
    locale: String,
    category: String,
    expected_module: bool,
    expected_diagnostics: Vec<ExpectedDiagnostic>,
    #[serde(default)]
    expected_branches: Vec<(String, String, String)>,
    #[serde(default)]
    alternate_source: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedDiagnostic {
    rule_id: String,
    severity: String,
    message_key: String,
    arguments: BTreeMap<String, String>,
}

#[test]
fn conditional_production_conformance_suite() {
    let root = repository_root().join("conformance/ko-KR/conditional-production");
    let mut directories = fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    directories.sort();
    let mut categories = BTreeSet::new();

    for directory in directories {
        let name = directory.file_name().unwrap().to_string_lossy();
        let case: Case =
            serde_json::from_str(&fs::read_to_string(directory.join("case.json")).unwrap())
                .unwrap();
        assert_eq!(case.spec_version, "0.4.0", "case {name}");
        assert_eq!(case.locale, "ko-KR", "case {name}");
        categories.insert(case.category.clone());
        let input = fs::read_to_string(directory.join("input.rspdl")).unwrap();
        let compilation = compile_ko(&input);
        assert_eq!(
            compilation,
            compile_ko(&input),
            "case {name} repeat determinism"
        );
        assert_eq!(
            compilation.module.is_some(),
            case.expected_module,
            "case {name}"
        );
        let diagnostics = compilation
            .diagnostics
            .iter()
            .map(|diagnostic| ExpectedDiagnostic {
                rule_id: diagnostic.rule_id.clone(),
                severity: serde_json::to_value(diagnostic.severity)
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_owned(),
                message_key: diagnostic.message_key.clone(),
                arguments: diagnostic.arguments.clone(),
            })
            .collect::<Vec<_>>();
        assert_eq!(
            diagnostic_projection(&diagnostics),
            diagnostic_projection(&case.expected_diagnostics),
            "case {name}"
        );
        if !case.expected_branches.is_empty() {
            let module = compilation.module.as_ref().expect("case expects a module");
            assert_eq!(
                branch_projection(module),
                case.expected_branches,
                "case {name}"
            );
        }
        if let Some(alternate) = case.alternate_source {
            let alternate_source = fs::read_to_string(directory.join(alternate)).unwrap();
            let alternate = compile_ko(&alternate_source);
            assert_eq!(
                alternate,
                compile_ko(&alternate_source),
                "case {name} alternate repeat determinism"
            );
            assert_eq!(
                alternate.module.is_some(),
                case.expected_module,
                "case {name} alternate module"
            );
            assert_eq!(
                diagnostic_projection(&diagnostics),
                diagnostic_projection(
                    &alternate
                        .diagnostics
                        .iter()
                        .map(|diagnostic| ExpectedDiagnostic {
                            rule_id: diagnostic.rule_id.clone(),
                            severity: serde_json::to_value(diagnostic.severity)
                                .unwrap()
                                .as_str()
                                .unwrap()
                                .to_owned(),
                            message_key: diagnostic.message_key.clone(),
                            arguments: diagnostic.arguments.clone()
                        })
                        .collect::<Vec<_>>()
                ),
                "case {name} diagnostic ordering"
            );
            assert_eq!(
                alternate.module.as_ref().map(branch_projection),
                compilation.module.as_ref().map(branch_projection),
                "case {name} semantic projection"
            );
        }
    }
    assert_eq!(
        categories,
        ["normal", "failure", "boundary", "false_positive"]
            .into_iter()
            .map(str::to_owned)
            .collect()
    );
}

fn branch_projection(module: &rspdl_domain::SemanticModule) -> Vec<(String, String, String)> {
    module
        .conditional_productions
        .iter()
        .flat_map(|production| {
            assert_eq!(
                production.instance_cardinality,
                ProductionCardinality::ExactlyOne
            );
            production.branches.iter().map(|branch| {
                (
                    branch.id.to_string(),
                    branch.variant_id.to_string(),
                    match branch.decision {
                        CreationDecision::Create => "create".to_owned(),
                        CreationDecision::Skip => "skip".to_owned(),
                    },
                )
            })
        })
        .collect()
}

fn diagnostic_projection(
    diagnostics: &[ExpectedDiagnostic],
) -> Vec<(&str, &str, &str, &BTreeMap<String, String>)> {
    diagnostics
        .iter()
        .map(|diagnostic| {
            (
                diagnostic.rule_id.as_str(),
                diagnostic.severity.as_str(),
                diagnostic.message_key.as_str(),
                &diagnostic.arguments,
            )
        })
        .collect()
}

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .to_path_buf()
}
