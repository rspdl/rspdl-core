use rspdl_compiler::{CheckOptions, check_ko, compile_ko};
use rspdl_domain::CanonicalType;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

const SOURCE: &str = r#"@모듈 값(values)
결제(payment)는 다음 필드들로 구성되어 있다.
    금액(amount): 필수 통화(KRW)
    할인율(discount): 필수 백분율
배송(shipment)은 다음 필드들로 구성되어 있다.
    무게(weight): 필수 수량(kg)
    위치(location): 필수 좌표
고객(customer)은 다음 필드들로 구성되어 있다.
    ID(id): 필수 UUID
    태그(tags): 필수 집합(문자열)
    결제(payment): 선택 참조(payment)
결제의 금액은 "10000 KRW" 이상이어야 한다.
배송의 무게는 "20 kg" 이하여야 한다.
"#;

#[test]
fn korean_parameterized_types_reach_canonical_ir_and_runtime_validation() {
    let compilation = compile_ko(SOURCE);
    assert!(
        compilation.diagnostics.is_empty(),
        "{:?}",
        compilation.diagnostics
    );
    let module = compilation.module.unwrap();
    let fields = module
        .models
        .iter()
        .flat_map(|model| model.fields.iter())
        .collect::<Vec<_>>();
    assert!(
        fields
            .iter()
            .any(|field| matches!(field.value_type, CanonicalType::Money(_)))
    );
    assert!(
        fields
            .iter()
            .any(|field| matches!(field.value_type, CanonicalType::Quantity(_)))
    );
    assert!(
        fields
            .iter()
            .any(|field| matches!(field.value_type, CanonicalType::Set(_)))
    );
    assert!(
        fields
            .iter()
            .any(|field| matches!(field.value_type, CanonicalType::Reference(_)))
    );

    let valid = r#"{"records":{"values.payment":[{"$id":"p","amount":"10000 KRW","discount":"15%"}],"values.shipment":[{"$id":"s","weight":"20 kg","location":"37.5,127"}],"values.customer":[{"$id":"c","id":"550e8400-e29b-41d4-a716-446655440000","tags":["a","b"]}]}}"#;
    assert!(!check_ko(SOURCE, valid, CheckOptions::default()).has_errors());

    let invalid = valid
        .replace("10000 KRW", "10000 USD")
        .replace("[\"a\",\"b\"]", "[\"a\",\"a\"]");
    let report = check_ko(SOURCE, &invalid, CheckOptions::default());
    assert!(report.has_errors());
    assert_eq!(report.runtime_diagnostics.len(), 2);
}

#[test]
fn runtime_rejects_dst_offset_mismatch_without_guessing_an_instant() {
    let source = "@모듈 예약(reservation)\n예약(reservation)은 다음 필드들로 구성되어 있다.\n    시작(starts_at): 필수 시간대 날짜시간\n";
    let input = r#"{"records":{"reservation.reservation":[{"$id":"r","starts_at":"2026-03-08T02:30:00-05:00 America/New_York"}]}}"#;
    let report = check_ko(source, input, CheckOptions::default());
    assert!(report.has_errors());
    assert_eq!(report.runtime_diagnostics.len(), 1);
}

#[derive(Deserialize)]
struct ConformanceCase {
    expected_module: bool,
    expected_compile_diagnostics: usize,
    expected_runtime_diagnostics: usize,
    expected_constraint_violations: usize,
    #[serde(default)]
    expected_compile_diagnostic_evidence: Vec<DiagnosticEvidence>,
    #[serde(default)]
    expected_runtime_diagnostic_evidence: Vec<DiagnosticEvidence>,
}
#[derive(Debug, Deserialize, PartialEq)]
struct DiagnosticEvidence {
    rule_id: String,
    message_key: String,
}

#[test]
fn typed_product_value_conformance_cases_are_deterministic() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .unwrap()
        .join("conformance/ko-KR/typed-product-values");
    let mut cases = fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    cases.sort();
    for case in cases {
        let expected: ConformanceCase =
            serde_json::from_str(&fs::read_to_string(case.join("case.json")).unwrap()).unwrap();
        let input = fs::read_to_string(case.join("input.rspdl")).unwrap();
        let compilation = compile_ko(&input);
        assert_eq!(
            compilation.module.is_some(),
            expected.expected_module,
            "{}",
            case.display()
        );
        assert_eq!(
            compilation
                .diagnostics
                .iter()
                .map(|diagnostic| DiagnosticEvidence {
                    rule_id: diagnostic.rule_id.clone(),
                    message_key: diagnostic.message_key.clone()
                })
                .collect::<Vec<_>>(),
            expected.expected_compile_diagnostic_evidence,
            "{}",
            case.display()
        );
        assert_eq!(
            compilation.diagnostics.len(),
            expected.expected_compile_diagnostics,
            "{}",
            case.display()
        );
        if let Ok(data) = fs::read_to_string(case.join("data.json")) {
            let report = check_ko(&input, &data, CheckOptions::default());
            assert_eq!(
                report.runtime_diagnostics.len(),
                expected.expected_runtime_diagnostics,
                "{}",
                case.display()
            );
            assert_eq!(
                report
                    .runtime_diagnostics
                    .iter()
                    .map(|diagnostic| DiagnosticEvidence {
                        rule_id: diagnostic.rule_id.clone(),
                        message_key: diagnostic.message_key.clone()
                    })
                    .collect::<Vec<_>>(),
                expected.expected_runtime_diagnostic_evidence,
                "{}",
                case.display()
            );
            assert_eq!(
                report.constraint_violations.len(),
                expected.expected_constraint_violations,
                "{}",
                case.display()
            );
        }
    }
}
