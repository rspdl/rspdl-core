---
id: rspdl-language-prd
title: RSPDL Product Requirements
type: prd
status: draft
created: 2026-07-26
version: "1.2"
summary: Defines the product and language requirements for turning explicit planning intent into deterministic, explainable implementation context.
topics:
  - language-design
  - data-lifecycle
  - policy-analysis
  - semantic-ir
  - diagnostics
  - conformance
related:
  - rspdl-product-vision
  - rspdl-compiler-architecture
  - problem-driven-development
  - field-provenance-and-sum-derivation
  - total-policy-condition-space-analysis
  - finite-relational-model-finding
  - conditional-data-production
problem_refs:
  - data-lifecycle-modeling-gap
  - policy-consistency-blind-spots
  - semantic-source-provenance-loss
last_updated: "2026-08-25"
owners:
  - rspdl-maintainers
target_spec: "0.3.0"
---

# RSPDL Product Requirements

## Why

- RSPDL은 기획자와 기획까지 맡은 개발자가 구현 전에 데이터와 정책의 빈틈을 발견하도록 돕는다.
- 정책 검토를 개인의 꼼꼼함과 개발 중 조건식 작성에 의존하면 결정 대기와 재작업이 반복된다.
- 자연어 문서는 데이터의 존재 시점, 조건 공간, 교차 참조와 모순을 결정적으로 검증하기 어렵다.
- 사람과 AI 에이전트가 같은 명시적 의도를 공유하려면 안정적인 ID와 기계 검증 가능한 의미 모델이 필요하다.
- 제품의 북극성과 사용자 약속은 [RSPDL Product Vision](product/vision.md)을 따른다.

## What

- RSPDL은 제품 기획의 데이터 모델, lifecycle, 권한, 정책과 유저 플로우를 표현하는 선언형 언어다.
- 사람과 AI가 문서를 작성할 수 있지만 의미와 유효성은 명세된 결정론적 규칙으로 판정한다.
- 핵심 사용자는 전담 기획자, 여러 역할을 동시에 수행하는 기획자와 기획 결정을 함께 맡은 개발자다.
- 핵심 결과는 다음과 같다.
  - 구현 가능한 결정과 명시적인 미결정 목록
  - Locale 독립 Canonical Semantic IR과 Semantic Graph
  - 원문 위치, 근거와 반례를 포함한 구조화된 진단
  - 변경이 영향을 주는 데이터, 정책, 플로우와 downstream consumer 목록
- 명시된 의도는 하나의 canonical context로 보존한다.
- 명시되지 않은 의도는 추측하지 않으며 현실 요구사항과의 100% 일치를 보장하지 않는다.

## How

