---
id: data-lifecycle-modeling-gap
title: Data Lifecycle Modeling Gap
type: problem
status: active
created: 2026-08-02
version: "1.1"
summary: Planning artifacts often omit when data comes into existence, changes, disappears, and remains available to dependent behavior.
topics:
  - data-lifecycle
  - state-transition
  - derivation
  - deletion-impact
related:
  - rspdl-product-vision
  - rspdl-language-prd
  - problem-driven-development
last_updated: "2026-08-13"
owners:
  - rspdl-maintainers
---

# Data Lifecycle Modeling Gap

## Why

- 기획 문서는 데이터의 모양을 설명하면서도 데이터가 언제 존재하는지는 생략하기 쉽다.
- 생성되지 않은 데이터는 조회, 수정, 삭제 또는 파생 계산의 입력이 될 수 없다.
- 삭제된 데이터에 의존하는 화면, 정책과 계산은 별도 동작이 정의되지 않으면 완성할 수 없다.
- 개발자는 구현 중에 이 공백을 발견하고 임의로 결정하거나 기획 결정을 기다리게 된다.
- 늦은 결정은 구현 중단, 서로 다른 가정, 데이터 마이그레이션과 재작업으로 이어진다.

## What

- 핵심 원인은 데이터 구조와 데이터 lifecycle을 별개의 기획 대상으로 다루지 않는 것이다.
- 최소 lifecycle 질문은 `생성`, `조회`, `수정`, `삭제`, `파생`이다.
- 각 연산은 데이터가 존재하는 상태, 필요한 권한, 전제조건과 이후 상태를 가져야 한다.
- 다음 항목은 대표적인 실패 유형이다.
  - 생성 경로가 없는 데이터를 조회하거나 수정한다.
  - 이미 삭제된 데이터를 다시 수정하거나 파생 계산에 사용한다.
  - 필수 파생값이 아직 생성되지 않은 입력에 의존한다.
  - 삭제가 다른 데이터, 정책 또는 플로우의 참조를 끊지만 후속 동작이 없다.
  - 상태 전이가 시작 상태에 도달할 수 없거나 종료 상태에서 빠져나갈 수 없다.
- 해결 여부는 문서의 상세도가 아니라 유효한 경로와 위반 경로를 기계적으로 구분할 수 있는지로 판단한다.

## How

- 데이터 관련 기능은 lifecycle 연산과 영향을 받는 상태를 명시한다.
- Canonical IR은 데이터 상태, 연산, 전제조건, 상태 전이와 파생 의존성을 표현한다.
- Semantic analysis는 존재하지 않는 데이터의 사용과 끊어진 의존성을 거부한다.
- 진단은 실패한 연산뿐 아니라 해당 상태에 도달한 경로와 관련 source span을 제공한다.
- 삭제 정책은 cascade, restrict, detach, retain 또는 명시적 미정의 중 하나로 드러나야 한다.
- 파생 데이터는 입력 가용성, 재계산 시점과 source 삭제 이후 동작을 드러내야 한다.
- 공개 의미 규칙에는 정상, 실패, 경계와 오탐 방지 conformance 사례를 둔다.
- 기능 제안과 RFC는 이 문서의 stable ID를 `problem_refs`에 연결한다.

## Constraints

- 데이터 lifecycle은 저장소별 CRUD API나 물리 삭제 구현을 고정하지 않는다.
- `선택 필드가 없음`, `아직 생성되지 않음`, `삭제됨`을 자동으로 같은 상태로 취급하지 않는다.
- 문서에 없는 보존 기간, 복구 가능성, 삭제 전파 방식을 AI가 추측하지 않는다.
- solver가 완전하게 판정하지 못한 경로는 성공으로 근사하지 않고 `unknown`으로 보고한다.
- 현재 vertical slice는 화면·행동 create producer, 화면 CRUD 소비, action create/update/delete 결과 충돌, 합계 계산의 field provenance, 재계산 선언과 미조회 입력 안내를 지원한다.
- 화면 순서·분기, 삭제 이후 접근과 relation/join 기반 집계 범위는 아직 구현하지 않았다.

## References

- [RSPDL Product Vision](../product/vision.md)
- [RSPDL Product Requirements](../prd.md)
- [Typed Domains and Logic Core](../rfcs/0002-typed-domains-and-logic-core.md)
- [Field Provenance, Screen Usage, and Sum Derivation Grammar](../rfcs/0005-field-provenance-and-sum-derivation.md)
- [Problem-driven Development](../guides/problem-driven-development.md)
