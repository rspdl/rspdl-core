---
id: conditional-data-production
title: Conditional Data Production for Notifications and Prices
type: rfc
status: proposed
version: "0.1"
summary: Defines Korean sentence-shaped conditional outputs with typed provenance, lifecycle availability, and condition-space analysis.
topics:
  - conditional-production
  - notification
  - pricing
  - field-provenance
  - lifecycle
  - policy-analysis
  - canonical-ir
related:
  - field-provenance-and-sum-derivation
  - total-policy-condition-space-analysis
  - natural-korean-domain-grammar
  - finite-relational-model-finding
  - frontend-semantic-analysis-contract
  - rspdl-language-prd
problem_refs:
  - data-lifecycle-modeling-gap
  - policy-consistency-blind-spots
  - semantic-source-provenance-loss
last_updated: "2026-08-25"
owners:
  - rspdl-maintainers
target_spec: "0.4.0"
---

# 조건부 데이터 생산: 알림과 가격

## 상태와 목적

이 RFC는 권한의 `allow` 또는 `deny`를 임의의 effect 목록으로 넓히지 않는다. 행동 또는 명시된 사건의 조건에 따라 typed output record를 생성하는 **조건부 데이터 생산**을 정의한다. 알림, 가격, 청구 예정, 감사 기록처럼 구현자가 실제 값을 전달받아야 하는 결과는 이 모델을 사용한다.

- 메시지 template은 원본 모델 경로나 행동 입력을 직접 참조하지 않고 output record의 선언된 field만 placeholder로 쓴다.
- output field 값은 선언된 행동 input, relation path, snapshot, constant 또는 지원되는 expression에서만 온다.
- 필수 output field와 relation slot은 모든 effective creation path에서 정확히 하나의 compatible producer를 가져야 한다. 0개는 gap이고 양립 불가능한 복수 producer는 conflict다.
- 삭제 전 source는 explicit snapshot 또는 retain 없이는 post-delete payload에 쓸 수 없다.
- 가격은 별도 pricing effect가 아니라 금액·통화·할인 field를 가진 output record의 derivation, coverage와 composition이다.

이것은 `data-lifecycle-modeling-gap`과 `policy-consistency-blind-spots`의 공동 결과이며, output binding의 원문 근거를 보존해야 하므로 `semantic-source-provenance-loss`도 함께 연결한다. 전자만 해결하면 모든 branch에서 payload가 완성되는지 알 수 없고, 후자만 해결하면 input으로 받지 않은 값이나 삭제된 값을 payload에 넣을 수 있다.

## 실패 시나리오

시설 점검 요청이 접수되면 담당 기술자에게 제목을 포함한 알림을 보내려 한다. 요청 제목의 원천, 기술자 cardinality와 요청을 삭제한 뒤에도 제목을 보낼지 쓰지 않으면 개발자는 조회·빈 문자열·복사 중 하나를 추측해야 한다.

대관 견적도 기본 금액, 장비 추가 금액과 할인 금액을 조건별로 정한 뒤 최종 금액을 만든다. 어떤 branch가 각 금액을 생산하는지와 값의 근거가 없으면 같은 기획에서 다른 청구액이 나온다.

## 가장 작은 vertical slice

첫 slice는 한 action invocation의 pre-state에서 하나의 output record를 생성한다. field producer는 typed action input, input record의 field, constant 및 명시적 pre-state snapshot만 지원한다. 목표 의미의 trigger는 `Action`과 명시적으로 선언된 `Event`를 구분하지만 첫 slice는 `Action`만 lower한다.

2026-08-25 현재 구현된 범위는 stable-ID typed action input, direct enum conditional creation decision과 무조건적 field producer다. Korean frontend와 common analyzer는 action+output production을 만들고 ExactlyOne `Create`/`Skip`, enum coverage, same-variant conflict 및 Create path의 required output field producer gap/conflict를 검사한다. producer는 direct Value input, ExistingModel input field 또는 explicit scalar constant를 action mutation 전(`PreMutation`)에 모든 Create branch에 똑같이 적용한다. conditional field producer, relation producer, snapshot, template은 아직 구현하지 않았다.

- action은 stable-ID typed input을 선언한다. existing record input은 action 직전에 존재해야 한다.
- output record는 하나 이상의 typed field를 가진다. output field와 output relation slot은 target과 producer span을 가진 binding으로만 채운다.
- 조건은 단일 닫힌 enum input의 independent branch이고, branch는 `생성` 또는 명시적 `생성하지 않음`을 선택한다.
- 단일 발신자·수신자·대상은 typed output relation slot으로 선언하고 action input에서 직접 바인딩한다. relation fan-out과 runtime relation join은 지원하지 않는다.
- snapshot은 mutation 전 input record field 값을 capture한다. deletion 뒤 원본을 읽는 것이 아니라 capture된 값을 output에 materialize한다.
- template placeholder는 output field ID만 보존하고 type을 검사한다.

