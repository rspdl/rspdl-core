---
id: rspdl-product-vision
title: RSPDL Product Vision
type: prd
status: active
created: 2026-08-02
version: "1"
summary: Defines the product promise of moving policy and data decisions before implementation while preserving explicitly modeled intent.
topics:
  - product-vision
  - planning-to-implementation
  - shift-left-validation
  - canonical-intent
related:
  - rspdl-language-prd
  - problem-driven-development
problem_refs:
  - data-lifecycle-modeling-gap
  - policy-consistency-blind-spots
last_updated: "2026-08-02"
owners:
  - rspdl-maintainers
---

# RSPDL Product Vision

## Why

- 제품 정책 검토는 출시를 위해 반드시 필요하지만 사용자 가치가 바로 보이지 않는 반복 작업이다.
- 이 작업은 기획 전담자가 없는 팀과 기획까지 맡은 개발자에게 특히 큰 인지 부하를 만든다.
- 여러 역할을 동시에 수행하는 기획자와 디자이너는 모든 데이터 상태와 조건 조합을 구현 전에 검토하기 어렵다.
- 개발자는 모호한 정책을 임의로 완성하면 재작업하고, 결정을 기다리면 구현 흐름이 멈춘다.
- 팀은 구현 뒤에 모순을 발견하는 대신 기획을 작성하는 순간 결정이 필요한 지점을 확인해야 한다.

## What

- RSPDL은 제품 기획의 데이터, 정책, 권한과 플로우를 사람이 읽고 기계가 검증할 수 있게 만드는 선언형 언어다.
- RSPDL의 제품 약속은 `한 번 명시하고, 일찍 검증하고, 영향과 근거를 함께 전달한다`이다.
- 기획자는 개발 단계에서 발생할 질문과 반례를 구현 전에 받는다.
- 개발자는 추측이 아니라 검증된 결정과 미결정 목록을 입력으로 구현한다.
- 팀은 첫 전달 뒤 동작하는 결과를 만들고 사용자와 의사결정자의 피드백으로 다음 반복을 시작한다.
- AI 에이전트는 같은 Canonical Semantic IR을 읽어 대화마다 전체 기획을 다시 해석하지 않는다.
- 코드 생성과 제품별 산출물 생성은 검증된 IR을 소비하는 후속 기능이며 RSPDL core 자체의 책임은 아니다.
- RSPDL이 보존하는 대상은 명시적으로 모델링된 의도다.
- RSPDL은 작성자가 말하지 않은 의도를 추측하거나 현실의 요구사항과 100% 일치한다고 주장하지 않는다.

## How

- 표면 언어를 결정적으로 파싱해 Locale 독립 Canonical Semantic IR로 변환한다.
- 데이터가 언제 존재하고 어떤 연산과 파생에 사용될 수 있는지 lifecycle 의미로 연결한다.
- 정책 조건 공간을 분석해 충돌, 누락, 중첩과 도달 불가능한 규칙을 구분한다.
- 각 진단에 Rule ID, 원문 위치, 관련 심볼과 재현 가능한 반례 또는 경로를 제공한다.
- Stable ID와 semantic graph로 한 번의 변경이 영향을 주는 정책, 데이터, 플로우와 downstream artifact를 찾는다.
- AI 제안도 사람의 문서와 같은 parser, linker, analyzer와 conformance test를 통과시킨다.
- 구현 순서는 하나의 사용자 시나리오를 끝까지 검증하는 vertical slice를 우선한다.

## Constraints

- 결정론적 규칙으로 증명할 수 없는 내용은 통과나 실패로 추측하지 않고 `unknown` 또는 `미정의`로 남긴다.
- 정합성은 모든 정책이 좋은 제품 판단이라는 뜻이 아니라, 명시된 규칙이 서로 양립하고 필요한 범위를 채운다는 뜻이다.
- AI는 선택지를 제안할 수 있지만 제품 결정을 대신 확정하지 않는다.
- 기획자의 역량을 문제 원인으로 취급하지 않고 역할 과부하와 표현 도구의 부재를 시스템 문제로 다룬다.
- 현재 구현 범위와 목표 범위를 README와 PRD에서 구분한다.

## References

- [RSPDL Product Requirements](../prd.md)
- [Data Lifecycle Modeling Gap](../problems/0001-data-lifecycle-modeling-gap.md)
- [Policy Consistency Blind Spots](../problems/0002-policy-consistency-blind-spots.md)
- [Problem-driven Development](../guides/problem-driven-development.md)
