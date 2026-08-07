---
id: frontend-semantic-analysis-contract
title: Frontend and Semantic Analysis Contract
type: spec
status: implemented
version: "1"
summary: Defines the locale-neutral Unlinked IR boundary that lets independent surface-language frontends use one linker and semantic analyzer.
topics:
  - compiler-frontend
  - unlinked-ir
  - semantic-analysis
  - locale-independence
  - conformance
related:
  - rust-korean-first-frontend
  - rspdl-compiler-architecture
  - natural-korean-domain-grammar
problem_refs:
  - data-lifecycle-modeling-gap
  - policy-consistency-blind-spots
last_updated: "2026-08-06"
owners:
  - rspdl-maintainers
target_spec: "0.2.0"
---

# Frontend and Semantic Analysis Contract

## 목적

`ko-KR`, 미래 `en-US`와 다른 표면 언어가 이름 해석, 타입 검사와 의미 규칙을 복제하지 않고 같은 분석 결과를 사용하게 한다.

Frontend는 표면 표현을 Locale 독립 `UnlinkedModule`로 desugar한다. 공통 analyzer는 모든 frontend output에 같은 linker, type checker와 semantic rule을 적용한다.

## Phase contract

```text
Locale Source -> Locale AST -> UnlinkedModule -> Link/Type Check -> SemanticModule -> Analysis
```

- Locale AST는 frontend 내부 타입이며 호환 계약이 아니다.
- `UnlinkedModule`은 선언 ID, 표시 이름, symbolic `SurfaceRef`, literal, source range와 의미 construct를 보존한다.
- `SemanticModule`은 모든 참조와 타입이 해석된 Canonical IR이다.
- frontend output은 신뢰하지 않는다. 공통 analyzer가 ID, 참조, 타입과 교차 선언 invariant를 다시 검증한다.

## Rust interface

모든 in-process frontend는 다음 동작 계약을 구현한다.

```rust
pub trait Frontend {
    fn language_id(&self) -> &'static str;
    fn lower_source(&self, source: &str) -> FrontendOutput;
}

pub struct FrontendOutput {
    pub module: Option<UnlinkedModule>,
    pub diagnostics: Vec<Diagnostic>,
}
```

`rspdl-compiler::compile_with_frontend`와 `compile_files_with_frontend`는 구체 Locale 타입이 아니라 이 계약을 입력으로 받는다.

## Frontend responsibility

Frontend가 소유한다.

- scanner, parser와 Locale AST
- syntax recovery와 Locale surface lint
- 표면 문형을 공통 의미 construct로 desugar
- 모든 declaration과 reference의 source range 보존

Frontend가 소유하지 않는다.

- stable ID qualification과 중복 판정
- 표시 이름 또는 ID 기반 symbol resolution
- field, enum, constraint와 policy type checking
- producer/consumer graph와 lifecycle 분석
- policy consistency 분석
- Canonical internal constraint 또는 policy ID 생성
- `RSPDL-LINK-*`, `RSPDL-TYPE-*`, `RSPDL-DATA-*` 의미 진단

## Analyzer responsibility

공통 analyzer는 `UnlinkedModule`을 입력으로 다음 순서를 적용한다.

1. declaration ID를 검증하고 module scope로 한정한다.
2. 표시 이름 reference를 stable Canonical ID로 연결한다.
3. enum, field, constraint와 policy 타입을 검사한다.
4. 해석된 의미만 사용해 anonymous rule ID를 생성한다.
5. Canonical `SemanticModule`을 구성한다.
6. data lifecycle과 policy 의미 규칙을 실행한다.

오류가 있으면 부분 `SemanticModule`을 성공으로 반환하지 않으며 structured diagnostic을 반환한다.

## Canonical generated IDs

Constraint와 policy의 anonymous ID는 Locale display text나 source 위치를 사용하지 않는다. Linker가 reference를 stable Canonical ID로 해석한 뒤 다음 semantic identity에 FNV-1a 64-bit를 적용한다.

- constraint: `model-id NUL operand NUL operator NUL operand`
- policy: `role-id NUL model-id NUL field-id NUL action-id NUL effect`

따라서 같은 stable ID와 의미를 사용하는 서로 다른 Locale frontend는 같은 anonymous ID를 만든다.

## Conformance evidence

- frontend unit test는 source가 expected `UnlinkedModule` construct와 `SurfaceRef`로 lowering되는지 검증한다.
- analyzer test는 Locale source 없이 hand-authored `UnlinkedModule`만 사용한다.
- 동일한 stable ID와 의미를 가진 Locale별 fixture는 Canonical ID, semantic result와 진단 Rule ID가 같아야 한다.
- 정상, 실패, 경계와 오탐 방지 fixture는 공통 analyzer를 통해 실행한다.
- `rspdl-domain`, Datalog와 solver backend는 Locale crate를 의존할 수 없다.

## 현재 비범위

- 하나의 workspace에서 source별로 서로 다른 Locale을 자동 선택하는 registry
- module import와 cross-module symbol resolution
- 외부 process 또는 stable Rust ABI plugin protocol
- 완전한 `SemanticGraph`와 impact analysis
