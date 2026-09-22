use rspdl_compiler::{Compilation, compile_ko};
use rspdl_domain::{HandlerKind, OutcomeDataVerification, OutcomeKind, PolicyVerification};

const NORMAL: &str = r#"---
모듈: 예약(booking)
화면:
  예약 화면:
    역할: [고객]
    권한:
      - 역할: 고객
        행동: 조회
        모델: 예약
        필드: 연락처
    레이아웃:
      - 버튼: { id: lookup, 이름: "조회", 행동: 조회 }
  예약 완료 화면:
    레이아웃:
      - 제목: "완료"
조회 결과:
  reservation_lookup:
    행동: 조회
    입력: 대상 예약
    모델: 예약
    필드: [연락처]
행동 결과:
  조회:
    - id: found
      유형: 성공
      제공 데이터:
        - 모델: 예약
          필드: 연락처
          조회 결과: reservation_lookup
    - id: not_found
      유형: 실패
흐름:
  - id: found_path
    출발: 예약 화면.lookup
    결과: found
    도착: 예약 완료 화면
  - 출발: 예약 화면.lookup
    결과: not_found
    처리:
      종류: 메시지
      id: missing
      내용: "예약을 찾지 못했습니다."
업무:
  예약 완료(complete):
    시작: 예약 화면
    완료:
      - 화면: 예약 완료 화면
        필수 데이터:
          - 모델: 예약
            필드: 연락처
---

예약(reservation)은 다음 필드들로 구성되어 있다.
    연락처(contact): 필수 문자열

고객(customer)은 역할이다.
조회(lookup)는 행동이다.
조회는 기존 예약을 대상 예약(target_reservation)으로 입력받는다.
고객은 예약의 연락처를 조회할 수 있다.
예약 등록 화면(create_screen)에서는 예약을 생성할 수 있다.
예약 등록 화면(create_screen)에서는 예약의 연락처를 입력할 수 있다.
예약 화면(lookup_screen)에서는 예약의 연락처를 조회할 수 있다.
예약 완료 화면(done_screen)에서는 예약의 연락처를 조회할 수 있다.
"#;

const CALCULATION: &str = r#"---
모듈: 견적(quote)
화면:
  계산 화면:
    레이아웃:
      - 버튼: { id: calculate, 이름: "계산", 행동: 계산 }
  완료 화면:
    레이아웃:
      - 제목: "완료"
행동 결과:
  계산:
    - id: calculated
      유형: 성공
      제공 데이터:
        - 모델: 견적
          필드: 총액
          계산 대상: total
흐름:
  - 출발: 계산 화면.calculate
    결과: calculated
    도착: 완료 화면
업무:
  견적 완료(complete):
    시작: 계산 화면
    완료:
      - 화면: 완료 화면
        필수 데이터:
          - 모델: 견적
            필드: 총액
---

견적(quote)은 다음 필드들로 구성되어 있다.
    총액(total): 필수 정수
항목(item)은 다음 필드들로 구성되어 있다.
    금액(amount): 필수 정수
계산(calculate)은 행동이다.
견적 생성 화면(create_quote)에서는 견적을 생성할 수 있다.
항목 생성 화면(create_item)에서는 항목을 생성할 수 있다.
항목 생성 화면(create_item)에서는 항목의 금액을 입력할 수 있다.
계산 화면(calculate_screen)에서는 견적의 총액을 조회할 수 있다.
완료 화면(done_screen)에서는 견적의 총액을 조회할 수 있다.
견적의 총액은 항목의 금액의 합계로 계산한다.
항목의 금액이 바뀔 때 견적의 총액을 다시 계산한다.
"#;

const GENERATION: &str = r#"---
모듈: 생산(production)
화면:
  시작 화면:
    레이아웃:
      - 버튼: { id: assign, 이름: "전달", 행동: 전달 }
  완료 화면:
    레이아웃:
      - 제목: "완료"
행동 결과:
  전달:
    - id: assigned
      유형: 성공
      제공 데이터:
        - 모델: 알림
          필드: 재시도 횟수
          생산자: retry_binding
흐름:
  - 출발: 시작 화면.assign
    결과: assigned
    도착: 완료 화면
업무:
  전달 완료(complete):
    시작: 시작 화면
    완료:
      - 화면: 완료 화면
        필수 데이터:
          - 모델: 알림
            필드: 재시도 횟수
---
상태(status)는 다음 값 중 하나다.
    접수됨(received)
    보류됨(held)
