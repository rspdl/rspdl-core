use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use rspdl_compiler::{KoSource, compile_ko, compile_ko_files};
use rspdl_domain::{SemanticModule, TextRange};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ProvenanceCase {
    spec_version: String,
    locale: String,
    category: String,
    #[serde(default)]
    expected_policy_slices: Vec<String>,
    #[serde(default)]
    expected_model_slice: Option<String>,
    #[serde(default)]
    expected_field_slices: Vec<String>,
    #[serde(default)]
    alternate_source: Option<String>,
}

const POLICY_SOURCE: &str = r#"@모듈 이중(both)

문서(doc)는 다음 필드들로 구성되어 있다.
    제목(title): 필수 문자열

편집자(editor)는 역할이다.
수정(update)은 행동이다.

편집자는 문서의 제목을 수정할 수 있다.

편집자는 문서의 제목을 수정할 수 없다.
"#;

const CORE_SOURCE: &str = r#"@모듈 비용 승인(expense)

비용 상태(status)는 다음 값 중 하나다.
    작성 중(draft)
    승인됨(approved)

비용 신청(request)은 다음 필드들로 구성되어 있다.
    식별자(id): 필수 문자열
    금액(amount): 필수 정수
    상태(status): 필수 비용 상태

비용 신청의 금액은 0보다 커야 한다.

회계 관리자(accounting_manager)는 역할이다.
변경(change)은 행동이다.

회계 관리자는 비용 신청의 상태를 변경할 수 있다.
"#;

const FIELD_INTENT_SOURCE: &str = r#"@모듈 내부 메모(hidden)

항목(item)은 다음 필드들로 구성되어 있다.
    메모(note): 필수 문자열

항목 작성 화면(create_item)에서는 항목을 생성할 수 있다.
항목 작성 화면(create_item)에서는 항목의 메모를 입력할 수 있다.
항목의 메모는 내부 관리에만 사용한다.
"#;

fn slice(source: &str, span: TextRange) -> &str {
    source
        .get(span.start..span.end)
        .unwrap_or_else(|| panic!("invalid UTF-8 byte span {span:?}"))
}

fn compile_module(source: &str) -> SemanticModule {
    let compilation = compile_ko(source);
    assert!(
        !compilation
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.is_error()),
        "{:?}",
        compilation.diagnostics
    );
    compilation.module.expect("valid source should compile")
}

#[test]
fn policy_rows_keep_distinct_sentence_spans_without_a_conflict_diagnostic() {
    let compilation = compile_ko(POLICY_SOURCE);
    assert!(
        compilation.diagnostics.is_empty(),
        "{:?}",
        compilation.diagnostics
    );
    let module = compilation.module.expect("policy source should compile");

    assert_eq!(module.policies.len(), 2);
    assert_ne!(module.policies[0].span, module.policies[1].span);
    assert_eq!(
        slice(POLICY_SOURCE, module.policies[0].span),
        "편집자는 문서의 제목을 수정할 수 있다."
    );
    assert_eq!(
        slice(POLICY_SOURCE, module.policies[1].span),
        "편집자는 문서의 제목을 수정할 수 없다."
    );
}

#[test]
fn semantic_ir_retains_every_source_backed_record_span() {
    let core = compile_module(CORE_SOURCE);
    assert_eq!(slice(CORE_SOURCE, core.span), "@모듈 비용 승인(expense)");
    assert!(slice(CORE_SOURCE, core.enums[0].span).contains("승인됨(approved)"));
    assert_eq!(
        slice(CORE_SOURCE, core.enums[0].variants[0].span),
        "작성 중(draft)"
    );
    assert!(slice(CORE_SOURCE, core.models[0].span).contains("상태(status): 필수 비용 상태"));
    assert_eq!(
        slice(CORE_SOURCE, core.models[0].fields[1].span),
        "금액(amount): 필수 정수"
    );
    assert!(slice(CORE_SOURCE, core.constraints[0].span).ends_with("커야 한다."));
    assert!(slice(CORE_SOURCE, core.roles[0].span).ends_with("역할이다."));
    assert!(slice(CORE_SOURCE, core.actions[0].span).ends_with("행동이다."));
    assert!(slice(CORE_SOURCE, core.policies[0].span).ends_with("수 있다."));

    let relation_source = include_str!("../../../examples/project-ownership.rspdl");
    let relations = compile_module(relation_source);
    assert!(slice(relation_source, relations.relations[0].span).ends_with("가질 수 있다."));
    assert!(
        slice(relation_source, relations.relational_constraints[0].span).ends_with("해야 한다.")
    );

    let provenance_source = include_str!("../../../examples/field-provenance.rspdl");
    let provenance = compile_module(provenance_source);
    assert!(slice(provenance_source, provenance.screens[0].span).contains("화면"));
    assert!(
        slice(provenance_source, provenance.screens[0].operations[0].span).contains("할 수 있다.")
    );
    assert!(slice(provenance_source, provenance.derivations[0].span).contains("합계로 계산한다."));
    assert!(slice(provenance_source, provenance.recalculations[0].span).contains("다시 계산한다."));

    let mutation_source = include_str!(
        "../../../conformance/ko-KR/data-usage/normal-action-create-producer/input.rspdl"
    );
    let mutation = compile_module(mutation_source);
    assert!(slice(mutation_source, mutation.action_data_mutations[0].span).contains("실행되면"));

    let field_intent = compile_module(FIELD_INTENT_SOURCE);
    assert!(slice(FIELD_INTENT_SOURCE, field_intent.field_intents[0].span).ends_with("사용한다."));
}

