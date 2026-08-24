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

- source-backed semantic record는 진단과 같은 좌표계의 source range로 원문 선언을 식별할 수 있어야
  한다.
- 정상 사례는 각 정책 record에서 서로 다른 선언 문장을 그대로 추출하는 것이다.
- 실패 사례는 allow와 deny가 함께 있어도 각 record가 자기 원문을 잃지 않는 것이다.
- 경계 사례는 여러 source를 compile할 때 range가 어느 file 기준인지 결정적으로 해석되는 것이다.
- 오탐 방지 사례는 source 위치가 stable ID, 의미 정렬, conflict·gap·overlap 판정을 바꾸지 않는 것이다.

## Constraints

- source 위치에서 표현되지 않은 정책 우선순위, lifecycle 동작 또는 conflict를 추론하지 않는다.
- application 전용 table, graph, filter와 navigation 상태를 core IR에 포함하지 않는다.
- source range는 semantic identity가 아니며 generated ID나 hash의 입력이 될 수 없다.
- 개별 reference token 수준의 navigation과 source rewrite는 이 문제의 초기 범위가 아니다.

## References

- [RSPDL Product Vision](../product/vision.md)
- [Core와 Application Projection 경계](../adr/0002-core-application-boundary.md)
- [Frontend and Semantic Analysis Contract](../specs/frontend-semantic-analysis-contract.md)
- [rspdl-core issue #27](https://github.com/rspdl/rspdl-core/issues/27)
