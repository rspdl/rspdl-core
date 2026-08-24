---
id: field-provenance-and-sum-derivation
title: Field Provenance, Screen Usage, Action Data Mutations, and Sum Derivation Grammar
type: rfc
status: implemented
created: 2026-08-03
version: "0.4"
summary: Defines sentence-shaped screen operations, action data mutations, provenance checks, sum dependencies, and recalculation triggers.
topics:
  - data-lifecycle
  - field-provenance
  - screen-usage
  - action-result
  - derivation
  - aggregation
  - diagnostics
related:
  - natural-korean-domain-grammar
  - core-application-boundary
  - rspdl-language-prd
problem_refs:
  - data-lifecycle-modeling-gap
  - semantic-source-provenance-loss
last_updated: "2026-08-24"
owners:
  - rspdl-maintainers
target_spec: "0.2.0"
---

# Field Provenance, Screen Usage, Action Data Mutations, and Sum Derivation Grammar

## Why

- 데이터 모델의 필드 선언만으로는 값이 어느 화면이나 계산에서 만들어지고 어디에서 소비되는지 알 수 없다.
- 생산자가 없는 필드를 조회·수정하거나 계산 입력으로 사용하면 개발자가 구현 중에 값을 임의로 채우거나 결정을 기다려야 한다.
- 입력됐지만 어떤 화면에서도 조회되지 않는 필드는 누락인지 내부 데이터인지 기획 문서만으로 구분하기 어렵다.
- 계산식은 원본 필드뿐 아니라 원본이 바뀌었을 때 다시 계산하는 정책까지 있어야 stale data를 피할 수 있다.
- 하나의 행동 결과가 같은 데이터 모델을 수정하면서 동시에 삭제한다면 구현자가 어느 결과를 적용할지 결정할 수 없다.

## What

- 화면 동작은 annotation이나 들여쓰기 block이 아닌 독립적인 한국어 문장으로 선언한다.
  ```rspdl
  장바구니 항목 입력 화면(create_item)에서는 장바구니 항목의 수량, 금액을 입력할 수 있다.
  장바구니 항목 화면(item_detail)에서는 장바구니 항목의 수량, 금액을 조회할 수 있다.
  ```
- 화면 stable ID는 화면 문장마다 표시 이름 뒤에 쓴다. 같은 ID와 이름의 문장들은 하나의 Canonical screen으로 병합된다.
- `입력`은 필드 생산자이고 `조회`와 `수정`은 필드 소비자다.
- 하나의 필드는 화면 입력과 계산 결과를 동시에 생산자로 가질 수 없다.
- 데이터 모델의 `생성` 화면이 있어야 그 모델을 화면이나 계산에서 사용할 수 있다.
- 화면 밖에서 데이터를 만드는 경로는 선언된 행동의 `생성` 결과로 표현할 수 있다.
  ```rspdl
  주문 등록(register_order)은 행동이다.
  주문 등록이 실행되면 주문을 생성한다.
  ```
- 행동 결과는 데이터 모델 단위의 `생성`, `수정`, `삭제` mutation이다. 같은 행동이 같은 모델에 서로 다른 mutation을 동시에 선언하면 충돌이다.
- 서로 다른 행동이 같은 모델을 각각 수정·삭제하거나, 같은 행동이 서로 다른 모델을 변경하는 것은 충돌이 아니다.
- 합계 계산은 대상 필드와 다른 데이터 모델을 포함할 수 있는 원본 필드를 연결한다.
  ```rspdl
  장바구니의 결제 예정 금액은 장바구니 항목의 금액의 합계로 계산한다.
  장바구니 항목의 금액이 바뀔 때 장바구니의 결제 예정 금액을 다시 계산한다.
  ```
- 입력됐지만 조회되지 않는 필드는 의도를 별도 문장으로 명시할 수 있다.
  ```rspdl
  감사 기록의 내부 메모는 내부 관리에만 사용한다.
  감사 기록의 위험 점수는 사용자 화면에서 조회하지 않는다.
  ```

## How

- 규범 문형은 다음과 같다.
  ```ebnf
  screen-model-operation = screen-name, stable-id, "에서는", model-reference,
      ("을" | "를"), ("생성할" | "조회할" | "수정할" | "삭제할"), "수", "있다", "." ;
  screen-field-operation = screen-name, stable-id, "에서는", model-reference, "의",
      field-list, ("을" | "를"), ("입력할" | "조회할" | "수정할"), "수", "있다", "." ;
  action-data-mutation = action-reference, ("이" | "가"), "실행되면",
      model-reference, ("을" | "를"), ("생성한다" | "수정한다" | "삭제한다"), "." ;
  field-list = field-reference, { ",", field-reference } ;
  sum-derivation = model-reference, "의", field-reference, ("은" | "는"),
      model-reference, "의", field-reference, "의", "합계로", "계산한다", "." ;
  recalculation = model-reference, "의", field-reference, ("이" | "가"), "바뀔", "때",
      model-reference, "의", field-reference, ("을" | "를"), "다시", "계산한다", "." ;
  ```