일반 산술, 반올림·세금·환율·외부 가격표, 다수 대상 fan-out·deduplication, 실제 relation JSON binding, delivery retry·UI navigation·external side effect, source-order priority는 지원하지 않는다. lower할 수 없는 의미는 empty value나 success가 아니라 `unsupported` 또는 `unknown`이다.

## 구현된 Korean 정규 문형

다음 두 문형만 현재 parser가 lower한다. branch는 stable ID, action, direct action input, enum variant, output model과 결정을 한 문장에 모두 쓴다. `하나 생성한다`는 ExactlyOne `Create`, `생성하지 않는다`는 `Skip`이다. variant marker는 받침이면 `이면`, 받침이 없으면 `라면`을 쓴다.

```rspdl
접수 상태 알림 생성(received_notice_create)은 점검 요청 전달의 요청 상태가 접수됨이면 점검 요청 전달 알림을 하나 생성한다.
보류 상태 알림 미생성(on_hold_notice_skip)은 점검 요청 전달의 요청 상태가 보류됨이면 점검 요청 전달 알림을 생성하지 않는다.
```

annotation과 block은 허용하지 않으며, source order는 priority가 아니다. 아래 비정규 예시는 설계 목표를 설명할 뿐 현재 정규 문법이 아니다.

### 구현된 무조건적 field producer 문형

아래 세 문형은 같은 action+output production에만 붙는 `PreMutation` binding이다. `상수`는 현재 integer, string, boolean과 target enum에 정확히 연결되는 named literal만 쓴다. relation, expression, snapshot/template과 조건부 field binding은 이 문형으로 해석하지 않는다.

```rspdl
알림 제목 기록(title_binding)은 점검 요청 전달이 실행될 때 알림 제목을 점검 요청 전달 알림의 제목으로 기록한다.
요청 제목 기록(request_title_binding)은 점검 요청 전달이 실행될 때 대상 요청의 제목을 점검 요청 전달 알림의 요청 제목으로 기록한다.
재시도 횟수 기록(retry_binding)은 점검 요청 전달이 실행될 때 상수 0을 점검 요청 전달 알림의 재시도 횟수로 기록한다.
```

## 문장형 비정규 설계 예시

이 절의 예시는 설계 의도를 검증하기 위한 **비정규** 한국어 문장이다. 현재 parser의 문법도 최종 문법도 아니며 annotation을 사용하지 않는다.

### 점검 요청 알림

```rspdl
시설(facility)은 다음 필드들로 구성되어 있다.
    이름(name): 필수 문자열

기술자(technician)는 다음 필드들로 구성되어 있다.
    표시 이름(display_name): 필수 문자열

점검 요청 상태(request_status)는 다음 값 중 하나다.
    접수됨(received)
    보류됨(on_hold)

점검 요청(maintenance_request)은 다음 필드들로 구성되어 있다.
    제목(title): 필수 문자열
    상태(status): 필수 점검 요청 상태

점검 요청은 기술자를 담당 기술자(assigned_technician)로 가져야 한다.
각 점검 요청은 담당 기술자를 최대 하나만 가져야 한다.

시설 운영자(facility_operator)는 다음 필드들로 구성되어 있다.
    표시 이름(display_name): 필수 문자열

점검 요청 전달(assign_request)은 행동이다.
점검 요청 전달은 기존 시설을 대상 시설(target_facility)로 입력받는다.
점검 요청 전달은 기존 점검 요청을 대상 요청(target_request)으로 입력받는다.
점검 요청 전달은 시설 운영자를 발신 운영자(sender_operator)로 입력받는다.
점검 요청 전달은 기술자를 수신 기술자(recipient_technician)로 입력받는다.

점검 요청 전달 알림(request_assigned_notice)은 다음 필드들로 구성되어 있다.
    시설 이름(facility_name): 필수 문자열
    요청 제목(request_title): 필수 문자열
    내용(content): 필수 문자열

점검 요청 전달 알림은 시설 운영자를 발신자(sender)로 정확히 하나 가져야 한다.
점검 요청 전달 알림은 기술자를 수신자(recipient)로 정확히 하나 가져야 한다.
점검 요청 전달 알림은 점검 요청을 대상(target)으로 정확히 하나 가져야 한다.

점검 요청 전달의 발신 운영자는 점검 요청 전달 알림의 발신자로 기록한다.
점검 요청 전달의 수신 기술자는 점검 요청 전달 알림의 수신자로 기록한다.
점검 요청 전달의 대상 요청은 점검 요청 전달 알림의 대상으로 기록한다.
점검 요청 전달의 대상 시설의 이름은 점검 요청 전달 알림의 시설 이름으로 기록한다.
점검 요청 전달의 대상 요청의 제목은 점검 요청 전달 알림의 요청 제목으로 기록한다.
점검 요청 전달의 대상 요청의 상태가 접수됨이면 점검 요청 전달 알림을 하나 생성한다.
점검 요청 전달의 대상 요청의 상태가 보류됨이면 점검 요청 전달 알림을 생성하지 않는다.

점검 요청 전달 알림의 내용은 다음 메시지 형식으로 생성한다.
    "{시설 이름}의 {요청 제목} 점검 요청이 담당 기술자에게 전달되었습니다."
```

