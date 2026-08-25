use std::fs;
use std::path::{Path, PathBuf};

use rspdl_compiler::compile_ko;
use rspdl_domain::{ProducerPhase, RelationSlotCardinality};

type RelationProjection = Vec<(String, String, Vec<(String, String, String)>)>;

#[test]
fn relation_producer_normal_conformance() {
    let source =
        fs::read_to_string(root().join("conformance/ko-KR/relation-producers/normal/input.rspdl"))
            .unwrap();
    let output = compile_ko(&source);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    let production = &output.module.unwrap().conditional_productions[0];
    assert_eq!(production.relation_slots.len(), 1);
    assert_eq!(
        production.relation_slots[0].relation_id.as_str(),
        "relprod.recipient"
    );
    assert_eq!(
        production.relation_slots[0].output_model_id.as_str(),
        "relprod.notice"
    );
    assert_eq!(
        production.relation_slots[0].cardinality,
        RelationSlotCardinality::ExactlyOne
    );
    assert_eq!(production.relation_producers.len(), 1);
    assert_eq!(
        production.relation_producers[0].id.as_str(),
        "relprod.recipient_binding"
    );
    assert_eq!(
        production.relation_producers[0].input_id.as_str(),
        "relprod.assign.recipient_technician"
    );
    assert_eq!(
        production.relation_producers[0].phase,
        ProducerPhase::PreMutation
    );
}

#[test]
fn relation_producer_failure_boundary_and_false_positive_matrix() {
    let normal =
        fs::read_to_string(root().join("conformance/ko-KR/relation-producers/normal/input.rspdl"))
            .unwrap();
    let missing = compile_ko(&normal.replace(
        "수신자 연결(recipient_binding)은 전달이 실행될 때 수신 기술자를 알림의 수신자로 연결한다.\n",
        "",
    ));
    assert_rule(&missing, "RSPDL-PROD-003");
    let duplicate = compile_ko(
        &(normal.replace(
            "수신자 연결(recipient_binding)",
            "수신자 연결(z_recipient_binding)",
        ) + "수신자 연결(a_recipient_binding)은 전달이 실행될 때 수신 기술자를 알림의 수신자로 연결한다.\n"),
    );
    let conflict = duplicate
        .diagnostics
        .iter()
        .find(|d| d.rule_id == "RSPDL-PROD-004")
        .unwrap();
    assert_eq!(
        conflict.argument("producer_ids"),
        Some("relprod.a_recipient_binding,relprod.z_recipient_binding")
    );
    let scalar = compile_ko(&normal.replace(
        "수신 기술자를 알림의 수신자로",
        "요청 상태를 알림의 수신자로",
    ));
    assert_rule(&scalar, "RSPDL-PROD-002");
    let endpoint_mismatch = compile_ko(
        &normal
            .replace(
                "기술자(technician)는 다음 필드들로 구성되어 있다.",
                "감사자(auditor)는 다음 필드들로 구성되어 있다.\n    이름(name): 필수 문자열\n기술자(technician)는 다음 필드들로 구성되어 있다.",
            )
            .replace(
                "전달은 기존 기술자를 수신 기술자(recipient_technician)로 입력받는다.",
                "전달은 기존 기술자를 수신 기술자(recipient_technician)로 입력받는다.\n전달은 기존 감사자를 검토자(auditor_input)로 입력받는다.",
            )
            .replace("수신 기술자를 알림의 수신자로", "검토자를 알림의 수신자로"),
    );
    let mismatch = endpoint_mismatch
        .diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.message_key == "semantic.relation_producer.source_endpoint_mismatch"
        })
        .unwrap();
    assert_eq!(mismatch.rule_id, "RSPDL-PROD-002");
    assert_eq!(
        mismatch.argument("endpoint_model_id"),
        Some("relprod.technician")
    );
    let non_exact = compile_ko(&normal.replace(
        "모든 알림은 수신자를 하나 이상 가져야 한다.\n각 알림은 수신자를 최대 하나만 가질 수 있다.\n",
        "",
    ));
    assert_rule(&non_exact, "RSPDL-PROD-007");
    let all_skip = compile_ko(&normal.replace("알림을 하나 생성한다", "알림을 생성하지 않는다").replace(
        "수신자 연결(recipient_binding)은 전달이 실행될 때 수신 기술자를 알림의 수신자로 연결한다.\n",
        "",
    ));
    assert!(
        all_skip.diagnostics.is_empty(),
        "{:?}",
        all_skip.diagnostics
    );

    let sender_relation = "알림은 기술자를 발신자(sender)로 가질 수 있다.\n모든 알림은 발신자를 하나 이상 가져야 한다.\n각 알림은 발신자를 최대 하나만 가질 수 있다.\n";
    let sender_producer =
        "발신자 연결(sender_binding)은 전달이 실행될 때 수신 기술자를 알림의 발신자로 연결한다.\n";
    let distinct_slots = normal
        .replace(
            "각 알림은 수신자를 최대 하나만 가질 수 있다.\n",
            &format!(
                "각 알림은 수신자를 최대 하나만 가질 수 있다.\n{sender_relation}"
            ),
        )
        .replace(
            "수신자 연결(recipient_binding)은 전달이 실행될 때 수신 기술자를 알림의 수신자로 연결한다.\n",
            &format!(
                "수신자 연결(recipient_binding)은 전달이 실행될 때 수신 기술자를 알림의 수신자로 연결한다.\n{sender_producer}"
            ),
        );
    let reordered = distinct_slots.replace(
        &format!(
            "수신자 연결(recipient_binding)은 전달이 실행될 때 수신 기술자를 알림의 수신자로 연결한다.\n{sender_producer}"
        ),
        &format!(
            "{sender_producer}수신자 연결(recipient_binding)은 전달이 실행될 때 수신 기술자를 알림의 수신자로 연결한다.\n"
        ),
    );
    let distinct_slots = compile_ko(&distinct_slots);
    let reordered = compile_ko(&reordered);
    assert!(
        distinct_slots.diagnostics.is_empty(),
        "{:?}",
        distinct_slots.diagnostics
    );
    assert!(
        reordered.diagnostics.is_empty(),
        "{:?}",
        reordered.diagnostics
    );
    assert_eq!(
        relation_projection(&distinct_slots),
        relation_projection(&reordered)
    );
}

fn relation_projection(output: &rspdl_compiler::Compilation) -> RelationProjection {
    output
        .module
        .as_ref()
        .unwrap()
        .conditional_productions
        .iter()
        .map(|production| {
            (
                production.id.to_string(),
                production
                    .relation_slots
                    .iter()
                    .map(|slot| format!("{}:{}", slot.relation_id, slot.endpoint_model_id))
                    .collect::<Vec<_>>()
                    .join(","),
                production
                    .relation_producers
                    .iter()
                    .map(|producer| {
                        (
                            producer.id.to_string(),
                            producer.relation_id.to_string(),
                            producer.input_id.to_string(),
                        )
                    })
                    .collect(),
            )
        })
        .collect()
}

fn assert_rule(output: &rspdl_compiler::Compilation, rule_id: &str) {
    assert!(
        output
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.rule_id == rule_id),
        "{:?}",
        output.diagnostics
    );
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .to_path_buf()
}