- Canonical IR은 screen별 model/field operation, action별 model mutation, 합계 원본 field ID, 대상 field ID, 재계산 원본 field ID와 비표시 의도를 정렬해 보존한다. 각 source-backed record는 자기 문장의 UTF-8 byte `TextRange`를 가지며, 재계산은 source·target field ID와 문장 span을 가진 별도 record로도 보존한다. 기존 `Compilation.action_data_mutation_provenance` sidecar는 resolved mutation의 `SourceId`를 함께 노출한다.
- 합계 원본과 대상은 현재 모두 정수 필드여야 한다.
- `RSPDL-DATA-001`은 조회·수정·계산 입력 필드에 화면 입력 또는 선언된 producer/derivation graph에서 구조적으로 도달 가능한 계산 생산자가 없을 때 발생한다.
- 구조적 도달 가능성은 선언된 계산 dependency의 고정점만 뜻하며 화면 실행 순서, 분기 또는 path별 데이터 availability를 추론하지 않는다.
- `RSPDL-DATA-002`는 조회·수정·삭제·계산 또는 행동 결과가 사용하는 데이터 모델에 화면 또는 행동 `생성` 결과가 없을 때 발생한다.
- `RSPDL-DATA-003`은 계산 필드의 재계산 시점이 없거나 둘 이상일 때 발생한다.
- `RSPDL-DATA-004`는 화면 동작, 생산자, 행동 결과, 계산, 재계산 또는 필드 의도가 중복·불일치할 때 발생한다. 같은 action ID와 model ID에 둘 이상의 mutation kind가 있으면 행동 결과 충돌이다.
- `RSPDL-DATA-005`는 합계 원본 또는 대상이 정수가 아닐 때 발생한다.
- `RSPDL-DATA-006`은 참조한 모델이나 필드가 없을 때 발생한다.
- `RSPDL-DATA-W001`은 생산된 필드가 조회되지 않고 내부/비표시 의도도 없을 때 안내한다.
- `RSPDL-DATA-W002`는 교차 모델 합계의 레코드 선택 관계가 없어 계산 범위가 `unknown`임을 안내한다.

## Constraints

- 현재 분석은 화면 선언 집합의 구조적 생산·소비 관계만 판정하며 화면 간 실행 순서와 분기를 모델링하지 않는다.
- 화면 `create`와 행동 `create`는 순서와 무관한 구조적 model producer다. 화면 read/update/delete, 계산의 source·target model과 행동 update/delete는 producer가 필요한 consumer다.
- 화면 ID는 행동 ID가 아니다. 같은 화면에서 수정·삭제 capability를 모두 제공하는 것만으로 행동 결과 충돌을 만들지 않는다.
- action mutation의 source provenance는 진단과 downstream traceability를 위한 metadata이며 action·model·mutation semantic identity나 conflict key에는 참여하지 않는다.
- 삭제 이후 조회·수정, 조건부 생성과 path별 availability는 성공으로 추측하지 않고 지원 범위 밖에 둔다.
- 행동 결과의 조건, 실행 순서, field 단위 mutation과 read/derive 결과는 아직 지원하지 않는다.
- 교차 모델 합계는 semantic dependency를 보존하지만 relation/join이 도입되기 전까지 실제 집계 레코드 범위를 실행하지 않는다.
- 화면의 stable ID와 데이터 동작은 core 의미지만 화면 배치, widget, navigation과 시각 상태는 application projection이다.
- 현재 계산식은 단일 정수 필드의 합계만 지원한다. 사칙연산, 조건, 다중 원본, snapshot과 조회 시 계산은 후속 문법이다.
- 기존 0.1 source에 화면·계산 문장이 없으면 provenance 분석을 강제하지 않는다.
- 화면·계산이 없는 기존 module의 JSON에는 빈 provenance collection을 출력하지 않는다.

## References

- [Data Lifecycle Modeling Gap](../problems/0001-data-lifecycle-modeling-gap.md)
- [RSPDL Product Requirements](../prd.md)
- [Core와 Application Projection 경계](../adr/0002-core-application-boundary.md)
- [Korean Domain Frontend Language Specification](0004-natural-korean-domain-grammar.md)