template은 원본을 몰래 읽지 않는다. `시설 이름`과 `요청 제목`은 action input에서 provenance를 가진 output field이고, `내용`은 그 output field만 placeholder로 쓰는 template expression의 producer다. 발신자·수신자·대상도 string ID가 아닌 cardinality가 있는 output relation slot이며, FieldAssignment와 똑같이 typed input root·availability·provenance를 검사한다. 관계를 통해 담당자마다 알림을 만드는 것은 후속 slice이며, 관계 경로의 방향·cardinality·instance identity·deduplication을 명시하기 전에는 이 문장을 fan-out으로 해석하지 않는다.

```rspdl
점검 요청 종료 알림(request_closed_notice)은 다음 필드들로 구성되어 있다.
    요청 제목(request_title): 필수 문자열

점검 요청 종료 알림은 기술자를 수신자(recipient)로 정확히 하나 가져야 한다.

점검 요청 종료(close_request)는 행동이다.
점검 요청 종료는 기존 점검 요청을 대상 요청(target_request)으로 입력받는다.
점검 요청 종료는 기술자를 수신 기술자(recipient_technician)로 입력받는다.
점검 요청 종료는 대상 요청의 제목을 삭제 전 요청 제목(request_title_before_delete)으로 미리 기록한다.
점검 요청 종료가 실행되면 대상 요청을 삭제한다.
점검 요청 종료가 실행되면 점검 요청 종료 알림을 생성한다.
점검 요청 종료의 수신 기술자는 점검 요청 종료 알림의 수신자로 기록한다.
점검 요청 종료의 삭제 전 요청 제목은 점검 요청 종료 알림의 요청 제목으로 기록한다.
```

snapshot 문장이 없으면 마지막 문장은 삭제된 요청 제목을 payload source로 사용한 lifecycle 오류다.

### 대관 견적 가격

```rspdl
행사 유형(event_kind)은 다음 값 중 하나다.
    전시(exhibition)
    강연(lecture)

선택 여부(selection)은 다음 값 중 하나다.
    예(yes)
    아니오(no)

통화(currency)은 다음 값 중 하나다.
    원화(krw)

대관 요청(room_booking)은 다음 필드들로 구성되어 있다.
    이용 시간(hours): 필수 정수
    행사 유형(event_kind): 필수 행사 유형

적용 가격표(pricing_table)은 다음 필드들로 구성되어 있다.
    통화(currency): 필수 통화
    전시 기본 요금(exhibition_base_amount): 필수 정수
    강연 기본 요금(lecture_base_amount): 필수 정수
    장비 추가 요금(equipment_amount): 필수 정수
    장기 이용 할인(long_use_discount): 필수 정수

대관 견적(room_quote)은 다음 필드들로 구성되어 있다.
    통화(currency): 필수 통화
    기본 금액(base_amount): 필수 정수
    장비 추가 금액(equipment_amount): 필수 정수
    할인 금액(discount_amount): 필수 정수
    최종 금액(total_amount): 필수 정수

대관 견적 계산(quote_room)은 행동이다.
대관 견적 계산은 기존 대관 요청을 대상 요청(target_booking)으로 입력받는다.
대관 견적 계산은 적용 가격표를 사용 가격표(applied_price_table)로 입력받는다.
대관 견적 계산은 선택 여부인 장비 추가 여부(include_equipment)로 입력받는다.
대관 견적 계산이 실행되면 대관 견적을 생성한다.

대관 견적 계산의 사용 가격표의 통화는 대관 견적의 통화로 기록한다.
대관 견적 계산의 대상 요청의 행사 유형이 전시이면 대관 견적의 기본 금액은 사용 가격표의 전시 기본 요금에서 계산한다.
대관 견적 계산의 대상 요청의 행사 유형이 강연이면 대관 견적의 기본 금액은 사용 가격표의 강연 기본 요금에서 계산한다.
대관 견적 계산의 장비 추가 여부가 예이면 대관 견적의 장비 추가 금액은 사용 가격표의 장비 추가 요금에서 계산한다.
대관 견적 계산의 장비 추가 여부가 아니오이면 대관 견적의 장비 추가 금액은 0으로 기록한다.
대관 견적 계산의 대상 요청의 이용 시간이 8 이상이면 대관 견적의 할인 금액은 사용 가격표의 장기 이용 할인에서 계산한다.
대관 견적 계산의 대상 요청의 이용 시간이 8 미만이면 대관 견적의 할인 금액은 0으로 기록한다.
대관 견적의 최종 금액은 기본 금액과 장비 추가 금액에서 할인 금액을 뺀 값으로 계산한다.
```

