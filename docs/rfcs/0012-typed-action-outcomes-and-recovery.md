---
id: typed-action-outcomes-and-recovery
title: 행동 결과, 역할 연결과 제한된 복구 의무
type: rfc
status: implemented
version: "1"
summary: Defines typed action outcomes, explicit screen-role bindings, outcome-specific data provenance, and bounded recovery obligations without inferring business policy.
topics:
  - action-outcome
  - screen-role
  - workflow-data
  - recovery
  - timeout
related:
  - stable-screen-elements-and-structured-editing
  - workflow-completion-data-availability
  - conditional-data-production
  - frontend-semantic-analysis-contract
problem_refs:
  - screen-structure-spec-divergence
  - data-lifecycle-modeling-gap
  - policy-consistency-blind-spots
last_updated: "2026-09-23"
owners:
  - rspdl-maintainers
target_spec: "0.8.0"
---

# 행동 결과, 역할 연결과 제한된 복구 의무

## 실패 시나리오와 경계

버튼의 action만 알아서는 성공, 거절, 취소, timeout을 어느 화면 상태가 처리하는지 알 수 없다. 경로의
설명 문자열을 조건으로 읽거나 `고객 모바일` 같은 환경 이름에서 역할을 추측하면 표시 문구가 실행 의미가
된다. 이 slice는 사용자가 명시한 outcome, 역할, 데이터 provenance와 복구 의무만 검사한다.

기존 outcome 없는 경로는 계속 컴파일하지만 outcome 처리 보장은 `unverified`다. timeout duration, 결제 취소,
좌석 해제 같은 실제 의무는 선언 없이 만들지 않는다.

## 제안 표면 계약

화면 layout은 역할을 명시할 수 있다. 환경 이름은 역할 사실이 아니다.

```rspdl
화면:
  예약 화면:
    역할: [고객]
    권한:
      - { 역할: 고객, 행동: 예약 요청, 모델: 예약, 필드: 상태 }
    레이아웃:
      - 버튼: { id: submit, 이름: "예약", 행동: 예약 요청 }

행동 결과:
  예약 요청:
    - { id: confirmed, 유형: 성공 }
    - id: rejected
      유형: 실패
      복구: { 종류: retry, 화면: 예약 화면, 요소: retry, 행동: 예약 요청 }
    - id: timed_out
      유형: timeout
      복구: { 종류: return, 경로: catalog.return_to_search }
```

flow edge는 `결과`로 outcome ID를 연결한다. 같은 화면 처리는 목적 화면 대신 `처리`를 사용한다.

```rspdl
흐름:
  - id: catalog.confirmed_path
    출발: 예약 화면.submit
    결과: confirmed
    도착: 예약 완료 화면
  - 출발: 예약 화면.submit
    결과: rejected
    처리: { 종류: 메시지, id: rejected_message }
  - 출발: 예약 화면.submit
    결과: timed_out
    처리: { 종류: 팝업, id: timeout_popup }
```

`처리`는 optional `내용`을 가질 수 있다. 이 값은 사용자가 쓴 표시 문구이며 compiler는 실행 조건이나
정책으로 해석하지 않는다. `내용`이 없으면 compiler나 소비자가 문구를 만들어 채우지 않는다.

복구의 `화면+요소+행동`은 handler에서 도달 가능한 정확한 버튼과 그 action을 함께 묶는다. `경로`는
새 optional flow stable ID를 가리키며 그 경로의 source가 handler에서 도달 가능해야 한다. `release`는
`{ 종류: release, 화면, 요소, 행동 }` 형태이고 action의 기존 mutation/producer 의미를 검사한다.

처리 종류는 첫 slice에서 `상태`, `메시지`, `팝업`, `로딩`의 닫힌 집합이다. label/설명은 표시 전용이다.
각 originating button과 outcome 쌍은 정확히 하나의 도착 또는 같은 화면 처리를 가져야 한다. 같은 action을
쓰는 다른 버튼의 처리는 공유 증거가 아니다. 중복·조건부 대안이 있으면 definitive missing으로 만들지 않고
ambiguity 또는 unknown을 낸다.

## 데이터 생산과 복구

성공 outcome은 `제공 데이터`를 선언할 수 있다. 각 field는 다음 중 하나의 compiler-owned source를 가진다.

```rspdl
조회 결과:
  reservation_lookup:
    행동: 예약 조회
    입력: target_reservation
    모델: 예약
    필드: [연락처]

행동 결과:
  예약 조회:
    - id: found
      유형: 성공
      제공 데이터:
        - { 모델: 예약, 필드: 연락처, 조회 결과: reservation_lookup }
  견적 계산:
    - id: calculated
      유형: 성공
      제공 데이터:
        - { 모델: 견적, 필드: 총액, 계산 대상: quote.total }
  예약 생성:
    - id: created
      유형: 성공
      제공 데이터:
        - { 모델: 예약, 필드: 상태, 생산자: booking_status_from_request }
```

