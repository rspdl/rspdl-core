---
id: workflow-completion-data-availability
title: 업무 완료 경로의 데이터 가용성 계약
type: rfc
status: implemented
version: "1"
summary: Defines explicit workflow starts, completion obligations, preconditions, and submit-bound input acquisition with conservative path analysis.
topics:
  - workflow
  - data-lifecycle
  - path-analysis
  - completion-contract
  - frontmatter
  - canonical-ir
related:
  - document-frontmatter-structure
  - field-provenance-and-sum-derivation
  - frontend-semantic-analysis-contract
  - conditional-data-production
problem_refs:
  - data-lifecycle-modeling-gap
  - screen-structure-spec-divergence
  - semantic-source-provenance-loss
last_updated: "2026-09-23"
owners:
  - rspdl-maintainers
target_spec: "0.6.0"
---

# 업무 완료 경로의 데이터 가용성 계약

## 실패 시나리오와 범위

필수 연락처 입력칸이 있어도 취소 버튼으로 완료 화면에 갈 수 있으면 연락처는 확보됐다고 할 수 없다.
기존 구조 분석은 필드 생산자가 문서 어딘가에 있는지만 검사하고, 어느 완료 경로에서 확보되는지는
검사하지 않는다. 첫 slice는 저자가 명시한 업무의 시작과 완료, 완료 시 필요한 데이터, 입력을 제출해
데이터를 확보하는 버튼을 연결한다. 선언하지 않은 업무 의무나 버튼 의미는 추측하지 않는다.

## 표면 문법

`업무`는 머리말의 선택적 최상위 키다. 기존 문서는 그대로 유효하다.

```rspdl
업무:
  예약 완료(complete_booking):
    시작: 항공편 선택 화면
    초기 데이터:
      - 모델: 예약
        필드: 연락처
    획득:
      - 출발: 승객 정보 화면.submit
        데이터:
          - 모델: 예약
            필드: 연락처
    완료:
      - 화면: 예약 완료 화면
        필수 데이터:
          - 모델: 예약
            필드: 연락처
```

`초기 데이터`는 검증된 생산자가 아니라 **업무 호출자가 충족해야 하는 명시적 precondition**이다.
기존 회원 데이터 조회나 외부 시스템 제공 값을 표현할 수 있지만 컴파일러가 실제 조회 성공을 보장하지
않는다. `획득`은 `화면.버튼` 출발점에 묶인다. 해당 필드는 그 화면의 입력 문장에 선언되고 실제
레이아웃 입력으로 배치되어야 한다. 입력 화면을 방문하거나 조회 문장이 있다는 사실만으로 확보되지
않는다. 완료 화면 도착 시 검사하므로 완료 화면 자체의 입력은 소급해 인정하지 않는다.

## Canonical IR과 분석

`WorkflowDefinition`은 stable workflow ID, 시작 화면 ID, 초기 데이터, 버튼별 획득 데이터, 완료
화면별 필수 데이터를 각 source span과 함께 보존한다. 모델과 필드는 Canonical ID로 연결된다.

분석은 시작 화면에서 도달 가능한 화면만 대상으로 한다. 각 화면 도착 시 반드시 확보된 필드 집합은
모든 선행 경로의 교집합이며, 버튼을 지날 때 그 버튼에 명시된 획득만 더한다. 유한 필드 집합 위의
고정점이므로 순환에서도 종료한다. 설명 문자열은 조건이 아니라 nondeterministic alternative다.
완료가 시작 화면이면 초기 데이터만 검사한다. 완료 화면에서 나가는 흐름은 그 완료 의무를 바꾸지 않는다.

## 진단

- `RSPDL-WORKFLOW-001` / `semantic.workflow.required_data_unavailable`: 완료에 도달하지만 필수
  데이터가 없는 경로. 완료 선언 위치와 결정적 누락 경로 witness를 arguments에 싣는다.
- `RSPDL-WORKFLOW-002` / `semantic.workflow.completion_unreachable`: 시작에서 완료에 도달 불가.
- `RSPDL-WORKFLOW-003` / `semantic.workflow.acquisition_source_not_found`: 획득 출발 흐름 없음.
- `RSPDL-WORKFLOW-004` / `semantic.workflow.acquired_data_not_placed_input`: 획득 필드가 해당 화면의
  선언되고 배치된 입력이 아님.
- `RSPDL-WORKFLOW-U001` / `semantic.workflow.verification_unknown`: 도달 경로에 delete 또는 조건부
  action 생산이 있어 현재 slice가 가용성을 확정할 수 없음. 이때 같은 의무에 확정적 누락 진단을
  함께 내지 않는다.

## 비범위와 증거

조건 설명 해석, 임의 action 결과의 field 생산, 삭제 후 복구, cross-file 화면 경로, 정책 실행과
동시성은 지원하지 않는다. 관련 의미가 경로에 나타나면 성공으로 근사하지 않고 unknown을 낸다.
`conformance/ko-KR/workflow-data`는 대체 입력 경로, 취소 우회, 초기 precondition, 도달 불가,
포매터 왕복을 고정한다. SDK JSON은 같은 Canonical IR과 진단 arguments를 그대로 노출한다.

## References

- [Data Lifecycle Modeling Gap](../problems/0001-data-lifecycle-modeling-gap.md)
- [Document Frontmatter Structure](0009-document-frontmatter-structure.md)
- [Field Provenance](0005-field-provenance-and-sum-derivation.md)