- 제품 루프는 `작성 → 정규화 → 검증 → 결정 → 구현 전달 → 사용자 피드백`이다.
- 의미 모델 요구사항은 다음과 같다.
  - `INTENT-001`: 모든 선언과 참조는 번역 가능한 표시 이름과 안정적인 machine ID를 분리한다.
  - `INTENT-002`: 권한, 데이터, 정책과 플로우를 하나의 Semantic Graph에서 연결한다.
  - `INTENT-003`: data, role, resource, action, predicate와 effect를 포함한 모든 의미 vocabulary는 typed stable ID로 먼저 선언하며, frontend나 analyzer가 알려지지 않은 단어를 새 의미로 추측하지 않는다.
  - `INTENT-004`: source-backed Canonical Semantic IR record는 해당 선언을 다시 찾을 수 있는 UTF-8 byte source span을 보존하며, 위치를 semantic identity, generated ID·semantic hash, duplicate key 또는 의미 정렬 근거로 사용하지 않는다.
  - `DATA-001`: 데이터의 생성, 조회, 수정, 삭제와 파생 연산을 존재 상태 및 전이와 연결할 수 있어야 한다.
  - `DATA-002`: 생성 전 사용, 삭제 후 사용, 끊어진 참조와 가용하지 않은 입력의 파생을 진단해야 한다.
  - `DATA-003`: record model을 entity sort로 사용하는 typed relation과 endpoint 참조 무결성을 표현하고, 실제 record 없이 bounded virtual model을 탐색할 수 있어야 한다.
  - `DATA-004`: relation의 nonempty, required, unique, exclusive, exhaustive와 compatible coexistence 의도는 명시적으로 Canonical IR에 보존하며 solver가 암묵적으로 추론하지 않는다.
  - `POLICY-001`: actor 또는 role, resource, action, condition, effect와 적용 범위를 표현해야 한다.
  - `POLICY-002`: conflict, gap, overlap과 unreachable을 서로 다른 결과로 분석해야 한다.
  - `POLICY-003`: decision point의 totality, default와 override를 Canonical IR에 명시적으로 보존하고, 누락 branch를 의도된 partial policy나 암묵적 우선순위로 해석하지 않는다.
  - `POLICY-004`: 초기 조건부 decision point는 선언된 유효 입력 domain 전체를 명시적 branch, default 또는 no-op 결과로 덮어야 하며, 조건의 반대 영역을 생략한 채 의도된 미정의로 간주할 수 없다.
  - `POLICY-005`: 상태별 조건부 invariant와 겹치는 branch 사이의 priority를 구분하고, source 순서·조건의 겉보기 구체성·effect 이름에서 priority를 추론하지 않는다.
  - `POLICY-006`: overlap은 effect compatibility와 분리해 판정하며, 배타적 decision slot, post-state와 cross-field·lifecycle invariant로 동작 충돌의 근거를 표현해야 한다.
  - `PROD-001`: 행동 또는 명시된 사건의 조건에 따라 typed output record를 생성하거나 명시적으로 생성하지 않는 conditional data production을 표현해야 한다. 이는 권한 effect의 임의 확장이 아니라 output instance와 field producer를 갖는 별도 의미 모델이다.
  - `PROD-002`: 필수 output field와 relation slot은 모든 effective creation path에서 action input, relation path, snapshot, constant 또는 지원되는 expression 중 정확히 하나의 typed provenance producer를 가져야 한다. producer 없음은 gap이고 양립 불가능한 복수 producer는 conflict다.
  - `PROD-003`: template은 output record의 선언된 field만 placeholder로 참조해야 하며, 원본 model path나 입력받지 않은 값을 직접 참조할 수 없다.
  - `PROD-004`: output provenance는 source span, typed path와 lifecycle phase를 보존해야 하며, 생성 전·삭제 후 source를 payload에 쓰려면 explicit snapshot 또는 retain provenance가 있어야 한다.
  - `PROD-005`: 알림의 발신자·수신자·대상은 typed output relation slot으로, 메시지와 가격의 금액·통화·할인은 typed output field로 표현하며 relation cardinality, field composition, rounding과 external source는 선언된 계약 없이 추론하지 않는다.
- 언어와 호환성 요구사항은 다음과 같다.
  - `SYNTAX-001`: 초기 문법은 자유 자연어가 아닌 구조화된 블록 형식이어야 한다.
  - `SYNTAX-002`: `@` annotation은 domain 의미가 아닌 문서 수준 metadata에만 허용한다. 현재 허용 목록은 문서의 module identity를 선언하는 `@모듈` 하나다.
  - `SYNTAX-003`: 새 `@` annotation은 문장이나 블록으로 의미를 결정적이고 읽기 쉽게 보존할 수 없다는 RFC 근거가 있을 때만 추가할 수 있다. 짧은 구현, parser 편의, 입력 길이와 기존 annotation 선례는 근거가 될 수 없다.
  - `SYNTAX-004`: 독립된 선언과 규칙은 그 문장만 읽어도 대상, 관계 방향과 cardinality 또는 compatibility 의도를 식별할 수 있어야 한다. 이 정보가 IR이나 별도 문서를 봐야만 드러나는 표면 문법은 허용하지 않는다.
  - `DATA-SHAPE-001`: record model은 하나 이상의 명시적 field를 가져야 한다. 추상 sort가 필요해지면 빈 record로 우회하지 않고 별도 제품 시나리오와 RFC에서 독립 construct로 설계한다.
  - `LOCALE-001`: 같은 의미의 Locale 문서는 정규화 후 동일한 Canonical IR을 생성해야 한다.
  - `LOCALE-002`: Locale frontend는 표시 이름을 stable ID로 연결한 공통 Unlinked IR을 만들고, stable ID validation·linking, type checking과 의미 규칙은 공통 analyzer가 한 번만 구현해야 한다.
  - `MODULE-001`: 여러 문서와 Locale에 걸쳐 심볼을 선언, 참조하고 연결할 수 있어야 한다.
  - `VERSION-001`: source는 사용한 언어 명세와 필요한 의미 규칙 버전을 선언할 수 있어야 한다.
  - `COMPAT-001`: 호환 구현체는 구현 독립 Conformance Test Suite로 의미 동등성을 증명해야 한다.
