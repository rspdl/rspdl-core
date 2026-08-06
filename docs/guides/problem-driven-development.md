---
id: problem-driven-development
title: Problem-driven Development
type: guide
status: active
created: 2026-08-02
version: "1"
summary: Defines how contributors trace every product or language change from a durable causal problem through evidence and conformance tests.
topics:
  - contribution-workflow
  - intent-traceability
  - problem-topic
  - definition-of-done
related:
  - rspdl-product-vision
  - data-lifecycle-modeling-gap
  - policy-consistency-blind-spots
last_updated: "2026-08-02"
owners:
  - rspdl-maintainers
---

# Problem-driven Development

## Why

- 기능 목록만 남으면 구현이 늘어날수록 제품 목적과 원래 의사결정 근거를 복원하기 어려워진다.
- 같은 원인에서 나온 기능이 서로 다른 용어와 가정으로 구현되면 정합성 도구 자체에 의도 부채가 쌓인다.
- 사람과 AI 에이전트가 동일한 원인, 해결 범위와 증명 기준을 읽어야 일관된 변경을 만들 수 있다.

## What

- `Problem Topic`은 반복해서 나타나는 하나의 인과 원인을 설명하는 stable knowledge document다.
- 탐색 중인 문제는 GitHub Issue로 논의하고, 수용된 지속 원인은 `docs/problems/`에 기록한다.
- 추적 방향은 `Problem Topic → PRD/RFC/ADR/Spec → Code → Test/Diagnostic`이다.
- 솔루션 문서의 `problem_refs`는 해결하려는 Problem Topic ID를 가진다.
- 기능 이름, 화면 이름, 구현 작업 또는 미리 정한 솔루션은 Problem Topic이 아니다.

## How

- 변경 전 발견 절차는 다음과 같다.
  - `knowledge_index.py query`로 원인과 관련 개념을 검색한다.
  - 일치하는 Problem Topic의 `graph`와 관련 문서 outline을 확인한다.
  - 기존 원인에 포함되면 새 토픽을 만들지 않고 해당 ID를 연결한다.
  - 기존 원인으로 설명할 수 없으면 문제 템플릿으로 토픽을 만들고 원인부터 검토한다.
- 구현 전 정의 항목은 다음과 같다.
  - 실패하는 사용자 또는 개발 시나리오
  - 문제를 발생시키는 인과 메커니즘
  - 현재 workaround와 재작업 비용
  - 해결을 증명할 정상, 실패, 경계와 오탐 방지 사례
  - 오류 위치, 관련 심볼, 반례 또는 경로를 포함한 진단 계약
- 데이터 변경은 다음 lifecycle 영향을 확인한다.
  - 생성 시점과 생성 주체
  - 조회와 수정이 가능한 존재 상태
  - 삭제 이후 참조와 파생 데이터의 동작
  - 파생값의 입력 가용성과 재계산 시점
- 정책 변경은 다음 condition-space 영향을 확인한다.
  - conflict, gap, overlap과 unreachable 가능성
  - totality와 명시적 default 여부
  - 우선순위 또는 override의 명시 여부
  - 결과를 재현하는 witness와 오탐 방지 사례
- 구현 순서는 다음과 같다.
  - Problem Topic을 `problem_refs`로 연결한다.
  - 가장 작은 end-to-end vertical slice를 선택한다.
  - 의미 규칙과 구조화된 진단을 먼저 계약한다.
  - 가장 가까운 계층의 unit test와 공개 conformance fixture를 추가한다.
  - knowledge index를 rebuild하고 전체 harness를 실행한다.
- 완료 조건은 다음과 같다.
  - 변경 이유를 Problem Topic까지 역추적할 수 있다.
  - 문서, 코드와 fixture가 같은 용어와 Rule ID를 사용한다.
  - 실패가 설명 가능한 evidence를 반환한다.
  - 구현되지 않은 범위와 `unknown` 동작이 명시되어 있다.
  - `./scripts/check.sh`가 통과한다.

## Constraints

- 사소한 오타, 링크 수정과 동작을 바꾸지 않는 정리는 새 Problem Topic을 요구하지 않는다.
- 하나의 변경이 여러 원인을 다루면 모든 관련 ID를 연결하되 PR 범위는 가능한 한 분리한다.
- Problem Topic은 특정 구현을 강제하지 않으며 솔루션 결정은 RFC 또는 ADR에 둔다.
- 테스트 snapshot 갱신만으로 의미 변경을 승인하지 않는다.
- 문서에 없는 제품 결정을 AI 에이전트가 추측해서 채우지 않는다.

## References

- [RSPDL Product Vision](../product/vision.md)
- [Data Lifecycle Modeling Gap](../problems/0001-data-lifecycle-modeling-gap.md)
- [Policy Consistency Blind Spots](../problems/0002-policy-consistency-blind-spots.md)
- [Contributing to RSPDL](../../CONTRIBUTING.md)
- [Knowledge Front Matter](../../.agents/skills/discover-rspdl-knowledge/references/frontmatter-schema.md)
