---
id: semantic-reference-direction-loss
title: Semantic Reference Direction Loss
type: problem
status: active
created: 2026-09-26
version: "1"
summary: Canonical IR preserves resolved target IDs but not a navigable account of which semantic records refer to each target.
topics:
  - semantic-ir
  - semantic-graph
  - downstream-navigation
  - impact-analysis
related:
  - rspdl-product-vision
  - rspdl-compiler-architecture
  - core-application-boundary
  - semantic-source-provenance-loss
last_updated: 2026-09-26
owners:
  - rspdl-maintainers
---

# Semantic Reference Direction Loss

## Why

- 기획자와 AI agent는 한 모델·필드·역할·행동을 바꾸기 전에 그 심볼을 사용하는 정책, 제약,
  화면과 workflow를 찾아야 한다.
- downstream application은 target ID만 든 Canonical IR에서 역방향 사용처를 찾으려고 모든 record를
  훑으며 어떤 필드가 참조인지 다시 분류한다.
- 새 IR construct가 추가되거나 필드 모양이 바뀌면 이 분류가 조용히 누락되어 변경 영향을 덜 보여준다.

## What

- analyzer는 참조를 선언에 연결하지만 결과는 각 record 안의 target ID만 보존한다. 연결 과정이 알던
  source와 target 관계가 공개 분석 계약에서 사라지는 것이 반복 원인이다.
- `*_id` 모양의 모든 값을 참조로 간주할 수 없다. 선언 ID, source provenance, runtime record ID와
  owner 안에서만 유일한 화면 요소·조회 결과 ID가 같은 JSON 모양을 쓴다.
- 소비자가 IR 타입과 scope 규칙을 알아야만 한 심볼의 사용처를 정확히 찾을 수 있으면 문제가 존재한다.

## How

- 정상 사례는 해석된 semantic reference마다 source record, target symbol, 관계 종류와 source range를
  downstream이 문법이나 IR field 목록 없이 식별할 수 있는 것이다.
- 실패 사례는 해석되지 않아 module이 없는 source에서 존재하지 않는 관계를 만들지 않는 것이다.
- 경계 사례는 여러 파일에 같은 local ID가 있어도 file과 owner scope로 서로 구별되는 것이다.
- 오탐 방지 사례는 선언 ID, label, source ID와 일반 문자열을 semantic reference로 내보내지 않는 것이다.

## Constraints

- 표현되지 않은 관계나 unresolved reference를 추측하지 않는다.
- application 전용 정렬, 필터, pagination 또는 화면 상태를 core 계약에 넣지 않는다.
- 이 문제는 module 사이 import resolution이나 cross-file linking을 새로 정의하지 않는다.

## References

- [RSPDL Product Vision](../product/vision.md)
- [RSPDL Compiler Architecture](../architecture.md)
- [Core와 Application Projection 경계](../adr/0002-core-application-boundary.md)
- [Semantic Source Provenance Loss](0005-semantic-source-provenance-loss.md)
- [rspdl-core issue #41](https://github.com/rspdl/rspdl-core/issues/41)
