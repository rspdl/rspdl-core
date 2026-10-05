use rspdl_compiler::{
    CheckOptions, KoSource, ModelFindingOptions, check_ko, check_ko_files, compile_ko,
    compile_ko_files, find_ko_model,
};
use rspdl_domain::{
    BoundedModelResult, StatementAnalysisStatus, StatementCondition, StatementPolicy,
    semantic_references,
};
use serde::Deserialize;
use std::{collections::BTreeSet, fs, path::PathBuf};

const NORMAL: &str =
    include_str!("../../../conformance/ko-KR/statements/normal-nested/input.rspdl");
#[derive(Deserialize)]
struct Case {
    spec_version: String,
    locale: String,
    category: String,
    expected_module: bool,
    expected_message_keys: Vec<String>,
}
fn semantic(value: &impl serde::Serialize) -> serde_json::Value {
    fn strip(v: &mut serde_json::Value) {
        match v {
            serde_json::Value::Object(o) => {
                o.retain(|key, _| key != "span" && !key.ends_with("_span"));
                for v in o.values_mut() {
                    strip(v);
                }
            }
            serde_json::Value::Array(a) => a.iter_mut().for_each(strip),
            _ => {}
        }
    }
    let mut v = serde_json::to_value(value).unwrap();
    strip(&mut v);
    v
}
fn valid(source: &str) -> rspdl_domain::SemanticModule {
    let result = compile_ko(source);
    assert!(
        !result.diagnostics.iter().any(|d| d.is_error()),
        "{:#?}",
        result.diagnostics
    );
    result.module.unwrap()
}
fn error(source: &str, key: &str) {
    let result = compile_ko(source);
    assert!(result.module.is_none(), "{result:#?}");
    assert!(
        result.diagnostics.iter().any(|d| d.message_key == key),
        "expected {key}: {:#?}",
        result.diagnostics
    );
}
#[test]
fn public_statement_fixtures_cover_normal_failure_boundary_and_false_positive() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conformance/ko-KR/statements");
    let mut paths = fs::read_dir(root)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.is_dir())
        .collect::<Vec<_>>();
    paths.sort();
    let mut categories = BTreeSet::new();
    for path in paths {
        let case: Case =
            serde_json::from_str(&fs::read_to_string(path.join("case.json")).unwrap()).unwrap();
        assert_eq!(case.spec_version, "0.3.0");
        assert_eq!(case.locale, "ko-KR");
        categories.insert(case.category);
        let source = fs::read_to_string(path.join("input.rspdl")).unwrap();
        let result = compile_ko(&source);
        assert_eq!(result, compile_ko(&source));
        assert_eq!(
            result.module.is_some(),
            case.expected_module,
            "{}: {:#?}",
            path.display(),
            result.diagnostics
        );
        assert_eq!(
            result
                .diagnostics
                .iter()
                .map(|d| d.message_key.clone())
                .collect::<Vec<_>>(),
            case.expected_message_keys,
            "{}",
            path.display()
        );
    }
    assert_eq!(
        categories,
        ["normal", "failure", "boundary", "false_positive"]
            .into_iter()
            .map(str::to_owned)
            .collect()
    );
}
#[test]
fn nested_conditions_and_multiple_assignments_preserve_explicit_intent() {
    let module = valid(NORMAL);
    let statement = &module.statements[0];
    assert_eq!(
        statement.analysis_status,
        StatementAnalysisStatus::LinkedAndTypeCheckedOnly
    );
    assert_eq!(statement.policy, StatementPolicy::Can);
    assert_eq!(statement.bindings.len(), 2);
    assert_eq!(statement.effects.len(), 2);
    let StatementCondition::And(children) = &statement.condition else {
        panic!("{:#?}", statement.condition)
    };
    assert_eq!(children.len(), 2);
    assert!(children.iter().any(|c| matches!(c, StatementCondition::Or(xs) if xs.iter().any(|x| matches!(x, StatementCondition::Not(_))))));
    let refs = semantic_references(&module);
    assert!(refs.iter().any(|r| r.from.kind == "statements"));
    assert!(
        refs.iter()
            .filter(|r| r.from.kind.starts_with("statement"))
            .all(|r| r.span.end > r.span.start)
    );
}
#[test]
fn formatter_is_idempotent_and_preserves_typed_semantics() {
    let first = rspdl_ko::format_source(NORMAL);
    assert!(first.diagnostics.is_empty(), "{:#?}", first.diagnostics);
    let text = first.text.unwrap();
    let second = rspdl_ko::format_source(&text);
    assert_eq!(second.text.as_deref(), Some(text.as_str()));
    assert_eq!(
        semantic(&valid(NORMAL).statements),
        semantic(&valid(&text).statements)
    );
}
#[test]
fn statement_clause_effect_and_assignment_order_is_not_priority() {
    let alternate = NORMAL.replace("    대상(target)은 변경의 대상 기록 입력을 사용한다.\n    값(value)은 변경의 새 횟수 입력을 사용한다.", "    값(value)은 변경의 새 횟수 입력을 사용한다.\n    대상(target)은 변경의 대상 기록 입력을 사용한다.")
        .replace("        횟수를 값으로 정한다.\n        제목을 \"변경됨\"으로 정한다.", "        제목을 \"변경됨\"으로 정한다.\n        횟수를 값으로 정한다.");
    let alternate = format!(
        "{}    대상을 조회한다.\n",
        alternate.replace("    대상을 조회한다.\n", "")
    );
    assert_ne!(alternate, NORMAL);
    assert_eq!(
        semantic(&valid(NORMAL).statements),
        semantic(&valid(&alternate).statements)
    );
}
#[test]
fn aliases_to_the_same_input_do_not_evade_write_or_delete_conflicts() {
    let alias = NORMAL.replace(
        "    값(value)은",
        "    다른 대상(other)은 변경의 대상 기록 입력을 사용한다.\n    값(value)은",
    );
    error(
        &format!("{alias}    다른 대상을 수정한다.\n        횟수를 1로 정한다.\n"),
        "semantic.statement.duplicate_write",
    );
    let source = format!("{alias}    다른 대상을 수정한다.\n        횟수를 1로 정한다.\n");
    let compilation = compile_ko(&source);
    let duplicate = compilation
        .diagnostics
        .iter()
        .find(|d| d.message_key == "semantic.statement.duplicate_write")
        .unwrap();
    assert_eq!(duplicate.rule_id, "RSPDL-STMT-008");
    assert_eq!(
        duplicate.arguments.get("input_id").map(String::as_str),
        Some("statement.change.target_record")
    );
    assert_eq!(
        duplicate.arguments.get("field_id").map(String::as_str),
        Some("statement.record.count")
    );
    assert_eq!(
        duplicate.arguments.get("statement_id").map(String::as_str),
        Some("statement.change_record")
    );
    assert!(source[duplicate.span.start..duplicate.span.end].contains("횟수를 1로 정한다"));
    error(
        &format!("{alias}    다른 대상을 삭제한다.\n"),
        "semantic.statement.delete_access_conflict",
    );
}
#[test]
fn foreign_trigger_inputs_and_scalar_effect_targets_are_rejected() {
    let foreign = NORMAL.replace("기록 변경(change_record)", "다른 변경(other_change)은 행동이다.\n다른 변경은 정수를 다른 횟수(other_count)로 입력받는다.\n기록 변경(change_record)").replace("값(value)은 변경의 새 횟수 입력을", "값(value)은 다른 변경의 다른 횟수 입력을");
    error(&foreign, "ko.statement.binding_trigger_mismatch");
    error(
        &NORMAL.replace("대상을 조회한다.", "값을 조회한다."),
        "semantic.statement.effect_requires_model",
    );
}
#[test]
fn can_cannot_and_attempt_are_distinct_and_one_modality_is_required() {
    assert_eq!(
        valid(&NORMAL.replace("할 수 있다.", "할 수 없다.")).statements[0].policy,
        StatementPolicy::Cannot
    );
    assert_eq!(
        valid(&NORMAL.replace("할 수 있다.", "자동으로 시도한다.")).statements[0].policy,
        StatementPolicy::DoAttempt
    );
    error(
        &NORMAL.replace(
            "    이 처리를 할 수 있다.",
            "    이 처리를 할 수 있다.\n    이 처리를 할 수 없다.",
        ),
        "ko.syntax.statement_invalid",
    );
}
#[test]
fn compiled_statements_are_explicitly_unsupported_at_execution_and_model_finding() {
    let single = check_ko(NORMAL, "{}", CheckOptions::default());
    assert!(single.has_errors());
    assert!(
        single
            .runtime_diagnostics
            .iter()
            .any(|d| d.rule_id == "RSPDL-STMT-090"
                && d.message_key == "runtime.statement.unsupported")
    );
    let workspace = check_ko_files(
        vec![KoSource::new("statement.rspdl", NORMAL)],
        "{}",
        CheckOptions::default(),
    );
    assert!(workspace.has_errors());
    assert!(
        workspace
            .runtime_diagnostics
            .iter()
            .any(|d| d.rule_id == "RSPDL-STMT-090")
    );
    let model = find_ko_model(NORMAL, ModelFindingOptions::default());
    assert!(model.has_errors());
    assert!(
        matches!(model.result, Some(BoundedModelResult::Unsupported { ref constructs, .. }) if constructs.iter().any(|c| c == "statement:statement.change_record")),
        "{model:#?}"
    );
}
#[test]
fn event_snapshot_can_be_read_but_cannot_be_mutated() {
    let event = NORMAL
        .replace("변경(change)은 행동이다.", "변경(change)은 사건이다.")
        .replace("으로 입력받는다.", "으로 담는다.")
        .replace("로 입력받는다.", "로 담는다.")
        .replace("실행될 때", "발생할 때");
    let read = event.replace("    대상을 수정한다.\n        횟수를 값으로 정한다.\n        제목을 \"변경됨\"으로 정한다.\n", "");
    valid(&read);
    error(&event, "semantic.statement.event_payload_read_only");
}

