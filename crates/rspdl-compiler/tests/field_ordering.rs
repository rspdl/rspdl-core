use rspdl_compiler::{CheckOptions, check_ko, compile_ko};
use rspdl_domain::{ConstraintOperand, RelationOperator};

fn source(value_type: &str, requirement: &str, optional: bool) -> String {
    format!(
        "@모듈 일정(schedule)\n프로젝트(project)는 다음 필드들로 구성되어 있다.\n    시작일(start): 필수 {value_type}\n    마감일(end): {} {value_type}\n프로젝트의 마감일은 시작일보다 {requirement} 한다.\n",
        if optional { "선택" } else { "필수" }
    )
}

fn data(start: &str, end: Option<&str>) -> String {
    let mut record = serde_json::json!({"$id": "p", "start": start});
    if let Some(end) = end {
        record["end"] = end.into();
    }
    serde_json::json!({"records": {"schedule.project": [record]}}).to_string()
}

#[test]
fn strict_and_inclusive_field_ordering_preserves_explicit_boundary() {
    for (requirement, operator, before, equal, after) in [
        ("커야", RelationOperator::GreaterThan, false, false, true),
        ("작아야", RelationOperator::LessThan, true, false, false),
        (
            "크거나 같아야",
            RelationOperator::GreaterThanOrEqual,
            false,
            true,
            true,
        ),
        (
            "작거나 같아야",
            RelationOperator::LessThanOrEqual,
            true,
            true,
            false,
        ),
    ] {
        let source = source("날짜", requirement, false);
        let compiled = compile_ko(&source);
        assert!(
            compiled.diagnostics.is_empty(),
            "{:?}",
            compiled.diagnostics
        );
        let constraint = &compiled.module.unwrap().constraints[0];
        assert_eq!(constraint.operator, operator);
        assert!(matches!(constraint.right, ConstraintOperand::Field(_)));
        for (end, accepted) in [
            ("2026-10-04", before),
            ("2026-10-05", equal),
            ("2026-10-06", after),
        ] {
            let report = check_ko(
                &source,
                &data("2026-10-05", Some(end)),
                CheckOptions::default(),
            );
            assert!(
                report.runtime_diagnostics.is_empty(),
                "{:?}",
                report.runtime_diagnostics
            );
            assert_eq!(
                report.constraint_violations.is_empty(),
                accepted,
                "{requirement}: {end}"
            );
        }
        let formatted = rspdl_ko::format_source(&source).text.unwrap();
        let reparsed = compile_ko(&formatted);
        assert!(reparsed.diagnostics.is_empty());
        let reparsed = reparsed.module.unwrap();
        assert_eq!(reparsed.constraints[0].operator, operator);
        assert_eq!(reparsed.constraints[0].right, constraint.right);
    }
}

#[test]
fn datetime_order_uses_instants_and_missing_optional_field_keeps_absence_semantics() {
    let source = source("날짜시간", "커야", true);
    let start = "2026-10-05T12:00:00Z";
    for (end, violations) in [
        (Some("2026-10-05T13:00:00+01:00"), 1),
        (Some("2026-10-05T12:00:01Z"), 0),
        (Some("2026-10-05T11:59:59Z"), 1),
        (None, 0),
    ] {
        let report = check_ko(&source, &data(start, end), CheckOptions::default());
        assert!(
            report.runtime_diagnostics.is_empty(),
            "{:?}",
            report.runtime_diagnostics
        );
        assert_eq!(report.constraint_violations.len(), violations);
    }
}

#[test]
fn unknown_right_field_and_incompatible_types_are_structured_errors() {
    let unknown = source("날짜", "커야", false).replace("시작일보다", "없는 필드보다");
    let report = compile_ko(&unknown);
    assert!(report.module.is_none());
    assert!(
        report
            .diagnostics
            .iter()
            .any(
                |diagnostic| diagnostic.message_key == "ko.reference.not_found"
                    && diagnostic
                        .arguments
                        .get("kind")
                        .is_some_and(|kind| kind == "field")
                    && diagnostic.is_error()
            ),
        "{:?}",
        report.diagnostics
    );
    let mismatch = source("날짜", "커야", false)
        .replace("시작일(start): 필수 날짜", "시작일(start): 필수 정수");
    let report = compile_ko(&mismatch);
    assert!(report.module.is_none());
    assert!(report.diagnostics.iter().any(|diagnostic| diagnostic.message_key == "semantic.constraint.operand_type_mismatch"));
    let unordered = compile_ko(&source("불리언", "커야", false));
    assert!(
        unordered
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message_key
                == "semantic.constraint.order_requires_ordered_type")
    );
}