- 진단과 영향 분석 요구사항은 다음과 같다.
  - `DIAG-001`: 진단은 Rule ID, severity, message key, source span, 관련 심볼과 evidence를 제공해야 한다.
  - `DIAG-002`: 지원하지 않는 의미와 solver timeout은 성공으로 근사하지 않고 `unknown`으로 반환해야 한다.
  - `DIAG-003`: 정적 policy finding은 선언된 유효 domain 안의 canonical witness를 포함하고, 유한 enum gap은 누락 variant 또는 동등한 compact region을 결정적인 순서로 제공해야 한다.
  - `IMPACT-001`: stable ID를 기준으로 한 변경의 direct 및 transitive semantic dependency를 찾을 수 있어야 한다.
  - `IMPACT-002`: 같은 source와 spec version은 같은 IR, 진단, evidence 순서를 생성해야 한다.
- 공개 의미 규칙의 증명 요구사항은 다음과 같다.
  - 정상 사례
  - 실패 사례
  - 경계 사례
  - 오류와 유사하지만 허용해야 하는 오탐 방지 사례
  - 입력 순서와 반복 실행이 결과를 바꾸지 않는 결정론 사례
- 현재 구현 범위는 다음과 같다.
  - 한국어 module, enum, record field, field constraint, role, action과 조건 없는 allow 또는 deny policy
  - 문장형 화면 create/read/update/delete와 field input/read/update 선언
  - 문장형 action create/update/delete 결과와 동일 action·model의 mutation conflict 검증
  - stable-ID typed action input의 한국어 문형, common linking/type checking, source-backed Canonical IR과 결정적 JSON 직렬화
  - direct enum action 또는 immutable Event payload의 한국어 conditional ExactlyOne Create/Skip, enum coverage와 same-variant conflict, Action direct input·ExistingModel input field·constant의 `PreMutation` producer 및 Event direct value·ExistingModel input field의 `TriggerPayload` producer, 무조건 output-field-only message template와 variant별 required output field gap/conflict
  - output-first binary relation에 Required와 Unique가 함께 있으면 ExactlyOne output relation slot으로 해석하고 Action 또는 Event direct ExistingModel input relation producer의 Create-path gap/conflict 검증
  - 화면 입력과 합계 계산을 생산자로 연결한 field provenance 검증
  - 단일 정수 필드 합계, 원본 변경 시 재계산과 내부/비표시 의도
  - parser, formatter, Canonical domain model, Z3 constraint check와 결정적 직접 runtime policy match
  - 단일 닫힌 enum decision point의 backend-neutral 정적 gap, compatible overlap 및 allow/deny conflict 분석 API와 Z3 witness
  - 하나 이상의 field를 가진 record model, 문장형 unary/binary typed relation과 `nonempty`, `required`, `unique`, `exclusive`, `exhaustive`, compatible `coexistent` 규칙
  - 모델별 finite scope에서 typed attribute constraint, endpoint integrity와 relation meta-rule을 grounding하는 bounded model finder, virtual entity/field/relation witness와 bound 한정 UNSAT rule evidence
  - 공통 `Frontend` trait, stable-ID Unlinked IR과 Locale 독립 linking, type checking 및 data usage analyzer
  - module, 선언, 규칙, screen operation과 recalculation을 포함한 source-backed Semantic IR record의 file-relative source span
  - runtime request별 `allowed`, `denied`, `conflict`, `unmatched` 분류
