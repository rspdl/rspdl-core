---
id: frontend-semantic-analysis-contract
title: Frontend and Semantic Analysis Contract
type: spec
status: implemented
version: "11"
summary: Defines stable-ID Unlinked records, semantic product value types, action data mutations, relations, rules, and the structured diagnostic boundary shared by frontends.
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
  - finite-relational-model-finding
problem_refs:
  - data-lifecycle-modeling-gap
  - policy-consistency-blind-spots
  - semantic-source-provenance-loss
last_updated: "2026-08-26"
owners:
  - rspdl-maintainers
target_spec: "0.4.0"
---

# Frontend and Semantic Analysis Contract

## 목적

`ko-KR`, 미래 `en-US`와 다른 표면 언어가 서로 다른 표현 형식을 사용하면서도 타입 검사와 의미 규칙을 복제하지 않고 같은 분석 결과를 사용하게 한다.

Frontend는 자기 Locale의 표시 이름을 선언 stable ID에 연결하고 Locale 독립 `UnlinkedModule`로 desugar한다. 공통 analyzer는 모든 frontend output의 stable ID를 검증·연결하고 같은 type checker와 semantic rule을 적용한다. Core는 표시 이름을 참조 해석에 사용하지 않는다.

## Phase contract

```text
Locale Source -> Locale AST -> Stable-ID UnlinkedModule -> Link/Type Check -> SemanticModule -> Analysis
```

- Locale AST는 frontend 내부 타입이며 호환 계약이 아니다.
- `UnlinkedModule`은 선언 ID, 표시 이름, stable-ID `SurfaceRef`, literal, source range와 의미 construct를 보존한다.
- Built-in field type은 base scalar 외에 `Money(currency)`, `Percentage`, `Quantity(unit)`, `Coordinate`, `LocalDateTime`, `ZonedDateTime`, `CalendarDuration`, 문자열 refinement, `List(element)`, `Set(element)`, `Map(key,value)`, `Reference(model)` variant로 lowering한다. Parameter와 nested type은 Unlinked IR에서도 손실 없이 보존한다. Frontend는 reference에 stable ID와 span만 넣으며, common analyzer가 local/fully-qualified target을 동일한 model symbol table에서 resolve하고 존재하지 않는 target을 `RSPDL-LINK-*` 진단으로 거부한다.
- action의 데이터 결과는 action·model `SurfaceRef`, `create|update|delete` mutation과 source range를 가진 `UnlinkedActionDataMutation`으로 보존한다. Linking 뒤에는 `ActionDataMutationDefinition.span`과 기존 `Compilation.action_data_mutation_provenance` sidecar가 같은 UTF-8 byte `TextRange`를 유지한다.
- conditional creation branch는 explicit declaration ID, optional legacy Action `action` reference와 authoritative tagged `trigger`, input/variant/output `SurfaceRef`, `Create|Skip`과 span을 가진 `UnlinkedCreationBranch`로 보존한다. Event branch는 `action`을 직렬화하지 않으며 frontend는 trigger owner의 direct enum input과 그 enum 안의 variant만 표시 이름에서 stable ID로 연결한다.
- Event는 immutable typed payload input을 가진 별도 declaration이며 branch는 tagged `Action|Event` trigger reference를 보존한다. common analyzer는 owner-scoped direct enum payload만 Event creation decision으로 허용한다. Event producer는 Event 전용 source variant와 `TriggerPayload` phase를 가지며 Action-shaped legacy `action` field를 직렬화하지 않는다.
- analyzer는 `(trigger_kind, trigger_id, output_model_id)`별 `ConditionalProductionDefinition`을 만들고 `ExactlyOne` instance cardinality, `decision_input_id`와 canonically sorted branch를 보존한다. Event production은 legacy `action_id`를 직렬화하지 않는다. 모든 production diagnostic은 `trigger_kind`와 `trigger_id`를 가지며 Action finding만 호환 `action_id`를 추가한다. enum coverage/conflict와 Create path required-field producer gap/conflict는 core가 판정한다.
- field producer는 declaration ID, authoritative tagged trigger/output model/output field `SurfaceRef`, optional Action direct enum input+variant condition과 source를 가진 `UnlinkedFieldProducer`로 보존한다. Action source는 legacy `ActionInput|InputField|Constant|Template`와 `PreMutation`을 그대로 유지하고, Event source는 `EventInput|EventInputField|Template`와 immutable `TriggerPayload`를 사용한다. frontend는 display names만 stable IDs로 lower한다. analyzer는 existing production attachment, exact type, trigger owner, condition input이 production `decision_input_id`와 같은 enum이고 variant가 그 enum 소속인지, Create variant별 field cardinality를 판정한다. conditional Event producer, Event constant, general boolean, multi-axis, default/override condition은 지원하지 않는다.
- message template은 위 field producer의 `Template { parts: Text | OutputField(SurfaceRef) }` source다. Korean frontend는 `{output field display name}`을 같은 output model의 stable field ID로 lower하고 `{{`, `}}`를 literal brace로 보존한다. common analyzer는 target/result/placeholder field `String`, same-output-model placeholder link, effective Create variant별 dependency producer cardinality, sorted cycle evidence와 canonical `field_evaluation_order`를 판정한다. template은 암시적 문자열 변환, action input, model/relation path, snapshot, localization/pluralization 또는 channel rendering을 표현하지 않는다.
- relation producer는 declaration ID, tagged trigger/input/output model/relation `SurfaceRef`와 span을 가진 `UnlinkedRelationProducer`로 보존한다. analyzer는 relation linking 뒤 output model이 first endpoint인 binary relation과 Required+Unique constraint를 ExactlyOne output slot으로 도출하고, same-trigger direct ExistingModel input의 endpoint exact match, Action `PreMutation` 또는 Event `TriggerPayload` phase 및 Create variant별 slot cardinality를 판정한다.
- `SemanticModule`은 모든 참조와 타입이 해석된 Canonical IR이며 source-backed record마다 선언 또는 규칙의 `span`을 보존한다. 여러 문장을 병합하는 screen은 최초 문장을, 각 operation은 자기 문장을 가리킨다.
- 재계산 dependency는 기존 `DerivationDefinition.recalculate_when_changed_field_ids`와 함께 source-backed `RecalculationDefinition`으로 보존한다.
- frontend output은 신뢰하지 않는다. 공통 analyzer가 ID 문법, 참조 존재성, 타입과 교차 선언 invariant를 다시 검증한다.

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

