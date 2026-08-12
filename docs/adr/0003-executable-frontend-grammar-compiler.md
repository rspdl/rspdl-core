---
id: executable-frontend-grammar-compiler
title: 실행 가능한 Frontend Grammar Compiler
type: adr
status: accepted
version: "2"
summary: Selects a small RSPDL-specific EBNF compiler and production-by-production differential migration instead of duplicating normative grammar in handwritten parsers.
topics:
  - executable-grammar
  - parser-generation
  - frontend-infrastructure
  - differential-testing
related:
  - rspdl-compiler-architecture
  - rust-korean-first-frontend
  - controlled-korean-surface-grammar
  - natural-korean-domain-grammar
problem_refs:
  - frontend-grammar-implementation-drift
last_updated: "2026-08-13"
owners:
  - rspdl-maintainers
---

# 실행 가능한 Frontend Grammar Compiler

## 상태

Accepted.

이 결정은 parser 구현 방식을 바꾸지만 지원 문법, Locale AST, Canonical IR과 공개 진단 계약은 바꾸지 않는다.

## 배경

한국어 frontend의 규범 production은 RFC의 EBNF에, 실행 동작은 handwritten parser의 문장 분류 함수와 cursor 코드에 별도로 존재한다. 화면, provenance와 관계 문형이 추가되면서 같은 token shape를 여러 위치에 반복하고 parser 전환을 검증할 공통 oracle이 없는 비용이 커졌다.

범용 parser generator를 직접 만드는 것은 현재 문제보다 넓다. 반대로 기존 handwritten parser를 즉시 교체하면 source span, contextual marker 분리와 복구 동작의 회귀를 한 번에 감수해야 한다.

## 결정

### RSPDL 전용 grammar compiler

workspace에 `rspdl-grammar-compiler`를 둔다. 이 crate는 제한된 EBNF를 grammar IR로 읽고 검증하며 Rust parser definition을 생성한다.

초기 문법 기능은 다음으로 제한한다.

- 이름 있는 공개 production
- sequence와 alternative
- optional과 repetition
- capture
- literal terminal
- Locale adapter가 구현하는 contextual matcher

compiler는 duplicate 또는 undefined rule, unknown matcher, nullable repetition과 left recursion을 build failure로 거부한다. runtime은 같은 입력을 둘 이상의 방식으로 완전히 parse하면 하나를 선택하지 않고 ambiguity를 반환한다.

### Locale adapter 경계

scanner와 contextual token 해석은 Locale frontend가 소유한다. 한국어의 `marked_ref("은", "는")` matcher는 parser가 기대하는 위치에서만 suffix marker를 분리하며 grammar compiler core에 한국어 token을 추가하지 않는다.

generated parser는 capture와 token range를 반환한다. Locale AST 구성, surface lint, 표시 이름 resolution, Unlinked IR lowering과 formatter는 기존 frontend 책임으로 남는다.

### 점진적 전환

handwritten parser를 한 번에 제거하지 않는다.

1. production 하나를 executable grammar로 선언한다.
2. 기존 parser와 generated parser를 같은 정상·실패·경계 corpus에 실행한다.
3. 성공 여부와 capture 또는 Locale AST가 동등함을 검증한다.
4. 동등성 gate를 만족한 production만 production path로 전환한다.
5. 모든 production이 전환된 뒤 중복 handwritten code를 제거한다.

첫 vertical slice는 policy statement다. 이어서 제약·literal, 선언·block item, 화면·provenance, 관계·meta-rule 문형도 같은 방식으로 이관한다. generated parser는 shadow test에서만 실행하고 사용자-visible parse 결과를 변경하지 않는다.

### 현재 migration 상태

`rspdl-ko`의 공개 문형은 다음 executable grammar 묶음으로 shadow migration되었다.

- policy statement
- field constraint와 literal
- module, enum, model, role, action 선언과 block item
- screen, sum derivation, recalculation, field intent
- unary/binary relation과 relation meta-rule

각 묶음은 기존 handwritten parser를 oracle로 삼아 정상 AST, 실패 shape, marker 경계와 false positive 방지를 비교한다. build script는 grammar 파일을 자동 발견하고 안정된 순서로 생성하므로 새 문형은 compiler wiring을 복사하지 않고 grammar와 adapter만 추가할 수 있다.

아직 production parser 전환과 handwritten code 제거는 수행하지 않았다. 그 전에 structured diagnostic과 recovery parity를 grammar/runtime 계약에 추가하고, 문형별 동등성 gate를 production path에서도 유지해야 한다.

## 테스트 계약

- grammar compiler unit test는 정상 compile, syntax error, duplicate/undefined rule, unknown matcher, nullable repetition과 left recursion을 검증한다.
- runtime test는 정상 match, 가장 먼 failure expectation과 ambiguity를 검증한다.
- Locale differential test는 정상, 실패, marker 경계와 false-positive prevention 입력을 기존 parser와 비교한다.
- production path 전환 전 기존 workspace test와 conformance fixture가 변경 없이 통과해야 한다.
- 임의 UTF-8 입력은 panic하지 않고 결과와 진단 순서가 반복 실행 사이에 같아야 한다.

## 결과

### 긍정적 결과

- 문법 production이 실행 가능한 source of truth가 된다.
- 문법 정의 오류와 중의성을 build 또는 가까운 unit test에서 발견한다.
- 기존 동작을 oracle로 유지하면서 production 단위로 안전하게 전환할 수 있다.
- 미래 Locale은 parser infrastructure를 다시 구현하지 않고 자기 scanner adapter와 grammar를 제공할 수 있다.

### 비용

- grammar compiler와 runtime 자체의 테스트·호환성을 유지해야 한다.
- migration 중에는 handwritten parser와 generated parser가 일시적으로 함께 존재한다.
- 정밀한 사용자 진단과 recovery metadata는 production을 옮길 때 별도로 선언해야 한다.

## 비범위

- lexer 또는 형태소 분석기 생성
- 범용 LR, GLR 또는 자연어 parser framework
- formatter와 lowering 자동 생성
- left-recursive expression grammar와 precedence 선언
- executable grammar 도입을 이유로 한 공개 문법 변경

## References

- [Frontend Grammar Implementation Drift](../problems/0003-frontend-grammar-implementation-drift.md)
- [RSPDL Compiler Architecture](../architecture.md)
- [Controlled Korean Surface Grammar](../rfcs/0001-controlled-korean-surface-grammar.md)