#[test]
fn optional_field_absence_is_not_treated_as_a_boolean_value() {
    error(
        &NORMAL.replace("완료(done): 필수 불리언", "완료(done): 선택 불리언"),
        "semantic.statement.optional_operand_unsupported",
    );
}

#[test]
fn sibling_conditions_and_named_statements_are_canonical_semantic_sets() {
    let group = "        값이 0보다 크거나 같다.\n        다음 조건 중 하나 이상을 만족할 때 적용한다.\n            대상의 완료가 거짓과 같다.\n            다음 조건을 만족하지 않을 때 적용한다.\n                대상의 제목이 \"잠김\"과 같다.\n";
    let reverse = "        다음 조건 중 하나 이상을 만족할 때 적용한다.\n            다음 조건을 만족하지 않을 때 적용한다.\n                대상의 제목이 \"잠김\"과 같다.\n            대상의 완료가 거짓과 같다.\n        값이 0보다 크거나 같다.\n";
    let alternate = NORMAL.replace(group, reverse);
    assert_ne!(alternate, NORMAL);
    assert_eq!(
        semantic(&valid(NORMAL).statements),
        semantic(&valid(&alternate).statements)
    );
    let extra = "삭제 허용(delete_allowed)은 다음과 같이 정한다.\n    변경이 실행될 때 적용한다.\n    시스템이 수행한다.\n    대상(target)은 변경의 대상 기록 입력을 사용한다.\n    이 처리를 할 수 있다.\n    대상을 삭제한다.\n";
    let header = "기록 변경(change_record)은 다음과 같이 정한다.";
    let (declarations, original) = NORMAL.split_once(header).unwrap();
    let original = format!("{header}{original}");
    let forward = format!("{declarations}{original}{extra}");
    let reverse = format!("{declarations}{extra}{original}");
    assert_eq!(
        semantic(&valid(&forward).statements),
        semantic(&valid(&reverse).statements)
    );
}

