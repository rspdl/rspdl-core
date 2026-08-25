use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use rspdl_compiler::compile_ko;
use rspdl_domain::{FieldProducerCondition, FieldProducerSource};
use serde::Deserialize;

#[derive(Deserialize)]
struct Case {
    spec_version: String,
    locale: String,
    category: String,
    expected_module: bool,
    expected_diagnostics: Vec<ExpectedDiagnostic>,
    expected_producers: Option<Vec<(String, String, String, String)>>,
    #[serde(default)]
    alternate_source: Option<String>,
}

#[derive(Clone, Deserialize)]
struct ExpectedDiagnostic {
    rule_id: String,
    severity: String,
    message_key: String,
    arguments: BTreeMap<String, String>,
}

#[test]
fn conditional_field_producer_conformance_suite() {
    let root = repository_root().join("conformance/ko-KR/conditional-field-producers");
    let mut directories = fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    directories.sort();
    let mut categories = BTreeSet::new();
    for directory in directories {
        let name = directory.file_name().unwrap().to_string_lossy();
        let case: Case =
            serde_json::from_str(&fs::read_to_string(directory.join("case.json")).unwrap())
                .unwrap();
        assert_eq!(case.spec_version, "0.4.0", "{name}");
        assert_eq!(case.locale, "ko-KR", "{name}");
        categories.insert(case.category.clone());
        let source = fs::read_to_string(directory.join("input.rspdl")).unwrap();
        let output = compile_ko(&source);
        assert_eq!(output, compile_ko(&source), "{name} repeat");
        assert_eq!(output.module.is_some(), case.expected_module, "{name}");
        assert_eq!(
            diagnostics(&output.diagnostics),
            expected_diagnostics(&case.expected_diagnostics),
            "{name}"
        );
        if let Some(expected_producers) = case.expected_producers {
            assert_eq!(
                output.module.as_ref().map(producers).unwrap_or_default(),
                expected_producers,
                "{name}"
            );
        }
        if let Some(alternate) = case.alternate_source {
            let alternate = compile_ko(&fs::read_to_string(directory.join(alternate)).unwrap());
            assert_eq!(
                diagnostics(&output.diagnostics),
                diagnostics(&alternate.diagnostics),
                "{name} diagnostics"
            );
            assert_eq!(
                output.module.as_ref().map(producers),
                alternate.module.as_ref().map(producers),
                "{name} projection"
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

fn producers(module: &rspdl_domain::SemanticModule) -> Vec<(String, String, String, String)> {
    module
        .conditional_productions
        .iter()
        .flat_map(|production| production.field_producers.iter())
        .map(|producer| {
            let source = match &producer.source {
                FieldProducerSource::ActionInput { input_id } => format!("action_input:{input_id}"),
                FieldProducerSource::InputField { input_id, field_id } => {
                    format!("input_field:{input_id}:{field_id}")
                }
                FieldProducerSource::Constant { value } => {
                    format!("constant:{}:{value:?}", value.value_type())
                }
                FieldProducerSource::Template { parts } => format!(
                    "template:{}",
                    parts
                        .iter()
                        .map(|part| match part {
                            rspdl_domain::TemplatePart::Text { value } => format!("text:{value}"),
                            rspdl_domain::TemplatePart::OutputField { field_id } =>
                                format!("field:{field_id}"),
                        })
                        .collect::<Vec<_>>()
                        .join("|")
                ),
            };
            let condition = match &producer.condition {
                Some(FieldProducerCondition::EnumVariant {
                    input_id,
                    variant_id,
                }) => format!("{input_id}:{variant_id}"),
                None => "always".into(),
            };
            (
                producer.id.to_string(),
                producer.output_field_id.to_string(),
                source,
                condition,
            )
        })
        .collect()
}

fn diagnostics(
    values: &[rspdl_domain::Diagnostic],
) -> Vec<(String, String, String, BTreeMap<String, String>)> {
    values
        .iter()
        .map(|diagnostic| {
            (
                diagnostic.rule_id.clone(),
                serde_json::to_value(diagnostic.severity)
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_owned(),
                diagnostic.message_key.clone(),
                diagnostic.arguments.clone(),
            )
        })
        .collect()
}

fn expected_diagnostics(
    values: &[ExpectedDiagnostic],
) -> Vec<(String, String, String, BTreeMap<String, String>)> {
    values
        .iter()
        .map(|diagnostic| {
            (
                diagnostic.rule_id.clone(),
                diagnostic.severity.clone(),
                diagnostic.message_key.clone(),
                diagnostic.arguments.clone(),
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
