# RSPDL Agent Context

## Mission

- 기획자와 기획까지 맡은 개발자가 구현 전에 데이터 lifecycle과 정책 정합성 문제를 발견하게 한다.
- 명시된 제품 의도를 deterministic Canonical Semantic IR과 explainable diagnostics로 전달한다.
- 기획 검토라는 반복 작업을 개인의 꼼꼼함에서 재현 가능한 시스템 검증으로 옮긴다.

## Product boundaries

- 표현된 의도는 stable ID와 semantic graph로 보존한다.
- 표현되지 않은 의도, 정책 우선순위와 lifecycle 동작을 추측하지 않는다.
- AI 출력도 사람의 출력과 같은 parser, analyzer와 conformance gate를 통과한다.
- compiler core는 IR, semantic analysis와 diagnostics를 소유한다.
- UI projection, 정책표, 검색, 집계와 code generation은 core IR을 소비하는 application 책임이다.
- 현재 구현과 목표 범위를 혼동하지 않는다. 현재 상태는 `README.md`와 `docs/prd.md`를 기준으로 한다.

## Required context workflow

- 제품 또는 의미 동작을 바꾸기 전에 `.agents/skills/develop-from-product-problem/SKILL.md`를 따른다.
- 문서를 찾거나 변경할 때 `.agents/skills/discover-rspdl-knowledge/SKILL.md`를 따른다.
- 관련 문서 전체를 먼저 읽지 말고 metadata query, problem graph, outline 순서로 context를 좁힌다.
- PRD, RFC, ADR, architecture와 spec은 하나 이상의 `problem_refs`로 causal Problem Topic에 연결한다.
- 기존 원인에 해당하면 새 Problem Topic을 만들지 않는다.

## Root-cause lenses

- `data-lifecycle-modeling-gap`: 생성되지 않았거나 삭제된 데이터는 조회, 수정, 삭제와 파생의 입력이 될 수 없다.
- `policy-consistency-blind-spots`: 조건 공간에서 conflict, gap, overlap과 unreachable을 구분해야 한다.
- 데이터 변경은 create, read, update, delete, derive와 dependency impact를 검토한다.
- 정책 변경은 totality, default, override, witness와 false-positive 사례를 검토한다.

## Implementation rules

- Locale-specific grammar와 token은 `rspdl-ko` 밖으로 누출하지 않는다.
- Canonical IR과 diagnostics는 입력 순서, 실행 시각, OS locale과 hash iteration에 의존하지 않는다.
- 지원하지 않는 의미나 solver timeout을 성공으로 근사하지 않고 structured error 또는 `unknown`으로 남긴다.
- 공개 의미 변경은 정상, 실패, 경계와 오탐 방지 사례를 요구한다.
- snapshot만 갱신해 의미 변경을 승인하지 않는다.
- 구현되지 않은 conformance 영역을 문서에서 구현된 것처럼 표현하지 않는다.

## Verification

- knowledge 문서 변경 뒤 index를 생성한다.
  - `python3 .agents/skills/discover-rspdl-knowledge/scripts/knowledge_index.py build`
- 제출 전 전체 harness를 실행한다.
  - `./scripts/check.sh`