`조회 결과` ID는 새 source-backed declaration이다. 같은 action의 existing-model input, 그 input model과
field를 정확히 연결하며 action 실행 성공을 보장하지 않는다. outcome이 이 ID를 채택할 때만 acquisition이
된다. `계산 대상`은 canonical target field ID이고, 정확히 하나의 기존 derivation이 그 field를 생산해야 한다.
`생산자`는 기존 `FieldProducerDefinition.id`다. producer trigger action이 outcome owner action과 같아야 하며
conditional producer는 해당 outcome coverage가 증명되지 않으면 unknown이다.

- 같은 action의 명시적 READ result와 model/field가 일치하는 lookup snapshot
- 선언된 derivation target에 연결된 calculation result
- 기존 action/event field producer 또는 creation producer에 연결된 generated result

화면 READ 문장, input 배치, outcome 이름만으로 availability를 만들지 않는다. initial workflow data는 계속
외부 precondition일 뿐 producer가 아니다. 성공 outcome의 source가 기존 typed IR과 owner/type까지 일치할 때만
C2 fixed point에 acquisition을 전달한다. 실패·취소·timeout outcome은 별도 명시 없이는 데이터를 얻지 않는다.
조건부 producer가 영향을 주고 coverage를 증명할 수 없으면 unknown이며 unavailable 오류를 함께 단정하지 않는다.

`제공 데이터`는 provenance만 존재한다는 뜻이 아니다. workflow availability로 전달하려면 그 source의
필수 입력도 해당 경로에서 확보되어야 한다. 합계 계산은 derivation source field가 먼저 available해야 하고,
field producer는 상수처럼 전제조건 없는 source이거나 명시된 input-field 전제조건이 available해야 한다.
선택 READ field는 성공 outcome만으로 값 존재를 보장하지 않으므로 `unknown`이다. 이 slice는 null 여부나
임의 value validation을 simulation하지 않는다.

필수 데이터 witness 탐색은 그 필드와 transitive prerequisite만 상태에 남기고, 같은 화면의 더 강한
available/unknown 상태를 제거한다. 그래도 단일 필드의 관련 상태가 16,384개를 넘으면 분석기는 임의로
누락을 단정하지 않고 resource-bound `unknown`을 반환한다.

`retry`, `return`, `release` 복구 의무는 정확한 source button/outcome에 귀속된다. handler에서 도달 가능한
명시적 action/path가 해당 의무를 수행해야 한다. 다른 화면의 같은 이름 버튼은 증거가 아니다. timeout은
명시 outcome으로 지원하지만 duration은 이 계약에 없다.

첫 slice가 `release`에 대해 증명하는 것은 handler에서 release action을 제공하는 정확한 버튼까지 도달할 수
있고 그 action에 기존 mutation 또는 production 의미가 있다는 사실까지다. 사용자가 그 버튼을 반드시 누르는지,
release가 실제로 실행되었는지와 idempotent한지는 `unknown`이며 이 계약의 완료 보장으로 주장하지 않는다.

## 역할과 정책

screen role과 button action을 기존 role/action/policy IR에 연결한다. 무조건 policy 또는 기존 core가 coverage를
명확히 증명하는 조건 공간에서만 allow/deny를 확정한다. 관련 conditional policy가 불완전하거나 backend가
지원하지 않으면 unknown이다. policy 부재의 의미는 기존 core 계약을 따르며 UI가 default allow/deny를 만들지
않는다. `권한` binding은 role/action/model과 optional field를 모두 명시한다. analyzer는 같은 resource tuple의
policy만 비교하며 같은 action을 언급해도 다른 model/field의 deny를 이 화면의 deny로 쓰지 않는다.

## 검증 행렬

- 정상: 성공은 다음 화면, 실패는 같은 화면 메시지, timeout은 팝업과 retry action으로 처리한다.
- 실패: 선언된 failure outcome에 originating button handler가 없다.
- 실패: timeout retry가 다른 화면의 무관한 retry 버튼만 가리킨다.
- 실패: recovery가 존재하지 않는 flow ID 또는 다른 action의 버튼을 가리킨다.
- 실패: READ result의 action/input/model/field가 서로 맞지 않는다.
- 실패: calculation target에 derivation이 없거나 둘 이상이고, generation producer owner가 다르다.
- 실패: 설명에 `성공`만 쓰고 outcome 연결을 생략한다.
- 경계: legacy edge는 유지되며 outcome 검증 상태만 unverified다.
- 경계: conditional producer/policy coverage가 불명확하면 Korean structured unknown을 반환한다.
- 반례: input field 또는 READ capability만으로 lookup acquisition을 주장하지 못한다.
- 반례: 환경 이름에 `고객`이 있어도 screen role binding이 되지 않는다.

## 비범위

임의 조건 문장 해석, timeout 시간 계산, 외부 시스템 성공 보장, 정책 simulation, 보상 transaction 자동 생성,
실시간 다중 역할 상태 동기화, 여러 문서에 나뉜 공통 모델 참조 연결은 포함하지 않는다.