#[test]
fn nested_boolean_groups_preserve_written_group_boundaries() {
    let start = NORMAL.find("    다음 조건을 모두").unwrap();
    let end = NORMAL.find("    이 처리를").unwrap();
    for (group, expected) in [
        (
            "    다음 조건 중 하나 이상을 만족할 때 적용한다.\n        다음 조건을 모두 만족할 때 적용한다.\n            값이 0보다 크다.\n            대상의 완료가 거짓과 같다.\n",
            "or",
        ),
        (
            "    다음 조건을 만족하지 않을 때 적용한다.\n        다음 조건을 모두 만족할 때 적용한다.\n            값이 0보다 크다.\n            대상의 완료가 거짓과 같다.\n",
            "not",
        ),
    ] {
        let source = format!("{}{group}{}", &NORMAL[..start], &NORMAL[end..]);
        let module = valid(&source);
        match (&module.statements[0].condition, expected) {
            (StatementCondition::Or(xs), "or") => {
                assert!(matches!(xs.as_slice(), [StatementCondition::And(ys)] if ys.len() == 2))
            }
            (StatementCondition::Not(inner), "not") => {
                assert!(matches!(inner.as_ref(), StatementCondition::And(ys) if ys.len() == 2))
            }
            (other, _) => panic!("{other:#?}"),
        }
    }
    let group = "    다음 조건 중 하나 이상을 만족할 때 적용한다.\n        다음 조건을 모두 만족할 때 적용한다.\n            값이 0보다 크다.\n            대상의 완료가 거짓과 같다.\n        대상의 제목이 \"열림\"과 같다.\n";
    let source = format!("{}{group}{}", &NORMAL[..start], &NORMAL[end..]);
    let module = valid(&source);
    assert!(
        matches!(&module.statements[0].condition, StatementCondition::Or(xs) if xs.len() == 2 && xs.iter().any(|x| matches!(x, StatementCondition::And(ys) if ys.len() == 2)))
    );
}

