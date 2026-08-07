---
id: rust-korean-first-frontend
title: Rust와 한국어 우선 독립 Locale Frontend
type: adr
status: accepted
version: "2"
summary: Selects Rust, a Korean-first rollout, and independent deterministic frontends that lower only to shared Unlinked IR.
topics:
  - rust
  - korean-first
  - controlled-language
  - locale-frontends
  - deterministic-parsing
related:
  - rspdl-language-prd
  - rspdl-compiler-architecture
  - controlled-korean-surface-grammar
  - frontend-semantic-analysis-contract
problem_refs:
  - data-lifecycle-modeling-gap
  - policy-consistency-blind-spots
last_updated: "2026-08-06"
owners:
  - rspdl-maintainers
target_spec: "0.2.0"
---

# Rust와 한국어 우선 독립 Locale Frontend

## 상태

Accepted.

이 문서는 구현 기술과 Locale frontend의 책임에 관해 합의한 사항을 기록한다. 구체적인 한국어 문형은 [Controlled Korean Surface Grammar RFC](../rfcs/0001-controlled-korean-surface-grammar.md)에서 별도로 다룬다.

## 배경

RSPDL은 `ko-KR`과 `en-US`처럼 표현 방식이 다른 문서를 공통 Canonical Semantic IR로 변환해야 한다. Locale은 단순 번역 테이블이 아니라 문법, 어순, 키워드, 진단 표현을 담당한다.

한국어 표면 문법은 자유로운 한국어를 해석하지 않는다. RSPDL이 정의한 제한 문형에서 조사와 종결 표현을 구조 표지로 인식하고, 고정된 슬롯을 추출한다.

## 결정

### Rust

RSPDL의 기준 구현은 Rust workspace로 개발한다.

선택 이유는 다음과 같다.

- 결정론적인 compiler pipeline을 명시적인 타입 경계로 표현할 수 있다.
- 형태소 분석기나 네이티브 NLP 런타임 없이 단일 구현을 제공할 수 있다.
- CLI, 라이브러리와 WASM 대상으로 확장할 수 있다.
- fixture 기반 conformance test runner를 같은 workspace에서 제공할 수 있다.

### 한국어 우선

`ko-KR` frontend를 최초 구현이자 초기 참조 frontend로 개발한다.

`en-US` frontend는 한국어 문법을 번역하거나 키워드만 치환하지 않는다. 영어에서 자연스럽고 제한된 표면 문법을 독립적으로 정의하고 구현하되, 동일한 Canonical IR과 의미 진단 계약을 만족해야 한다.

### 독립 Locale frontend

각 Locale frontend는 다음 구성 요소를 소유한다.

- scanner와 lexer
- CST와 Locale AST
- CFG parser와 오류 복구
- Locale surface lint
- formatter
- Locale AST에서 공통 Unlinked IR로의 lowering

공통 코어는 한국어 조사나 영어 어순을 알지 않는다. Locale frontend는 내부 문법이 달라도 공통 `FrontendOutput` 계약을 통해 symbolic reference와 source provenance를 가진 `UnlinkedModule`을 반환한다.

Frontend는 symbol resolution, type checking, anonymous semantic ID 생성, lifecycle 또는 policy 분석을 수행하지 않는다. 공통 linker와 analyzer가 모든 frontend output에 같은 규칙을 적용한다. 구체적인 계약은 [Frontend and Semantic Analysis Contract](../specs/frontend-semantic-analysis-contract.md)를 따른다.

### 형태소 분석을 사용하지 않는 정확성 경로

compiler correctness는 Kiwi 또는 다른 형태소·품사·자연어 분석기에 의존하지 않는다.

한국어 frontend는 다음 방식으로만 입력을 인식한다.

1. 인용 식별자, 원시 어절, 구두점과 주석을 scan한다.
2. parser가 기대하는 위치에서 정의된 접미 marker를 분리한다.
3. 고정 문형의 슬롯과 종결 token sequence를 검증한다.
4. Locale AST를 Unlinked IR로 lowering한다.

자연스러운 조사 선택은 parse 성공 조건이 아니다. surface linter가 비차단 진단을 만들고 formatter가 권장 표현으로 정규화한다.

## 결과

### 긍정적 결과

- 지원되는 문형과 파싱 결과가 결정론적이다.
- 한국어 문법 오류에 구체적인 source span과 복구 힌트를 제공할 수 있다.
- 외부 NLP 모델, 사전, 네이티브 라이브러리 버전에 결과가 좌우되지 않는다.
- Locale 사이의 구현 결합을 낮추고 IR 동등성을 conformance fixture로 검증할 수 있다.

### 비용

- Locale마다 scanner, parser, lint와 formatter를 구현해야 한다.
- 새 문형은 명시적인 grammar production, lowering과 fixture를 함께 추가해야 한다.
- 동일 의미를 표현하는 문형이 늘어날수록 중의성 검토와 오탐 방지 테스트가 필요하다.

## 결정하지 않은 사항

다음 사항은 이 ADR의 범위가 아니다.

- RSPDL 파일 확장자
- 세부 선언 문법과 전체 예약어
- parser library 사용 여부
- 한국어에서 지원할 문형의 최종 목록
- stable machine ID와 한국어 표시 이름을 연결하는 표면 문법
- 영어 frontend의 구체적인 문형과 구현 시점