알림(notice)은 다음 필드들로 구성되어 있다.
    재시도 횟수(retry_count): 선택 정수
전달(assign)은 행동이다.
전달은 상태를 요청 상태(request_status)로 입력받는다.
접수 생성(received_create)은 전달의 요청 상태가 접수됨이면 알림을 하나 생성한다.
보류 생성(held_create)은 전달의 요청 상태가 보류됨이면 알림을 하나 생성한다.
재시도 횟수 기록(retry_binding)은 전달이 실행될 때 상수 0을 알림의 재시도 횟수로 기록한다.
시작 화면(start)에서는 알림의 재시도 횟수를 조회할 수 있다.
완료 화면(done)에서는 알림의 재시도 횟수를 조회할 수 있다.
알림 생성 화면(create_notice)에서는 알림을 생성할 수 있다.
알림 생성 화면(create_notice)에서는 알림의 재시도 횟수를 입력할 수 있다.
"#;

fn compile(source: &str) -> Compilation {
    compile_ko(source)
}
fn strip_spans<T: serde::Serialize>(value: &T) -> serde_json::Value {
    fn strip(v: &mut serde_json::Value) {
        match v {
            serde_json::Value::Object(o) => {
                o.remove("span");
                o.values_mut().for_each(strip)
            }
            serde_json::Value::Array(a) => a.iter_mut().for_each(strip),
            _ => {}
        }
    }
    let mut v = serde_json::to_value(value).unwrap();
    strip(&mut v);
    v
}

#[test]
fn declaration_order_does_not_change_outcome_or_lookup_semantic_sets() {
    let reordered = NORMAL.replace(
        r#"    - id: found
      유형: 성공
      제공 데이터:
        - 모델: 예약
          필드: 연락처
          조회 결과: reservation_lookup
    - id: not_found
      유형: 실패"#,
        r#"    - id: not_found
      유형: 실패
    - id: found
      유형: 성공
      제공 데이터:
        - 모델: 예약
          필드: 연락처
          조회 결과: reservation_lookup"#,
    );
    assert_ne!(reordered, NORMAL);

    let original = compile(NORMAL).module.unwrap();
    let reordered = compile(&reordered).module.unwrap();
    assert_eq!(
        strip_spans(&original.action_outcomes),
        strip_spans(&reordered.action_outcomes)
    );
    assert_eq!(
        strip_spans(&original.lookup_results),
        strip_spans(&reordered.lookup_results)
    );
}

#[test]
fn typed_outcomes_link_exact_handlers_policy_and_lookup_data() {
    let result = compile(NORMAL);
    assert!(
        !result.diagnostics.iter().any(|d| d.is_error()),
        "{:#?}",
        result.diagnostics
    );
    let module = result.module.unwrap();
    assert_eq!(module.lookup_results.len(), 1);
    assert_eq!(module.action_outcomes.len(), 2);
    assert_eq!(module.action_outcomes[0].kind, OutcomeKind::Success);
    assert_eq!(
        module.action_outcomes[0].provided_data[0].verification,
        OutcomeDataVerification::Verified
    );
    assert_eq!(
        module.screen_layouts[0].permissions[0].verification,
        PolicyVerification::Allowed
    );
    assert_eq!(
        module.screen_paths[1].handler.as_ref().unwrap().kind,
        HandlerKind::Message
    );
    assert_eq!(
        module.screen_paths[1]
            .handler
            .as_ref()
            .unwrap()
            .content
            .as_deref(),
        Some("예약을 찾지 못했습니다.")
    );
    assert!(
        !result
            .diagnostics
            .iter()
            .any(|d| d.message_key == "semantic.workflow.required_data_unavailable")
    );
}

#[test]
fn normal_lookup_conformance_fixture_matches_case_contract() {
    let source =
        include_str!("../../../conformance/ko-KR/action-outcomes/normal-lookup/input.rspdl");
    let result = compile(source);
    assert!(result.module.is_some(), "{:#?}", result.diagnostics);
    assert_eq!(
        result
            .diagnostics
            .iter()
            .map(|d| d.message_key.as_str())
            .collect::<Vec<_>>(),
        vec!["semantic.layout.required_input_without_slot"]
    );
}