pub struct SurfaceRef {
    pub id: String,
    pub span: TextRange,
}

pub struct ActionDataMutationProvenance {
    pub action_id: CanonicalId,
    pub model_id: CanonicalId,
    pub mutation: DataMutationKind,
    pub source_id: SourceId,
    pub span: TextRange,
}

pub struct RecalculationDefinition {
    pub source_field_id: CanonicalId,
    pub target_field_id: CanonicalId,
    pub span: TextRange,
}
```

`rspdl-compiler::compile_with_frontend`, `compile_source_with_frontend`와 `compile_files_with_frontend`는 구체 Locale 타입이 아니라 이 계약을 입력으로 받는다. 문자열만 받는 entry point는 결정적인 `<inline>` source ID를 사용하고, `Source`를 받는 single/workspace entry point는 caller가 제공한 path를 source ID로 보존한다. 성공한 `Compilation`은 `action_data_mutation_provenance[*].source_id`와 `span`으로 provenance를 노출한다.

`Frontend::language_id`는 미래 Locale registry와 artifact provenance를 위한 식별 hook이다. 현재 compiler entry point는 호출자가 frontend 구현을 직접 주입하므로 compilation artifact나 진단에 이 값을 복사하지 않는다.

`SurfaceRef.id`는 module-local ID 또는 fully-qualified ID다. 표시 이름, 번역 문자열 또는 source 문장 조각을 넣을 수 없다. 공통 analyzer는 local ID를 현재 module로 한정하고 fully-qualified ID는 그대로 검증한다. Bare local reference가 둘 이상의 qualified declaration suffix와 일치하면 첫 선언을 선택하지 않고 ambiguity 진단을 반환한다.

## Frontend responsibility

Frontend가 소유한다.

- scanner, parser와 Locale AST
- syntax recovery와 Locale surface lint
- 표면 문형을 공통 의미 construct로 desugar
- 행동 결과 문형을 stable-ID action·model reference와 Locale 독립 mutation kind로 desugar
- 하나 이상의 field를 가진 Locale record 선언을 `UnlinkedDataModel`로 desugar
- Locale 표시 이름 reference를 같은 source에 선언된 stable ID로 연결
- 모든 declaration과 reference의 source range 보존
- 표시 이름과 일치하는 선언이 없거나 둘 이상의 선언과 일치하면 Locale reference 진단을 반환하고 module 생성을 중단

Frontend가 소유하지 않는다.

- stable ID 문법 검증, module qualification과 중복 판정
- stable ID 기반 symbol resolution
- field, enum, constraint와 policy type checking
- 확장 scalar literal의 canonical validation, ordered-type capability 검사와 위도·경도 범위 검사
- relation parameter, cardinality와 compatibility group 검증
- producer/consumer graph와 lifecycle 분석
- 동일 action·model의 중복 또는 상충하는 data mutation 판정
- policy consistency 분석
- Canonical internal constraint, policy 또는 relation meta-rule ID 생성
- `RSPDL-LINK-*`, `RSPDL-TYPE-*`, `RSPDL-DATA-*`, `RSPDL-REL-*` 의미 진단

## Analyzer responsibility

공통 analyzer는 `UnlinkedModule`을 입력으로 다음 순서를 적용한다.

1. declaration과 reference의 stable ID를 검증하고 module scope로 한정한다.
2. reference stable ID를 정확히 하나의 선언과 연결한다.
3. enum, field, constraint, policy와 relation signature를 검사한다.
   빈 `UnlinkedDataModel`은 `RSPDL-DATA-007`로 거부하고, relation cardinality 규칙에 명시된 anchor model이 relation의 첫 parameter와 같은지 검사한다.
4. 해석된 의미만 사용해 anonymous rule ID를 생성한다.
5. Canonical `SemanticModule`을 구성하고 source-backed record의 span을 semantic identity와 분리해 보존한다.
6. data lifecycle, 동일 action 결과의 mutation compatibility, relation compatibility와 policy 의미 규칙을 실행한다.

오류가 있으면 부분 `SemanticModule`을 성공으로 반환하지 않으며 structured diagnostic을 반환한다.

## Diagnostic contract

Core와 frontend가 교환하는 진단은 표시 문장이 아닌 다음 구조를 사용한다.

```rust
pub struct Diagnostic {
    pub rule_id: String,
    pub severity: Severity,
    pub message_key: String,
    pub arguments: BTreeMap<String, String>,
    pub span: TextRange,
}
```

Runtime input과 backend 실행 진단은 source `span` 대신 JSON `path`를 갖는 `RuntimeDiagnostic`을 사용하되, 동일하게 `rule_id`, `severity`, `message_key`와 정렬된 `arguments`만 저장한다.

- `rule_id`는 의미 규칙의 안정적인 식별자다.
- `message_key`와 `arguments`는 기계 비교와 Locale rendering의 입력이다.
- `arguments`는 직렬화와 비교 결과가 결정적이도록 key 순서가 정렬된다.
- 사람이 읽는 한국어·영어 문장은 core diagnostic에 저장하지 않는다.
- Locale crate, CLI 또는 application이 최종 사용자 Locale에 맞춰 rendering한다.
- JSON compiler output은 rendering 전 구조화 진단을 그대로 반환한다.

## Canonical generated IDs

Constraint, policy, conditional production과 relation meta-rule의 anonymous ID는 Locale display text나 source 위치를 사용하지 않는다. Analyzer가 frontend에서 받은 reference stable ID를 Canonical ID로 연결한 뒤 다음 semantic identity에 FNV-1a 64-bit를 적용한다.

- constraint: `model-id NUL operand NUL operator NUL operand`
- policy: `role-id NUL model-id NUL field-id NUL action-id NUL effect`
- conditional production: Action은 `action-id NUL output-model-id`, Event는 `event NUL event-id NUL output-model-id`
- relation meta-rule: normalized kind 뒤에 model ID 하나 또는 정렬·중복 제거된 relation ID 목록

따라서 같은 stable ID와 의미를 사용하는 서로 다른 Locale frontend는 같은 anonymous ID를 만든다.

## Conformance evidence

- frontend unit test는 source reference가 expected stable-ID `SurfaceRef`로 lowering되는지 검증한다.
- analyzer test는 Locale source 없이 hand-authored `UnlinkedModule`만 사용한다.
- compiler conformance는 action mutation의 `SourceId`와 UTF-8 byte `TextRange`가 `Compilation`까지 보존되는지 검사한다.
- conditional-production conformance는 normal/failure/boundary/false-positive Create/Skip cases, exact structured diagnostics와 source-order 독립 semantic projection을 검사한다.
- field-producer conformance는 세 source form, 무조건/enum-variant 조건 IR projection, variant별 missing/type/duplicate payload diagnostics, explicit `0`/`false`/empty string 및 source-order 독립성을 검사한다.
- message-template conformance는 output-only normal case, brace escape, syntax/link/type failure, optional dependency gap, direct+template conflict, self/multi-node cycle, All-Skip, unrelated-field false positive과 canonical dependency/evaluation projection을 검사한다.
- source provenance conformance는 각 record의 span으로 원문을 UTF-8 slice할 수 있는지, multi-file span이 containing file 기준인지, 위치 변화가 generated ID를 바꾸지 않는지 검사한다.
- 동일한 stable ID와 의미를 가진 Locale별 fixture는 Canonical ID, semantic result와 `rule_id`, `message_key`, `arguments`가 같아야 한다.
- 정상, 실패, 경계와 오탐 방지 fixture는 공통 analyzer를 통해 실행한다.
- `rspdl-domain`과 solver backend는 Locale crate를 의존할 수 없다. Compiler의 runtime matcher는 Locale AST나 표시 이름이 아니라 해석된 `SemanticModule`만 검사해야 한다.

## 초기 버전 호환 정책

이 계약은 `0.x` 초기 버전이며 아직 안정화된 외부 구현 호환성을 약속하지 않는다. 이전의 rendered `Diagnostic.message` 또는 표시 이름 기반 `SurfaceRef`를 위한 adapter, deprecated constructor와 migration layer를 두지 않는다. 새 frontend는 이 문서의 stable-ID reference와 structured diagnostic 형식을 직접 구현해야 한다.

## 현재 비범위

- 하나의 workspace에서 source별로 서로 다른 Locale을 자동 선택하는 registry
- module import와 cross-module symbol resolution
- 외부 process 또는 stable Rust ABI plugin protocol
- 완전한 `SemanticGraph`와 impact analysis
