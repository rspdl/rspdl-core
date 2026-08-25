use std::fs;
use std::path::{Path, PathBuf};

use rspdl_compiler::{compile_ko, compile_with_frontend};
use rspdl_domain::ProductionTriggerKind;
use rspdl_domain::{
    FieldProducerSource, Frontend, ProducerPhase, ProductionTriggerDefinition, SurfaceRef,
    UnlinkedFieldProducerSource,
};
use rspdl_ko::KoreanFrontend;

fn source() -> String {
    fs::read_to_string(repository_root().join("conformance/ko-KR/events/normal/input.rspdl"))
        .unwrap()
}

struct KoreanMissingEventLeafFrontend;

impl Frontend for KoreanMissingEventLeafFrontend {
    fn language_id(&self) -> &'static str {
        KoreanFrontend.language_id()
    }

    fn lower_source(&self, source: &str) -> rspdl_domain::FrontendOutput {
        let mut output = KoreanFrontend.lower_source(source);
        let module = output.module.as_mut().expect("Event fixture lowers");
        let producer = module
            .field_producers
            .iter_mut()
            .find(|producer| producer.declaration.id.as_deref() == Some("request_title_binding"))
            .expect("fixture has an Event record-field producer");
        let UnlinkedFieldProducerSource::EventInputField { field, .. } = &mut producer.source
        else {
            panic!("fixture producer must remain EventInputField")
        };
        *field = SurfaceRef::stable_id("missing_field", field.span());
        output
    }
}