#[test]
fn outcome_frontmatter_round_trips_without_losing_authored_contracts() {
    let parsed = rspdl_ko::parse(NORMAL);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let formatted = rspdl_ko::format_document(&parsed.document.unwrap()).unwrap();
    let output = rspdl_ko::format_source(&formatted);
    assert!(
        output.text.is_some(),
        "{}\n{:#?}",
        formatted,
        output.diagnostics
    );
    let second = output.text.unwrap();
    assert_eq!(formatted, second);
    assert_eq!(
        strip_spans(&compile(NORMAL).module),
        strip_spans(&compile(&formatted).module)
    );
}

#[test]
fn every_originating_button_needs_each_outcome_handler() {
    let source=NORMAL.replace("  - 출발: 예약 화면.lookup\n    결과: not_found\n    처리:\n      종류: 메시지\n      id: missing\n      내용: \"예약을 찾지 못했습니다.\"\n","");
    let result = compile(&source);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.message_key == "semantic.outcome.handler_missing")
    );
}

#[test]
fn input_capability_without_adopted_lookup_result_does_not_acquire_data() {
    let source=NORMAL.replace("      제공 데이터:\n        - 모델: 예약\n          필드: 연락처\n          조회 결과: reservation_lookup\n","");
    let result = compile(&source);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.message_key == "semantic.workflow.required_data_unavailable")
    );
}

#[test]
fn unrelated_resource_deny_does_not_override_exact_permission_scope() {
    let source=NORMAL.replace("예약(reservation)은 다음 필드들로 구성되어 있다.","기록(log)은 다음 필드들로 구성되어 있다.\n    연락처(contact): 필수 문자열\n\n예약(reservation)은 다음 필드들로 구성되어 있다.")
        .replace("고객은 예약의 연락처를 조회할 수 있다.","고객은 예약의 연락처를 조회할 수 있다.\n고객은 기록의 연락처를 조회할 수 없다.");
    let result = compile(&source);
    assert!(
        !result
            .diagnostics
            .iter()
            .any(|d| d.message_key == "semantic.screen_permission.denied"),
        "{:#?}",
        result.diagnostics
    );
}

#[test]
fn optional_lookup_data_is_unknown_not_verified_availability() {
    let source = NORMAL.replace(
        "연락처(contact): 필수 문자열",
        "연락처(contact): 선택 문자열",
    );
    let result = compile(&source);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.message_key == "semantic.outcome.optional_data_unknown")
    );
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.message_key == "semantic.workflow.verification_unknown")
    );
    assert!(
        !result
            .diagnostics
            .iter()
            .any(|d| d.message_key == "semantic.workflow.required_data_unavailable")
    );
}

#[test]
fn the_same_action_on_another_button_does_not_share_handlers() {
    let source=NORMAL.replace("      - 버튼: { id: lookup, 이름: \"조회\", 행동: 조회 }","      - 버튼: { id: lookup, 이름: \"조회\", 행동: 조회 }\n      - 버튼: { id: lookup_again, 이름: \"다시 조회\", 행동: 조회 }");
    let result = compile(&source);
    let missing = result
        .diagnostics
        .iter()
        .filter(|d| d.message_key == "semantic.outcome.handler_missing")
        .count();
    assert_eq!(missing, 2, "{:#?}", result.diagnostics);
}

#[test]
fn failure_outcome_cannot_claim_success_data() {
    let source=NORMAL.replace("    - id: not_found\n      유형: 실패","    - id: not_found\n      유형: 실패\n      제공 데이터:\n        - 모델: 예약\n          필드: 연락처\n          조회 결과: reservation_lookup");
    let result = compile(&source);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.message_key == "semantic.outcome.non_success_provides_data")
    );
}

#[test]
fn retry_target_must_be_reachable_from_each_specific_handler() {
    let source=NORMAL
        .replace("  예약 완료 화면:\n    레이아웃:","  다른 화면:\n    레이아웃:\n      - 버튼: { id: retry, 이름: \"재시도\", 행동: 조회 }\n  예약 완료 화면:\n    레이아웃:")
        .replace("    - id: not_found\n      유형: 실패","    - id: not_found\n      유형: 실패\n      복구: { 종류: retry, 화면: 다른 화면, 요소: retry, 행동: 조회 }")
        .replace("예약 완료 화면(done_screen)에서는 예약의 연락처를 조회할 수 있다.","다른 화면(other_screen)에서는 예약의 연락처를 조회할 수 있다.\n예약 완료 화면(done_screen)에서는 예약의 연락처를 조회할 수 있다.");
    let result = compile(&source);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.message_key == "semantic.recovery.target_invalid"),
        "{:#?}",
        result.diagnostics
    );
}