첫 slice는 가격표 field 복사와 constant까지만 지원한다. 목표 가격 모델에서는 각 금액이 통화 unit을 가지며, 가산·차감 expression은 같은 통화 unit의 amount만 조합할 수 있다. expression result는 operand currency와 rounding mode를 evidence에 보존한다. 통화가 다르면 explicit conversion source와 conversion timestamp·rate snapshot 없이는 `RSPDL-PROD-002` type/composition error다. 같은 output의 field를 계산 입력으로 쓸 때는 source 순서가 아니라 acyclic dependency graph로 연결하며 cycle은 오류다. 여기서 이미 확정하는 것은 모든 금액 field가 모든 creation path에서 하나의 producer를 가져야 하고, `0`은 명시적 constant이며 미배정과 같지 않다는 점이다.

## Canonical IR과 분석 규칙

```text
ConditionalProduction {
    id, trigger: ActionId | EventId, output_model
    instance_cardinality: ExactlyOne
    inputs: [TypedInputBinding]
    creation_slot, field_slots: [OutputFieldSlot]
    relation_slots: [OutputRelationSlot]
    creation_branches: [CreationBranch]
    field_producers: [FieldProducer]
    relation_producers: [RelationProducer]
    snapshots: [SnapshotBinding]
    templates: [OutputTemplate]
    source_span
}

CreationBranch {
    id, condition: TypedBooleanExpr, creates: Create | Skip, source_span
}

FieldProducer {
    id, output_field, condition: TypedBooleanExpr
    source: InputPath | SnapshotId | Constant | ExpressionId
    phase: PreMutation | PostMutation, source_span
}

RelationProducer {
    id, output_relation, condition: TypedBooleanExpr
    source: InputPath | SnapshotId | RelationPath
    phase: PreMutation | PostMutation, source_span
}
```

`InputPath`의 root는 trigger의 declared input stable ID여야 한다. 첫 slice에는 action input record field 접근과 single relation-slot input binding만 허용하며 `EventId` lowering은 후속 slice다. `OutputRelationSlot`은 relation endpoint type과 required/unique cardinality를 갖고, `RelationProducer`도 `FieldProducer`와 같은 provenance·type·availability 검사를 받는다. future `RelationPath`는 edge 방향, cardinality, availability phase와 output instance multiplicity를 IR에 보존해야 한다. template은 `OutputFieldId`만 가지며 원본 path를 갖지 않는다. `ExpressionId`가 같은 output의 다른 field를 참조할 때 analyzer는 stable ID dependency graph를 만들고 cycle을 source 순서와 무관하게 거부한다.

creation branch가 정한 `Create` effective region에서 required field slot과 required output relation slot은 정확히 하나의 typed producer를 가져야 한다. field/relation producer는 각자 조건을 가지므로, 가격의 기본 금액·장비 추가 금액·할인 금액은 creation 조건을 cross-product로 복제하지 않고 독립적으로 coverage와 conflict를 분석한다. optional absence, explicit `Skip`, `0`, `false`, 빈 문자열과 solver `UNKNOWN`은 서로 대체되지 않는다. `내용` template도 content field의 `ExpressionId` producer다. 금액 field의 additive/mergeable composition은 field slot에 별도 contract가 선언될 때만 허용한다.

creation slot은 RFC-0006의 typed domain·totality·effective condition을 재사용한다. total slot에서 `Create`/`Skip` 모두 없으면 creation gap이고, `Create`와 `Skip` 또는 둘 이상의 `Create`가 함께 effective하면 creation conflict다. payload gap은 `Create` region 안에서만 판정한다. source 순서와 조건의 겉보기 구체성은 priority가 아니다.

provenance evidence는 다음 chain을 보존한다.