#[test]
fn workspace_spans_are_relative_to_each_containing_file() {
    let sources = BTreeMap::from([
        ("policy.rspdl", POLICY_SOURCE),
        ("intent.rspdl", FIELD_INTENT_SOURCE),
    ]);
    let compilation = compile_ko_files(
        sources
            .iter()
            .map(|(path, source)| KoSource::new(*path, *source))
            .collect(),
    );
    assert!(!compilation.has_errors());

    for file in compilation.files {
        let source = sources[file.path.as_str()];
        let module = file.module.expect("valid workspace file should compile");
        assert!(module.span.end <= source.len());
        assert!(slice(source, module.span).starts_with("@모듈"));
        for model in module.models {
            assert!(model.span.end <= source.len());
            assert!(slice(source, model.span).contains("필드들로 구성되어 있다."));
        }
    }
}

#[test]
fn source_positions_do_not_participate_in_generated_policy_identity() {
    let shifted = POLICY_SOURCE.replace("\n편집자는", "\n\n\n편집자는");
    let original = compile_module(POLICY_SOURCE);
    let shifted = compile_module(&shifted);

    let original_ids = original
        .policies
        .iter()
        .map(|policy| policy.id.clone())
        .collect::<Vec<_>>();
    let shifted_ids = shifted
        .policies
        .iter()
        .map(|policy| policy.id.clone())
        .collect::<Vec<_>>();
    assert_eq!(original_ids, shifted_ids);
    assert_ne!(original.policies[0].span, shifted.policies[0].span);
}

#[test]
fn source_provenance_conformance_suite() {
    let root = repository_root().join("conformance/ko-KR/source-provenance");
    let mut directories = fs::read_dir(&root)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", root.display()))
        .map(|entry| entry.expect("conformance entry should be readable").path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    directories.sort();

    let mut categories = BTreeSet::new();
    for directory in directories {
        let name = directory.file_name().unwrap().to_string_lossy();
        let case: ProvenanceCase = serde_json::from_str(
            &fs::read_to_string(directory.join("case.json")).expect("case should be readable"),
        )
        .expect("case should be valid JSON");
        assert_eq!(case.spec_version, "0.3.0", "case {name}");
        assert_eq!(case.locale, "ko-KR", "case {name}");
        categories.insert(case.category.clone());

        let source = fs::read_to_string(directory.join("input.rspdl")).unwrap();
        let module = compile_module(&source);
        let policy_slices = module
            .policies
            .iter()
            .map(|policy| slice(&source, policy.span).to_owned())
            .collect::<Vec<_>>();
        assert_eq!(policy_slices, case.expected_policy_slices, "case {name}");

        if let Some(expected) = case.expected_model_slice {
            assert_eq!(
                slice(&source, module.models[0].span),
                expected,
                "case {name}"
            );
        }
        if !case.expected_field_slices.is_empty() {
            let actual = module.models[0]
                .fields
                .iter()
                .map(|field| slice(&source, field.span).to_owned())
                .collect::<Vec<_>>();
            assert_eq!(actual, case.expected_field_slices, "case {name}");
        }

        if let Some(alternate_source) = case.alternate_source {
            let alternate = fs::read_to_string(directory.join(alternate_source)).unwrap();
            let alternate_module = compile_module(&alternate);
            assert_eq!(
                module
                    .policies
                    .iter()
                    .map(|policy| &policy.id)
                    .collect::<Vec<_>>(),
                alternate_module
                    .policies
                    .iter()
                    .map(|policy| &policy.id)
                    .collect::<Vec<_>>(),
                "case {name} generated IDs"
            );
            assert_ne!(
                module.policies[0].span, alternate_module.policies[0].span,
                "case {name} spans"
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

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .to_path_buf()
}
