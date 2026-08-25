use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use rspdl_compiler::compile_ko;
use rspdl_domain::ActionInputKind;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Case {
    spec_version: String,
    locale: String,
    category: String,
    expected_module: bool,
    expected_diagnostics: Vec<ExpectedDiagnostic>,
    expected_inputs: Vec<ExpectedInput>,
    #[serde(default)]
    alternate_source: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct ExpectedDiagnostic {
    rule_id: String,
    severity: String,
    message_key: String,
    #[serde(default)]
    arguments: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct ExpectedInput {
    action_id: String,
    id: String,
    local_id: String,
    name: String,
    kind: String,
    target: String,
}

#[test]
fn sentence_shaped_action_input_conformance_suite() {
    let root = repository_root().join("conformance/ko-KR/action-inputs");
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
        assert_eq!(case.spec_version, "0.3.0", "case {name}");
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
        assert_eq!(diagnostics, case.expected_diagnostics, "case {name}");

        let inputs = input_projection(compilation.module.as_ref());
        assert_eq!(inputs, case.expected_inputs, "case {name}");

        if let Some(alternate_source) = &case.alternate_source {
            let alternate = fs::read_to_string(directory.join(alternate_source)).unwrap();
            let alternate_compilation = compile_ko(&alternate);
            assert!(alternate_compilation.diagnostics.is_empty(), "case {name}");
            assert_eq!(
                input_projection(alternate_compilation.module.as_ref()),
                inputs,
                "case {name} semantic identity must not depend on declaration order"
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

fn input_projection(module: Option<&rspdl_domain::SemanticModule>) -> Vec<ExpectedInput> {
    module
        .into_iter()
        .flat_map(|module| &module.actions)
        .flat_map(|action| {
            action.inputs.iter().map(|input| {
                let (kind, target) = match &input.kind {
                    ActionInputKind::ExistingModel { model_id } => {
                        ("existing_model", model_id.to_string())
                    }
                    ActionInputKind::Value { value_type } => ("value", value_type.to_string()),
                };
                ExpectedInput {
                    action_id: action.id.to_string(),
                    id: input.id.to_string(),
                    local_id: input.local_id.to_string(),
                    name: input.name.clone(),
                    kind: kind.into(),
                    target,
                }
            })
        })
        .collect()
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
