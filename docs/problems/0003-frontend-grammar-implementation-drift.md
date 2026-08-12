---
id: frontend-grammar-implementation-drift
title: Frontend Grammar Implementation Drift
type: problem
status: active
created: 2026-08-12
version: "2"
summary: Normative grammar and executable parsers are maintained separately, causing repeated implementation work and undetected drift as surface languages grow.
topics:
  - frontend-development
  - executable-grammar
  - parser-maintenance
  - specification-drift
  - regression-safety
related:
  - rspdl-compiler-architecture
  - controlled-korean-surface-grammar
  - natural-korean-domain-grammar
  - rust-korean-first-frontend
last_updated: "2026-08-13"
owners:
  - rspdl-maintainers
---

# Frontend Grammar Implementation Drift

## Why

- Locale frontend 개발자는 문형을 추가할 때 규범 EBNF, 문장 분류, parser cursor, 진단과 fixture를 각각 수정해야 한다.
- 같은 문법 결정을 여러 표현으로 옮기는 동안 구현 누락과 판별 순서에 따른 오탐이 생기며, 다른 Locale frontend도 같은 기반 작업을 반복해야 한다.
- 손 parser를 안전하게 교체할 동등성 증거가 없으면 이미 동작하는 문법을 한 번에 재작성하거나 계속 중복 구현하는 선택지만 남는다.

## What

- 규범 문법과 실행 parser가 서로 다른 source of truth인 것이 반복 비용의 원인이다.
- 의미 분석, lowering이나 Locale 표현 품질처럼 문법 밖의 책임이 복잡한 것은 이 문제와 구분한다.
- 문형 하나를 추가할 때 동일한 token 순서와 분기 조건을 문서와 Rust 코드에 반복 작성해야 하거나, 규범 production과 parser가 다른 입력 집합을 허용하면 문제가 존재한다.

## How

- 규범 production과 executable parser의 허용 입력 집합이 독립적으로 수정되면 같은 문형에 서로 다른 token boundary와 분기 우선순위가 축적된다.
- drift는 RFC에 있는 정상·실패 문형이 parser에서는 다르게 수용되거나, source span·진단·복구 결과가 문서의 경계와 맞지 않을 때 관찰된다.
- 새 문형마다 이미 문서화한 token 순서와 marker 조건을 parser 코드에 다시 옮기고 별도 회귀 fixture를 작성하는 시간이 반복 비용으로 나타난다.
- Locale 수와 문형 수가 늘수록 중복된 판별 로직과 검증 조합이 함께 증가하므로, 변경 범위가 작아도 회귀 확인 비용은 지속적으로 커진다.

## Constraints

- parser 자동화가 Locale별 lowering, semantic analysis, lint와 formatter 의미를 추측하거나 합치지 않는다.
- 문법 충돌을 선언 순서로 숨기거나 여러 parse 중 하나를 임의 선택하지 않는다.
- generated parser 도입 자체가 공개 문법 또는 Canonical IR 변경의 근거가 될 수 없다.
- 범용 자연어 이해, 형태소 분석과 모든 parser algorithm을 지원하는 범용 compiler-compiler는 이 문제의 필수 범위가 아니다.

## References

- [RSPDL Compiler Architecture](../architecture.md)
- [Controlled Korean Surface Grammar](../rfcs/0001-controlled-korean-surface-grammar.md)
- [Korean Domain Frontend Language Specification](../rfcs/0004-natural-korean-domain-grammar.md)
- [Rust와 한국어 우선 독립 Locale Frontend](../adr/0001-rust-korean-first-frontends.md)
