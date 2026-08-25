use std::fs;
use std::path::{Path, PathBuf};

use rspdl_compiler::compile_ko;
use rspdl_domain::ProductionTriggerDefinition;

fn source() -> String {
    fs::read_to_string(repository_root().join("conformance/ko-KR/events/normal/input.rspdl"))
        .unwrap()
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

    let required = input.replace("내용(content): 선택 문자열", "내용(content): 필수 문자열");
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
        .replace("내용(content): 선택 문자열", "내용(content): 필수 문자열");
    assert!(compile_ko(&all_skip).diagnostics.is_empty());
}

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .to_path_buf()
}
