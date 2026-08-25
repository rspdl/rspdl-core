use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use rspdl_compiler::compile_ko;
use rspdl_domain::{CreationDecision, FieldProducerSource, ProducerPhase, ProductionCardinality};
use serde::Deserialize;

#[derive(Deserialize)]
struct Case {
    spec_version: String,
    locale: String,
    category: String,
    expected_module: bool,
    expected_diagnostics: Vec<ExpectedDiagnostic>,
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
fn field_producer_conformance_suite() {
    let root = repository_root().join("conformance/ko-KR/field-producers");
    let mut dirs = fs::read_dir(root)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect::<Vec<_>>();
    dirs.sort();
    let mut categories = BTreeSet::new();
    for dir in dirs {
        let name = dir.file_name().unwrap().to_string_lossy();
        let case: Case =
            serde_json::from_str(&fs::read_to_string(dir.join("case.json")).unwrap()).unwrap();
        assert_eq!(case.spec_version, "0.4.0");
        assert_eq!(case.locale, "ko-KR");
        categories.insert(case.category);
        let source = fs::read_to_string(dir.join("input.rspdl")).unwrap();
        let compiled = compile_ko(&source);
        assert_eq!(compiled, compile_ko(&source), "{name} repeat");
        assert_eq!(compiled.module.is_some(), case.expected_module, "{name}");
        assert_eq!(
            actual_diagnostics(&compiled.diagnostics),
            expected_diagnostics(&case.expected_diagnostics),
            "{name}"
        );
        if name == "normal" {
            assert_eq!(
                producers(compiled.module.as_ref().unwrap()),
                vec![
                    (
                        "production.request_title_binding".into(),
                        "production.notice.request_title".into(),
                        "input_field:production.assign.target_request:production.request.title"
                            .into()
                    ),
                    (
                        "production.retry_binding".into(),
                        "production.notice.retry_count".into(),
                        "constant:integer:0".into()
                    ),
                    (
                        "production.title_binding".into(),
                        "production.notice.title".into(),
                        "action_input:production.assign.notification_title".into()
                    )
                ]
            );
        }
        if name == "boundary-explicit-values" {
            assert_eq!(
                producers(compiled.module.as_ref().unwrap()),
                vec![
                    (
                        "production.empty_binding".into(),
                        "production.notice.body".into(),
                        "constant:string:".into(),
                    ),
                    (
                        "production.false_binding".into(),
                        "production.notice.success".into(),
                        "constant:boolean:false".into(),
                    ),
                ]
            );
        }
        if let Some(module) = compiled.module.as_ref() {
            for production in &module.conditional_productions {
                assert_eq!(
                    production.instance_cardinality,
                    ProductionCardinality::ExactlyOne
                );
                assert_eq!(production.branches.len(), 2);
                assert!(
                    production
                        .branches
                        .iter()
                        .any(|branch| branch.decision == CreationDecision::Skip)
                );
                if name != "boundary-all-skip" {
                    assert!(
                        production
                            .branches
                            .iter()
                            .any(|branch| branch.decision == CreationDecision::Create)
                    );
                }
            }
        }
        if let Some(alt) = case.alternate_source {
            let alternate_source = fs::read_to_string(dir.join(alt)).unwrap();
            let alternate = compile_ko(&alternate_source);
            assert_eq!(
                alternate,
                compile_ko(&alternate_source),
                "{name} alternate repeat"
            );
            assert_eq!(
                actual_diagnostics(&compiled.diagnostics),
                actual_diagnostics(&alternate.diagnostics)
            );
            assert_eq!(
                compiled.module.as_ref().map(producers),
                alternate.module.as_ref().map(producers)
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
fn producers(module: &rspdl_domain::SemanticModule) -> Vec<(String, String, String)> {
    module
        .conditional_productions
        .iter()
        .flat_map(|p| {
            p.field_producers.iter().map(|f| {
                assert_eq!(f.phase, ProducerPhase::PreMutation);
                let s = match &f.source {
                    FieldProducerSource::ActionInput { input_id } => {
                        format!("action_input:{input_id}")
                    }
                    FieldProducerSource::InputField { input_id, field_id } => {
                        format!("input_field:{input_id}:{field_id}")
                    }
                    FieldProducerSource::EventInput { .. }
                    | FieldProducerSource::EventInputField { .. } => {
                        panic!("Action field-producer conformance cannot contain an Event source")
                    }
                    FieldProducerSource::Constant { value } => format!(
                        "constant:{}:{}",
                        value.value_type(),
                        value
                            .as_integer()
                            .map(ToString::to_string)
                            .or_else(|| value.as_boolean().map(|value| value.to_string()))
                            .or_else(|| value.as_string().map(str::to_owned))
                            .unwrap_or_else(|| value
                                .as_enum_variant()
                                .map(ToString::to_string)
                                .unwrap_or_default())
                    ),
                    FieldProducerSource::Template { parts } => format!(
                        "template:{}",
                        parts
                            .iter()
                            .map(|part| match part {
                                rspdl_domain::TemplatePart::Text { value } =>
                                    format!("text:{value}"),
                                rspdl_domain::TemplatePart::OutputField { field_id } =>
                                    format!("field:{field_id}"),
                            })
                            .collect::<Vec<_>>()
                            .join("|")
                    ),
                };
                (f.id.to_string(), f.output_field_id.to_string(), s)
            })
        })
        .collect()
}
fn expected_diagnostics(
    values: &[ExpectedDiagnostic],
) -> Vec<(String, String, String, BTreeMap<String, String>)> {
    values
        .iter()
        .map(|d| {
            (
                d.rule_id.clone(),
                d.severity.clone(),
                d.message_key.clone(),
                d.arguments.clone(),
            )
        })
        .collect()
}
fn actual_diagnostics(
    values: &[rspdl_domain::Diagnostic],
) -> Vec<(String, String, String, BTreeMap<String, String>)> {
    values
        .iter()
        .map(|d| {
            (
                d.rule_id.clone(),
                serde_json::to_value(d.severity)
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_owned(),
                d.message_key.clone(),
                d.arguments.clone(),
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
