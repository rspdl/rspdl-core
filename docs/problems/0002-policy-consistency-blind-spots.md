---
id: policy-consistency-blind-spots
title: Policy Consistency Blind Spots
type: problem
status: active
created: 2026-08-02
version: "1.1"
summary: Prose planning hides contradictory, uncovered, overlapping, and unreachable policy branches that become visible only during implementation.
topics:
  - policy-conflict
  - policy-gap
  - condition-coverage
  - counterexample
related:
  - rspdl-product-vision
  - rspdl-language-prd
  - problem-driven-development
last_updated: "2026-08-12"
owners:
  - rspdl-maintainers
---

# Policy Consistency Blind Spots

## Why

- 자연어 정책은 조건 공간을 명시적으로 나누지 않아 모순과 누락을 한눈에 보기 어렵다.
- 개발자는 정책을 조건식과 분기로 옮기는 동안 같은 입력에 다른 결과가 생기는 지점을 발견한다.
- 구현 전에 발견하지 못한 정책 공백은 개발자의 임의 결정, 의사결정 대기 또는 배포 후 예외 처리로 이어진다.
- 우연히 의도와 맞는 구현은 검증 가능한 성공이 아니며 한 끗 차이의 해석 차이가 재작업을 만든다.
- 정책 검토가 개인의 꼼꼼함에 의존하면 팀 규모와 정책 수가 늘수록 의도 부채가 누적된다.

## What

- 핵심 원인은 정책 문장이 적용되는 조건 공간과 결과를 완전한 분기로 보지 않는 것이다.
- 정합성 분석은 다음 상태를 서로 구분해야 한다.
  - `conflict`: 같은 입력에서 양립할 수 없는 결과가 동시에 적용된다.
  - `gap`: 결과가 필요하지만 어떤 정책도 적용되지 않는 입력이 존재한다.
  - `overlap`: 둘 이상의 정책이 적용되지만 결과가 양립한다.
  - `unreachable`: 전제조건상 절대 적용될 수 없는 정책이 존재한다.
- 모든 overlap이 오류는 아니며 모든 unmatched가 gap인 것도 아니다.
- total policy, explicit default와 의도된 미정의 범위가 언어 의미로 구분되어야 한다.
- 해결 여부는 경고 수가 아니라 분류의 정확성, 반례의 재현성, 오탐 방지 사례로 판단한다.

## How

- 정책은 actor 또는 role, resource, action, condition, effect와 적용 범위를 Canonical IR로 표현한다.
- analyzer는 조건 공간을 분할하고 conflict, gap, overlap과 unreachable을 별도 Rule ID로 진단한다.
- 각 진단은 충돌한 정책 ID, 관련 source span과 구체적인 witness assignment를 제공한다.
- 우선순위, deny override, allow override 또는 기본 결과는 작성자가 명시할 때만 적용한다.
- 정책 수정 시 semantic graph를 따라 영향을 받는 플로우, 데이터와 다른 정책을 함께 계산한다.
- 공개 규칙에는 정상, 실패, 경계와 오탐 방지 conformance 사례를 둔다.
- 기능 제안과 RFC는 이 문서의 stable ID를 `problem_refs`에 연결한다.

## Constraints

- 정책 정합성은 정책이 사용자에게 바람직하거나 법적으로 적절하다는 판단을 대신하지 않는다.
- 우선순위를 임의로 정해 conflict를 숨기지 않는다.
- solver timeout과 지원하지 않는 조건은 통과가 아니라 `unknown`으로 보고한다.
- 런타임 요청 하나의 unmatched 결과만으로 전체 조건 공간의 gap을 증명하지 않는다.
- 현재 구현은 조건 없는 runtime allow/deny의 `conflict`와 `unmatched`를 분류한다. 별도의 backend-neutral API는 단일 닫힌 enum decision point에서 정적 `gap`, compatible `overlap`, allow/deny `conflict`를 Z3로 판정하지만, 아직 조건부 정책 표면 언어·compiler 진단과 연결되지 않았다. 데이터 relation 쪽에는 unary/binary relation과 명시적 `exclusive`, `exhaustive`, `coexistent`, cardinality를 finite scope에서 검증하는 bounded model finder가 연결되어 있다. `default`, `override`, `unreachable`, 일반 effect compatibility와 임의 양화식은 아직 지원하지 않는다.

## References

- [RSPDL Product Vision](../product/vision.md)
- [RSPDL Product Requirements](../prd.md)
- [Korean Domain Frontend Language Specification](../rfcs/0004-natural-korean-domain-grammar.md)
- [Problem-driven Development](../guides/problem-driven-development.md)