- 아직 구현하지 않은 요구사항은 다음과 같다.
  - 화면 간 순서·분기, 삭제 이후 접근과 path별 데이터 availability
  - relation/join 기반 교차 모델 집계 실행과 일반 계산식
  - 조건부 정책의 한국어 표면 문법, Canonical IR lowering과 compiler structured diagnostic 연결
  - 다중 입력 domain과 일반 effect compatibility, 조건부 field requiredness, explicit default와 override
  - snapshot/retain lifecycle analysis, 조건부 Event field/relation producer와 structured diagnostic 확장
  - relation path 기반 다수 output 생성, 실제 relation JSON binding, output delivery, 일반 expression·통화·반올림·가격표 snapshot과 field composition
  - effective condition에 기반한 unreachable 분석
  - 유저 플로우, 컬렉션, module import와 다국어 의미 동등성
  - 3항 이상 관계, 임의의 quantified formula, 실제 JSON relation binding, relation join/projection/aggregation과 CRUD transition
  - semantic dependency 기반 영향 분석과 downstream code generation
- 성공 기준은 다음과 같다.
  - 대표 시나리오에서 데이터 lifecycle과 정책 사각지대를 구현 전에 재현 가능한 evidence로 찾는다.
  - 모든 공개 규칙에 정상, 실패, 경계와 오탐 방지 fixture가 존재한다.
  - 모든 호환 구현체와 Locale이 동일한 의미 결과를 생성한다.
  - 한 source 변경의 영향을 stable ID로 추적하고 소비자가 필요한 context만 선택할 수 있다.

## Constraints

- AI 출력도 사람의 출력과 같은 parser, semantic analysis와 conformance gate를 통과한다.
- compiler는 문서에 없는 사실, 정책 우선순위 또는 lifecycle 동작을 추측하지 않는다.
- 정적 조건 공간 분석은 typed SMT query를 기준 경로로 삼는다. 현재 runtime policy match는 선언된 무조건 allow/deny 정책을 직접 결정적으로 대조한다. Datalog evaluator는 제거했으며, 재귀적 관계 폐쇄가 필요해질 때에만 별도 RFC로 다시 검토한다.
- `push/pop`과 assumption switch 같은 증분 solver 전략은 관찰 가능한 의미, witness와 진단 순서를 바꿀 수 없다.
- RSPDL core는 Canonical IR, semantic analysis와 diagnostics를 소유한다.
- 정책표, IA, UI projection, 검색, 집계와 code generation은 공개 IR을 소비하는 application 책임이다.
- 자유 형식 자연어 직접 해석과 특정 제품 UI는 초기 언어 범위에 포함하지 않는다.
- 릴리스 명세는 SemVer를 따르고 승인된 의미 변경은 RFC와 conformance fixture를 함께 요구한다.
- 표현 편의보다 해석의 단일성, 오류의 조기 발견과 설명 가능성을 우선한다.
- Readability는 formatter 취향이나 lint가 아니라 표면 문법의 correctness gate다. Domain 선언을 annotation 목록으로 축약하는 변경은 parser가 결정적으로 해석할 수 있더라도 승인하지 않는다.

## References

- [RSPDL Product Vision](product/vision.md)
- [Data Lifecycle Modeling Gap](problems/0001-data-lifecycle-modeling-gap.md)
- [Policy Consistency Blind Spots](problems/0002-policy-consistency-blind-spots.md)
- [RSPDL Compiler Architecture](architecture.md)
- [Core and Application Projection Boundary](adr/0002-core-application-boundary.md)
- [Korean Domain Frontend Language Specification](rfcs/0004-natural-korean-domain-grammar.md)
- [Field Provenance, Screen Usage, Action Data Mutations, and Sum Derivation Grammar](rfcs/0005-field-provenance-and-sum-derivation.md)
- [Total Policy Condition Spaces and SMT-First Consistency Analysis](rfcs/0006-total-policy-condition-space-analysis.md)
- [Conditional Data Production for Notifications and Prices](rfcs/0008-conditional-data-production.md)
