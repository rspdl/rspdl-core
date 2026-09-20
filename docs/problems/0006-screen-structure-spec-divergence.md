---
id: screen-structure-spec-divergence
title: Screen Structure and Flow Spec Divergence
type: problem
status: active
created: 2026-09-20
version: "1"
summary: Screen hierarchy, on-screen elements, and paths between screens live outside the spec, so they are never checked against the declared data meaning and drift from it.
topics:
  - information-architecture
  - screen-structure
  - wireframe
  - screen-flow
  - artifact-drift
related:
  - rspdl-product-vision
  - rspdl-language-prd
  - data-lifecycle-modeling-gap
  - semantic-source-provenance-loss
  - problem-driven-development
last_updated: "2026-09-20"
owners:
  - rspdl-maintainers
---

# Screen Structure and Flow Spec Divergence

## Why

- 기획 산출물의 중심은 정보구조(IA), 화면 안에 무엇이 놓이는지, 화면 사이를 어떻게 오가는지다.
- 이 셋은 보통 스프레드시트 IA, 별도 화면 흐름도와 디자인 도구 파일에만 존재한다.
- 같은 화면의 의미는 명세에, 구조와 경로는 도구에 있어 두 산출물이 서로를 모른다.
- 어느 한쪽만 고쳐도 아무것도 깨지지 않으므로 둘의 어긋남은 구현 중에야 드러난다.
- 개발자는 폼에 놓인 입력이 어떤 데이터를 만드는지, 버튼이 어디로 가는지를 다시 추측한다.

## What

- 핵심 원인은 **화면의 구조와 경로를 명세의 일부가 아니라 별도 산출물로 다루는 것**이다.
- 화면이 데이터를 생산·소비하는 지점이라는 사실만 명세에 있고, 그 화면이 무엇으로 이루어져
  있고 어디로 이어지는지는 명세 밖에 있어 기계적 대조 대상이 되지 못한다.
- 다음 항목은 대표적인 실패 유형이다.
  - 와이어프레임의 폼에 있는 입력이 명세의 어떤 입력 선언에도 대응하지 않는다.
  - 명세가 선언한 필수 입력이 어떤 화면에도 놓이지 않아 사용자가 채울 자리가 없다.
  - 화면 흐름도의 화살표가 더 이상 존재하지 않는 화면을 가리킨다.
  - 어떤 경로로도 닿을 수 없는 화면이 남아 있고, 그 사실이 어디에도 드러나지 않는다.
  - 내부용·비표시로 선언된 필드가 와이어프레임에 노출된다.
  - IA의 분류가 화면 목록과 어긋나, 분류에만 있는 화면과 분류 없는 화면이 동시에 생긴다.
- 해결 여부는 산출물이 예쁜지가 아니라 **구조·경로와 선언된 의미의 불일치를 기계가 지목할 수
  있는지**로 판단한다.

## How

- 화면의 분류, 화면 안의 요소와 화면 사이 경로는 명세에 선언되어 원문 span을 가진다.
- 요소가 참조하는 필드는 그 화면의 선언된 조작에 실제로 존재해야 한다.
- 경로의 출발 요소와 도착 화면은 선언된 stable ID로만 가리킨다.
- Semantic analysis는 대응 없는 요소, 자리 없는 필수 입력, 끊어진 경로와 닿을 수 없는 화면을
  거부하거나 안내한다.
- 진단은 어긋난 쪽만이 아니라 대조된 **양쪽 선언의 span**을 함께 제공한다.
- 공개 의미 규칙에는 정상, 실패, 경계와 오탐 방지 conformance 사례를 둔다.
- 기능 제안과 RFC는 이 문서의 stable ID를 `problem_refs`에 연결한다.

## Constraints

- 구조는 의미 단위까지만 다루고 좌표, 크기, 색, 간격과 시각 상태는 포함하지 않는다.
  그것은 디자인의 영역이며 application projection이 소유한다.
- 특정 UI 프레임워크의 component 이름이나 렌더링 결과를 명세 대상으로 삼지 않는다.
- 선언되지 않은 배치 의도를 자동으로 채우지 않는다. 비어 있는 것은 비어 있는 채로 드러낸다.
- 분류 깊이와 화면 묶음은 사용자 조사로 정해지는 설계 결정이므로, 특정 깊이를 의미 규칙으로
  강제하지 않고 관례를 벗어날 때 안내한다.
- 경로가 조건에 따라 갈리는 경우의 조건식 의미는 이 문제의 범위가 아니다. 경로의 존재와
  끝점의 정합성만 다룬다.

## References

- [RSPDL Product Vision](../product/vision.md)
- [RSPDL Product Requirements](../prd.md)
- [Data Lifecycle Modeling Gap](0001-data-lifecycle-modeling-gap.md)
- [Semantic Source Provenance Loss](0005-semantic-source-provenance-loss.md)
- [Field Provenance, Screen Usage, Action Data Mutations, and Sum Derivation Grammar](../rfcs/0005-field-provenance-and-sum-derivation.md)
- [Core와 Application Projection 경계](../adr/0002-core-application-boundary.md)
- [Problem-driven Development](../guides/problem-driven-development.md)