```text
output field/relation -> slot producer -> input path | snapshot | constant | expression
                      -> declaration/source span -> lifecycle phase and availability proof
```

- `RSPDL-PROD-001`: declared input에서 시작하지 않는 source path
- `RSPDL-PROD-002`: source/output/template 또는 relation endpoint type/cardinality mismatch
- `RSPDL-PROD-003`: Create path의 required output field 또는 relation slot producer gap
- `RSPDL-PROD-004`: compatible하지 않은 simultaneous field 또는 relation slot producer conflict
- `RSPDL-PROD-005`: create 전 또는 delete 뒤 source field를 snapshot/retain 없이 사용
- `RSPDL-PROD-006`: output field가 아닌 template placeholder 또는 unavailable placeholder
- `RSPDL-PROD-007`: exact lowering이 불가능한 relation, expression 또는 lifecycle path
- `RSPDL-PROD-008`: output field expression dependency cycle
- `RSPDL-POLICY-007`: creation slot conflict
- `RSPDL-POLICY-008`: total creation slot gap

각 finding은 production/action/output/slot/branch/binding/snapshot stable ID와 span, canonical witness 또는 finite enum compact region, provenance edge와 lifecycle transition을 evidence로 포함한다. `unknown`과 `unsupported`는 정확한 construct와 backend reason을 포함한다. source 위치와 input order는 identity·priority·witness ordering에 참여하지 않는다.

## 적합성 계획

| 분류 | 사례와 기대 계약 |
| --- | --- |
| 정상 | action input에서 발신자·수신자·대상 relation slot과 세 payload field를 복사하고 모든 상태에 Create 또는 explicit Skip을 둔다. template은 output field만 쓴다. |
| 정상 | 삭제 전 제목 snapshot이 post-delete 종료 알림 field를 생산하며 evidence가 capture span을 가리킨다. |
| 정상 | 대관 유형·장비 여부의 모든 조합에서 각 금액 field에 정확히 하나의 producer가 있다. |
| 실패 | 선언하지 않은 input 값을 payload에 쓰면 `RSPDL-PROD-001`과 끊긴 provenance root를 반환한다. |
| 실패 | Create branch가 필수 field 또는 필수 발신자·수신자·대상 relation slot을 배정하지 않으면 `RSPDL-PROD-003`과 branch witness를 반환한다. |
| 실패 | 같은 effective region에서 다른 기본 금액을 쓰면 `RSPDL-PROD-004`와 두 assignment evidence를 반환한다. |
| 실패 | 삭제된 요청의 제목을 snapshot 없이 읽으면 `RSPDL-PROD-005`와 deletion transition을 반환한다. |
| 실패 | 최종 금액과 할인 금액이 서로를 계산 입력으로 쓰면 `RSPDL-PROD-008`과 canonical dependency cycle을 반환한다. |
| 경계 | explicit Skip은 creation/payload gap이 아니며, 미배정 할인 field는 0이 아니다. |
| 오탐 방지 | 상호 배타적인 행사 유형의 서로 다른 기본 금액은 conflict가 아니다. snapshot value는 원본 삭제 뒤에도 unavailable이 아니다. |
| 결정론 | branch·source 나열 순서를 바꿔도 IR identity, diagnostic order와 witness가 같다. |
| unsupported | fan-out, runtime relation join, 일반 arithmetic 또는 외부 환율은 silent success가 아니라 unsupported/unknown이다. |

## 후속 결정

정확한 Korean input/output 문형, relation fan-out, action 간 transaction, expression AST, decimal·currency·rounding·price-table snapshot, default/override surface syntax, delivery/retry/idempotency는 후속 RFC가 정한다. output field가 없는 빈 record, template의 임의 model path, annotation payload binding은 readability와 provenance 계약을 우회하므로 채택하지 않는다.

## References

- [RSPDL Product Requirements](../prd.md)
- [Data Lifecycle Modeling Gap](../problems/0001-data-lifecycle-modeling-gap.md)
- [Policy Consistency Blind Spots](../problems/0002-policy-consistency-blind-spots.md)
- [Semantic Source Provenance Loss](../problems/0005-semantic-source-provenance-loss.md)
- [Field Provenance, Screen Usage, Action Data Mutations, and Sum Derivation Grammar](0005-field-provenance-and-sum-derivation.md)
- [Total Policy Condition Spaces and SMT-First Consistency Analysis](0006-total-policy-condition-space-analysis.md)
- [Finite Relational Rules and Bounded Model Finding](0007-finite-relational-model-finding.md)
- [Korean Domain Frontend Language Specification](0004-natural-korean-domain-grammar.md)