#[test]
fn event_conditional_production_conformance() {
    let input = source();
    let output = compile_ko(&input);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    let module = output.module.unwrap();
    let production = &module.conditional_productions[0];
    assert!(production.action_id.is_none());
    assert!(matches!(
        &production.trigger,
        ProductionTriggerDefinition::Event(_)
    ));
    let semantic_json = serde_json::to_string(&module).unwrap();
    assert!(!semantic_json.contains("\"action_id\""));
    assert!(!semantic_json.contains("\"action\""));
    assert!(
        production
            .field_producers
            .iter()
            .all(|producer| { producer.phase == ProducerPhase::TriggerPayload })
    );
    assert!(matches!(
        production
            .field_producers
            .iter()
            .find(|producer| producer.id.as_str() == "event.title_binding")
            .map(|producer| &producer.source),
        Some(FieldProducerSource::EventInput { .. })
    ));
    assert!(matches!(
        production
            .field_producers
            .iter()
            .find(|producer| producer.id.as_str() == "event.request_title_binding")
            .map(|producer| &producer.source),
        Some(FieldProducerSource::EventInputField { .. })
    ));
    assert_eq!(production.relation_producers.len(), 1);
    assert_eq!(
        production.relation_producers[0].phase,
        ProducerPhase::TriggerPayload
    );

    let missing_variant = input.replace(
        "보류 알림 미생성(held_notice_skip)은 점검 요청 접수됨의 요청 상태가 보류됨이면 점검 전달 알림을 생성하지 않는다.\n",
        "",
    );
    let missing_diagnostic = compile_ko(&missing_variant)
        .diagnostics
        .into_iter()
        .find(|d| d.rule_id == "RSPDL-POLICY-008")
        .expect("missing Event enum variant must be diagnosed");
    assert_eq!(missing_diagnostic.argument("trigger_kind"), Some("event"));
    assert_eq!(
        missing_diagnostic.argument("trigger_id"),
        Some("event.request_received")
    );
    assert_eq!(missing_diagnostic.argument("action_id"), None);

    let conflict = input.replace(
        "보류 알림 미생성(held_notice_skip)은 점검 요청 접수됨의 요청 상태가 보류됨이면 점검 전달 알림을 생성하지 않는다.",
        "접수 중복 미생성(received_notice_skip)은 점검 요청 접수됨의 요청 상태가 접수됨이면 점검 전달 알림을 생성하지 않는다.",
    );
    let conflict_diagnostic = compile_ko(&conflict)
        .diagnostics
        .into_iter()
        .find(|d| d.rule_id == "RSPDL-POLICY-007")
        .expect("same Event enum variant must conflict");
    assert_eq!(conflict_diagnostic.argument("trigger_kind"), Some("event"));
    assert_eq!(
        conflict_diagnostic.argument("trigger_id"),
        Some("event.request_received")
    );
    assert_eq!(conflict_diagnostic.argument("action_id"), None);

    let scalar = input.replace(
        "상태를 요청 상태(request_status)로 담는다",
        "문자열을 요청 상태(request_status)로 담는다",
    );
    let scalar_diagnostic = compile_ko(&scalar)
        .diagnostics
        .into_iter()
        .find(|d| d.rule_id == "RSPDL-PROD-002")
        .expect("scalar Event decision must be rejected");
    assert_eq!(scalar_diagnostic.argument("trigger_kind"), Some("event"));
    assert_eq!(
        scalar_diagnostic.argument("trigger_id"),
        Some("event.request_received")
    );
    assert_eq!(scalar_diagnostic.argument("action_id"), None);

    let required = input.replace(
        "알림 내용 조합(content_template)은 점검 요청 접수됨이 발생할 때 \"{제목}: {요청 제목}\"를 점검 전달 알림의 내용으로 조합한다.\n",
        "",
    );
    let required_diagnostic = compile_ko(&required)
        .diagnostics
        .into_iter()
        .find(|d| d.rule_id == "RSPDL-PROD-003")
        .expect("Event Create still requires payload for a required field");
    assert_eq!(required_diagnostic.argument("trigger_kind"), Some("event"));
    assert_eq!(
        required_diagnostic.argument("trigger_id"),
        Some("event.request_received")
    );
    assert_eq!(required_diagnostic.argument("action_id"), None);

    let all_skip = input
        .replace(
            "점검 전달 알림을 하나 생성한다",
            "점검 전달 알림을 생성하지 않는다",
        )
        .replace("내용(content): 필수 문자열", "내용(content): 선택 문자열")
        .replace("제목(title): 필수 문자열", "제목(title): 선택 문자열")
        .replace("요청 제목(request_title): 필수 문자열", "요청 제목(request_title): 선택 문자열")
        .replace(
            "수신자 연결(recipient_binding)은 점검 요청 접수됨이 발생할 때 수신 기술자를 점검 전달 알림의 수신자로 연결한다.\n",
            "",
        );
    assert!(compile_ko(&all_skip).diagnostics.is_empty());

    let same_named_action = input
        .replace(
        "점검 요청 접수됨(request_received)은 사건이다.",
        "점검 요청 접수됨(request_received)은 사건이다.\n점검 요청 접수됨(request_received_action)은 행동이다.",
    )
        .replace(
            "접수 알림 생성(received_notice_create)은 점검 요청 접수됨의 요청 상태가 접수됨이면 점검 전달 알림을 하나 생성한다.\n보류 알림 미생성(held_notice_skip)은 점검 요청 접수됨의 요청 상태가 보류됨이면 점검 전달 알림을 생성하지 않는다.\n",
            "",
        );
    let same_named_output = KoreanFrontend.lower_source(&same_named_action);
    assert!(
        same_named_output.diagnostics.is_empty(),
        "`발생할 때` must select Event even when an Action shares its display name: {:?}",
        same_named_output.diagnostics,
    );
    assert!(
        same_named_output
            .module
            .unwrap()
            .field_producers
            .iter()
            .all(|producer| producer.trigger.kind == ProductionTriggerKind::Event)
    );

    let wrong_owner = input.replace(
        "점검 요청 접수됨이 발생할 때",
        "점검 요청 접수됨이 실행될 때",
    );
    assert!(
        compile_ko(&wrong_owner)
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.rule_id == "RSPDL-KO-REF-001")
    );

    let value_type_mismatch = input.replace(
        "알림 제목을 점검 전달 알림의 제목으로 기록한다.",
        "요청 상태를 점검 전달 알림의 제목으로 기록한다.",
    );
    let type_diagnostic = compile_ko(&value_type_mismatch)
        .diagnostics
        .into_iter()
        .find(|diagnostic| diagnostic.rule_id == "RSPDL-PROD-002")
        .expect("wrong Event value type must be rejected");
    assert_eq!(type_diagnostic.argument("trigger_kind"), Some("event"));
    assert_eq!(type_diagnostic.argument("action_id"), None);

    let relation_type_mismatch = input.replace(
        "수신 기술자를 점검 전달 알림의 수신자로 연결한다.",
        "대상 요청을 점검 전달 알림의 수신자로 연결한다.",
    );
    let relation_diagnostic = compile_ko(&relation_type_mismatch)
        .diagnostics
        .into_iter()
        .find(|diagnostic| diagnostic.rule_id == "RSPDL-PROD-002")
        .expect("wrong Event relation endpoint must be rejected");
    assert_eq!(relation_diagnostic.argument("trigger_kind"), Some("event"));
    assert_eq!(relation_diagnostic.argument("action_id"), None);

    let missing_leaf = compile_with_frontend(&KoreanMissingEventLeafFrontend, &input);
    let leaf_diagnostic = missing_leaf
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.message_key == "semantic.field.not_found")
        .expect("Event ExistingModel field leaf must retain a production diagnostic");
    assert_eq!(leaf_diagnostic.rule_id, "RSPDL-LINK-003");
    assert_eq!(leaf_diagnostic.argument("trigger_kind"), Some("event"));
    assert_eq!(
        leaf_diagnostic.argument("trigger_id"),
        Some("event.request_received")
    );
    assert_eq!(leaf_diagnostic.argument("action_id"), None);
}

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .to_path_buf()
}