#[test]
fn field_ordering_conformance_is_deterministic() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../conformance/ko-KR/field-ordering");
    let mut cases = std::fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    cases.sort();
    let mut categories = std::collections::BTreeSet::new();
    for case in cases {
        let source = std::fs::read_to_string(case.join("input.rspdl")).unwrap();
        let expected: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(case.join("case.json")).unwrap())
                .unwrap();
        assert_eq!(expected["spec_version"], "0.4.0");
        assert_eq!(expected["locale"], "ko-KR");
        categories.insert(expected["category"].as_str().unwrap().to_owned());
        let diagnostics = expected["expected_compile_diagnostics"].as_array().unwrap();
        if !diagnostics.is_empty() {
            let first = compile_ko(&source);
            assert_eq!(first, compile_ko(&source));
            assert!(first.module.is_none(), "{}", case.display());
            assert_eq!(
                first
                    .diagnostics
                    .iter()
                    .map(|diagnostic| (
                        diagnostic.rule_id.as_str(),
                        diagnostic.message_key.as_str()
                    ))
                    .collect::<Vec<_>>(),
                diagnostics
                    .iter()
                    .map(|diagnostic| (
                        diagnostic["rule_id"].as_str().unwrap(),
                        diagnostic["message_key"].as_str().unwrap()
                    ))
                    .collect::<Vec<_>>(),
                "{}",
                case.display()
            );
            continue;
        }
        let data = std::fs::read_to_string(case.join("data.json")).unwrap();
        let first = check_ko(&source, &data, CheckOptions::default());
        let second = check_ko(&source, &data, CheckOptions::default());
        assert_eq!(first, second, "{}", case.display());
        assert!(
            first.compilation.diagnostics.is_empty(),
            "{:?}",
            first.compilation.diagnostics
        );
        assert!(
            first.runtime_diagnostics.is_empty(),
            "{:?}",
            first.runtime_diagnostics
        );
        if let Some(evidence) = expected["expected_violation_evidence"].as_array() {
            for (violation, evidence) in first.constraint_violations.iter().zip(evidence) {
                assert_eq!(
                    violation.model_id.to_string(),
                    evidence["model_id"].as_str().unwrap()
                );
                assert_eq!(violation.record_id, evidence["record_id"].as_str().unwrap());
                assert_eq!(
                    violation.left.canonical_text(),
                    evidence["left"].as_str().unwrap()
                );
                assert_eq!(
                    violation.right.canonical_text(),
                    evidence["right"].as_str().unwrap()
                );
                let constraint = first
                    .compilation
                    .module
                    .as_ref()
                    .unwrap()
                    .constraints
                    .iter()
                    .find(|constraint| constraint.id == violation.constraint_id)
                    .unwrap();
                assert!(
                    matches!(&constraint.left, ConstraintOperand::Field(id) if id.to_string() == evidence["left_field_id"].as_str().unwrap())
                );
                assert!(
                    matches!(&constraint.right, ConstraintOperand::Field(id) if id.to_string() == evidence["right_field_id"].as_str().unwrap())
                );
            }
        }
        assert_eq!(
            first.constraint_violations.len(),
            expected["expected_violations"].as_u64().unwrap() as usize,
            "{}",
            case.display()
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
fn field_ordering_reuses_canonical_comparable_types() {
    for (value_type, start, end) in [
        ("소수", "1.1", "1.2"),
        ("시간", "12:00:00", "12:00:01"),
        ("기간", "PT1S", "PT2S"),
        ("위도", "1.1", "1.2"),
        ("경도", "1.1", "1.2"),
        (
            "지역 날짜시간",
            "2026-10-05T12:00:00",
            "2026-10-05T12:00:01",
        ),
        (
            "시간대 날짜시간",
            "2026-10-05T12:00:00+09:00 Asia/Seoul",
            "2026-10-05T12:00:01+09:00 Asia/Seoul",
        ),
        ("통화(KRW)", "10 KRW", "11 KRW"),
        ("백분율", "10%", "11%"),
        ("수량(kg)", "10 kg", "11 kg"),
        ("날짜", "2026-10-05", "2026-10-06"),
        ("날짜시간", "2026-10-05T00:00:00Z", "2026-10-06T00:00:00Z"),
    ] {
        let source = source(value_type, "커야", false);
        let valid = check_ko(&source, &data(start, Some(end)), CheckOptions::default());
        assert!(!valid.has_errors(), "{value_type}: {valid:?}");
        assert!(valid.constraint_violations.is_empty(), "{value_type}");
        let invalid = check_ko(&source, &data(end, Some(start)), CheckOptions::default());
        assert!(!invalid.has_errors(), "{value_type}: {invalid:?}");
        assert_eq!(invalid.constraint_violations.len(), 1, "{value_type}");
    }
}

#[test]
fn integer_field_comparison_does_not_change_numeric_literal_comparisons() {
    let fields = source("정수", "커야", false);
    let valid = r#"{"records":{"schedule.project":[{"$id":"p","start":1,"end":2}]}}"#;
    let invalid = r#"{"records":{"schedule.project":[{"$id":"p","start":2,"end":1}]}}"#;
    assert!(
        check_ko(&fields, valid, CheckOptions::default())
            .constraint_violations
            .is_empty()
    );
    assert_eq!(
        check_ko(&fields, invalid, CheckOptions::default())
            .constraint_violations
            .len(),
        1
    );
    let literal = fields.replace("시작일보다", "0보다");
    let compiled = compile_ko(&literal);
    assert!(compiled.diagnostics.is_empty());
    assert!(matches!(
        compiled.module.unwrap().constraints[0].right,
        ConstraintOperand::Constant(_)
    ));
    assert!(
        check_ko(&literal, invalid, CheckOptions::default())
            .constraint_violations
            .is_empty()
    );
}