#[test]
fn authored_names_and_clause_reference_provenance_are_preserved() {
    let module = valid(NORMAL);
    let statement = &module.statements[0];
    assert_eq!(statement.name, "기록 변경");
    assert!(statement.bindings.iter().any(|b| b.name == "대상"));
    let refs = semantic_references(&module);
    let trigger = refs
        .iter()
        .find(|r| r.from.kind == "statements" && r.field == "trigger")
        .unwrap();
    assert!(NORMAL[trigger.span.start..trigger.span.end].contains("실행될"));
    let actor = refs
        .iter()
        .find(|r| r.from.kind == "statements" && r.field == "actor")
        .unwrap();
    assert!(NORMAL[actor.span.start..actor.span.end].contains("수행한다"));
    let field = refs
        .iter()
        .find(|r| r.from.kind == "statements" && r.field == "operand.field_id")
        .unwrap();
    let text = &NORMAL[field.span.start..field.span.end];
    assert!(text.contains("완료") || text.contains("제목"));
    assert!(!text.contains("다음과 같이 정한다"));
    let invalid = NORMAL.replace("대상의 완료가", "대상의 없는 필드가");
    let compilation = compile_ko(&invalid);
    let diagnostic = compilation
        .diagnostics
        .iter()
        .find(|d| d.message_key == "ko.reference.not_found")
        .unwrap();
    assert!(invalid[diagnostic.span.start..diagnostic.span.end].contains("없는 필드"));
}
#[test]
fn unsupported_create_arithmetic_and_implicit_clock_are_syntax_errors() {
    error(
        &NORMAL.replace("대상을 조회한다.", "대상을 생성한다."),
        "ko.syntax.statement_invalid",
    );
    let arithmetic =
        compile_ko(&NORMAL.replace("횟수를 값으로 정한다.", "횟수를 값 + 1로 정한다."));
    assert!(arithmetic.module.is_none());
    assert!(arithmetic.diagnostics.iter().any(|d| d.is_error()));
    error(
        &NORMAL.replace(
            "변경이 실행될 때 적용한다.",
            "아홉 시간이 지날 때 적용한다.",
        ),
        "ko.syntax.statement_invalid",
    );
}

