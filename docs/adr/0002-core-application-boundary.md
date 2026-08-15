---
id: core-application-boundary
title: Core와 Application Projection 경계
type: adr
status: active
created: 2026-07-31
version: "2"
summary: Keeps compiler, IR, semantic analysis, and diagnostics in the RSPDL core while assigning view projections, filtering, and aggregation to applications.
topics:
  - compiler-boundary
  - semantic-ir
  - application-projection
  - policy-tables
related:
  - rspdl-language-prd
  - rspdl-compiler-architecture
  - field-provenance-and-sum-derivation
problem_refs:
  - data-lifecycle-modeling-gap
  - policy-consistency-blind-spots
last_updated: "2026-08-03"
owners:
  - rspdl-maintainers
target_spec: "0.2.0"
---

# Core와 Application Projection 경계

## Why

- RSPDL core의 변경 기준을 언어 의미와 compiler correctness에 맞춘다.
- 정책표, 사용자별 조회와 리소스별 조회는 제품마다 열, 그룹, 필터와 표시 규칙이 달라진다.
- application 조회 요구가 Canonical IR과 semantic API의 구조를 오염시키지 않게 한다.
- core와 application이 독립적인 릴리스 주기와 호환성 계약을 유지하게 한다.

## What

- RSPDL core는 scanner, parser, AST, lowering, linker, Canonical IR과 Semantic Graph를 소유한다.
- RSPDL core는 semantic check, 실행 backend 계약, 구조화된 진단과 결정적 serialization을 소유한다.
- application은 정책표와 기타 view model의 projection을 소유한다.
- application은 사용자별·리소스별 필터, 검색, 정렬, 그룹, 집계와 pagination을 소유한다.
- application의 표시용 집계와 달리 source에 선언된 계산식과 dependency 검증은 core 의미다.
- application은 표시 label, locale별 table column과 undefined 표시 정책을 소유한다.
- 화면의 stable ID와 데이터 생성·입력·조회·수정·삭제 동작은 lifecycle 검증에 필요한 core 의미다.
- 화면 배치, widget, navigation과 시각 상태는 application projection이다.
- `rspdl-compiler`와 `rspdl-cli`에는 application 전용 table 또는 query API를 추가하지 않는다.

## How

- compiler는 application이 임의의 projection을 만들 수 있도록 stable ID와 typed reference를 포함한 Canonical IR을 반환한다.
- compiler는 오류 위치와 근거를 포함한 구조화된 진단을 반환한다.
- application은 공개 IR과 diagnostic serialization만 의존해 view model을 만든다.
- 여러 application이 projection을 공유해야 하면 core와 분리된 integration 또는 application library로 구현한다.
- projection이 언어 의미 자체가 될 때만 별도 RFC와 ADR로 core 편입을 재검토한다.

## Constraints

- core API에는 특정 화면, table column, filter option, pagination 또는 UI 상태를 포함하지 않는다.
- application 편의를 위해 IR에 중복 표시 필드나 집계 결과를 저장하지 않는다.
- semantic check 결과의 설명 가능성과 결정성은 application 경계로 넘기지 않는다.
- 화면을 데이터 생산·소비 지점으로 참조하는 것은 허용하지만 특정 UI component 구조를 core IR에 포함하지 않는다.
- conformance fixture는 application view shape가 아니라 IR, 의미 결과와 진단을 검증한다.

## References

- [RSPDL Language Product Requirements Document](../prd.md)
- [RSPDL Compiler Architecture](../architecture.md)
- [Rust와 한국어 우선 독립 Locale Frontend](0001-rust-korean-first-frontends.md)
- [Field Provenance, Screen Usage, Action Data Mutations, and Sum Derivation Grammar](../rfcs/0005-field-provenance-and-sum-derivation.md)