#[test]
fn permission_field_typo_is_an_error_not_model_scope_permission() {
    let source = NORMAL.replace(
        "        필드: 연락처\n    레이아웃:",
        "        필드: 없는 필드\n    레이아웃:",
    );
    let result = compile(&source);
    assert!(result.module.is_none());
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.message_key == "ko.reference.not_found")
    );
}

#[test]
fn disconnected_unknown_outcome_does_not_mask_reachable_missing_data() {
    let source=NORMAL
        .replace("      제공 데이터:\n        - 모델: 예약\n          필드: 연락처\n          조회 결과: reservation_lookup\n","")
        .replace("연락처(contact): 필수 문자열","연락처(contact): 필수 문자열\n    메모(note): 선택 문자열")
        .replace("  예약 완료 화면:\n    레이아웃:","  고립 화면:\n    레이아웃:\n      - 버튼: { id: note, 이름: \"메모 조회\", 행동: 메모 조회 }\n  예약 완료 화면:\n    레이아웃:")
        .replace("행동 결과:\n","  note_lookup:\n    행동: 메모 조회\n    입력: 대상 예약\n    모델: 예약\n    필드: [메모]\n행동 결과:\n")
        .replace("흐름:\n","  메모 조회:\n    - id: noted\n      유형: 성공\n      제공 데이터:\n        - 모델: 예약\n          필드: 메모\n          조회 결과: note_lookup\n흐름:\n")
        .replace("업무:\n","  - 출발: 고립 화면.note\n    결과: noted\n    처리:\n      종류: 상태\n      id: shown\n업무:\n")
        .replace("조회(lookup)는 행동이다.","조회(lookup)는 행동이다.\n메모 조회(note_lookup_action)는 행동이다.\n메모 조회는 기존 예약을 대상 예약(target_note_reservation)으로 입력받는다.")
        .replace("예약 완료 화면(done_screen)에서는 예약의 연락처를 조회할 수 있다.","고립 화면(isolated_screen)에서는 예약의 메모를 조회할 수 있다.\n예약 완료 화면(done_screen)에서는 예약의 연락처를 조회할 수 있다.");
    let result = compile(&source);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.message_key == "semantic.workflow.required_data_unavailable"),
        "{:#?}",
        result.diagnostics
    );
}

#[test]
fn calculation_outcome_requires_its_derivation_source_on_the_path() {
    let missing = compile(CALCULATION);
    assert!(
        missing
            .diagnostics
            .iter()
            .any(|d| d.message_key == "semantic.workflow.required_data_unavailable"),
        "{:#?}",
        missing.diagnostics
    );
    let with_source = CALCULATION.replace(
        "    시작: 계산 화면\n",
        "    시작: 계산 화면\n    초기 데이터:\n      - 모델: 항목\n        필드: 금액\n",
    );
    let normal = compile(&with_source);
    assert!(
        !normal
            .diagnostics
            .iter()
            .any(|d| d.message_key == "semantic.workflow.required_data_unavailable"),
        "{:#?}",
        normal.diagnostics
    );
}

#[test]
fn unconditional_constant_generation_can_prove_success_data() {
    let result = compile(GENERATION);
    assert!(
        !result.diagnostics.iter().any(|d| d.message_key
            == "semantic.workflow.required_data_unavailable"
            || d.message_key == "semantic.workflow.verification_unknown"),
        "{:#?}",
        result.diagnostics
    );
    let module = result
        .module
        .unwrap_or_else(|| panic!("{:#?}", result.diagnostics));
    assert_eq!(
        module.action_outcomes[0].provided_data[0].verification,
        OutcomeDataVerification::Verified
    );
}

#[test]
fn conditional_generation_coverage_is_unknown_not_verified() {
    let source = GENERATION.replace(
        "보류 생성(held_create)은 전달의 요청 상태가 보류됨이면 알림을 하나 생성한다.",
        "보류 미생성(held_skip)은 전달의 요청 상태가 보류됨이면 알림을 생성하지 않는다.",
    );
    let result = compile(&source);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.message_key == "semantic.outcome.producer_coverage_unknown")
    );
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.message_key == "semantic.workflow.verification_unknown")
    );
    assert!(
        !result
            .diagnostics
            .iter()
            .any(|d| d.message_key == "semantic.workflow.required_data_unavailable")
    );
}