#[test]
fn typed_field_to_input_and_enum_constants_link_without_erasing_types() {
    valid(&NORMAL.replace(
        "값이 0보다 크거나 같다.",
        "대상의 횟수가 값보다 작거나 같다.",
    ));
    let source = NORMAL.replace("관리자(admin)는 역할이다.", "상태(status)는 다음 값 중 하나다.\n    열림(open)\n    닫힘(closed)\n관리자(admin)는 역할이다.")
        .replace("완료(done): 필수 불리언", "완료(done): 필수 불리언\n    상태(status): 필수 상태")
        .replace("대상의 완료가 거짓과 같다.", "대상의 상태가 열림과 같다.");
    let module = valid(&source);
    assert!(
        semantic_references(&module)
            .iter()
            .any(|r| r.to.kind == "enums.variants")
    );
    error(
        &NORMAL.replace("대상의 완료가 거짓과 같다.", "대상의 완료가 참보다 크다."),
        "semantic.statement.comparison_unsupported",
    );
}

#[test]
fn quoted_aliases_that_resemble_literals_or_field_paths_round_trip() {
    let source = NORMAL
        .replace("대상(target)은", "`대상 의 횟수`(target)은")
        .replace("대상의", "`대상 의 횟수`의")
        .replace("대상을", "`대상 의 횟수`를")
        .replace("값(value)은", "`참`(value)은")
        .replace("값이", "`참`이")
        .replace("값으로", "`참`으로");
    let original = valid(&source);
    assert!(
        original.statements[0]
            .bindings
            .iter()
            .any(|b| b.name == "참")
    );
    let formatted = rspdl_ko::format_source(&source);
    assert!(
        formatted.diagnostics.is_empty(),
        "{:#?}",
        formatted.diagnostics
    );
    let text = formatted.text.unwrap();
    assert_eq!(
        semantic(&original.statements),
        semantic(&valid(&text).statements)
    );
    assert_eq!(
        rspdl_ko::format_source(&text).text.as_deref(),
        Some(text.as_str())
    );
}

#[test]
fn workspace_rejects_duplicate_explicit_statement_ids_across_modules() {
    let first = NORMAL.replace("(change_record)", "(shared.rule)");
    let second = first.replace("@모듈 문장(statement)", "@모듈 두번째(second)");
    let sources = vec![
        KoSource::new("a.rspdl", first),
        KoSource::new("b.rspdl", second),
    ];
    let forward = compile_ko_files(sources.clone());
    let reverse = compile_ko_files(sources.into_iter().rev().collect());
    assert_eq!(forward, reverse);
    assert!(forward.has_errors());
    assert_eq!(forward.files.len(), 2);
    for file in forward.files {
        assert!(
            file.diagnostics
                .iter()
                .any(|d| d.message_key == "compiler.symbol.duplicate_id"),
            "{:#?}",
            file.diagnostics
        );
    }
}

#[test]
fn system_actor_keeps_its_explicit_clause_provenance() {
    let source = NORMAL.replace("관리자가 수행한다.", "시스템이 수행한다.");
    let module = valid(&source);
    let statement = &module.statements[0];
    assert!(matches!(
        statement.actor,
        rspdl_domain::StatementActor::System
    ));
    assert_eq!(
        &source[statement.actor_span.start..statement.actor_span.end],
        "시스템이 수행한다."
    );
}
