---
id: semantic-source-provenance-loss
title: Semantic Source Provenance Loss
type: problem
status: active
created: 2026-08-24
version: "1"
summary: Semantic records lose their source locations during analysis, preventing reordered downstream views from returning users to the declarations that produced them.
topics:
  - semantic-ir
  - source-provenance
  - downstream-navigation
  - explainability
related:
  - rspdl-product-vision
  - rspdl-language-prd
  - core-application-boundary
  - frontend-semantic-analysis-contract
last_updated: 2026-08-24
owners:
  - rspdl-maintainers
---

# Semantic Source Provenance Loss

## Why

- 기획자는 정책표, lifecycle graph와 dependency view에서 문제를 찾은 뒤 자신이 작성한 선언으로
  돌아가야 한다.
- downstream application이 Canonical Semantic IR을 표·그래프처럼 source 순서와 다른 형태로
  재배열하면 semantic record가 어느 문장에서 왔는지 알 수 없다.
- 소비자가 표시 이름과 한국어 문형으로 원문을 다시 검색하면 core 문법을 중복 구현하게 되고,
  문법 변경 뒤 잘못된 문장을 조용히 연결할 수 있다.

## What

- scanner와 frontend는 source range를 계산하지만 공통 analyzer가 Canonical Semantic IR을 만들 때
  대부분의 위치가 사라지는 것이 반복 원인이다.
- diagnostic만 source span을 갖는 상태는 잘못된 입력을 표시하는 데는 충분하지만, 진단이 없는
  정상 정책·데이터·dependency record의 근거를 보여주지 못한다.
- 같은 semantic key의 allow와 deny처럼 compiler diagnostic 없이 사람이 검토해야 하는 record를
  재배열했을 때 원문을 정확히 찾을 수 없으면 문제가 존재한다.

## How

- 정상 사례는 source 순서와 다르게 배열된 각 semantic record에서 사용자가 그 근거가 된 선언으로
  정확히 돌아갈 수 있는 것이다.
- 실패 사례는 별개의 선언에서 나온 record가 같은 semantic key를 가질 때 원문 근거를 서로
  구별하지 못하는 것이다.
- 경계 사례는 여러 source를 함께 compile하고 결과를 재배열해도 각 record의 원문 근거가 어느
  source에 속하는지 모호하지 않은 것이다.
- 오탐 방지 사례는 downstream이 core 문법을 다시 해석하지 않고도 원문 근거를 찾는 것이다.

## Constraints

- source 위치에서 표현되지 않은 정책 우선순위, lifecycle 동작 또는 conflict를 추론하지 않는다.
- application 전용 table, graph, filter와 navigation 상태를 core IR에 포함하지 않는다.

## References

- [RSPDL Product Vision](../product/vision.md)
- [RSPDL Product Requirements](../prd.md)
- [Core와 Application Projection 경계](../adr/0002-core-application-boundary.md)
- [Frontend and Semantic Analysis Contract](../specs/frontend-semantic-analysis-contract.md)
- [rspdl-core issue #27](https://github.com/rspdl/rspdl-core/issues/27)
