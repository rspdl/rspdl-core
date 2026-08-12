---
id: finite-relational-model-finding
title: Finite Relational Rules and Bounded Model Finding
type: rfc
status: implemented
version: "1"
summary: Defines unary and binary relations, explicit relational meta-rules, and bounded virtual-data model finding without runtime records.
topics:
  - first-order-logic
  - relation
  - bounded-model-finding
  - cardinality
  - counterexample
related:
  - typed-domains-and-logic-core
  - natural-korean-domain-grammar
  - rspdl-compiler-architecture
  - total-policy-condition-space-analysis
problem_refs:
  - data-lifecycle-modeling-gap
  - policy-consistency-blind-spots
last_updated: "2026-08-12"
owners:
  - rspdl-maintainers
target_spec: "0.3.0"
---

# Finite Relational Rules and Bounded Model Finding

## 상태와 목적

이 RFC는 구현된 첫 relational vertical slice를 정의한다. RSPDL의 record model은 실제 JSON record가 없어도 bounded analysis 안에서 논리적 개체 sort로 사용될 수 있다. Relation은 개체 tuple에서 Boolean으로 가는 predicate이고, 메타 규칙은 relation이 만족해야 하는 1차 논리 불변식이다.

이 slice의 목적은 데이터를 생성하거나 세는 것이 아니다. Solver가 선언을 시험할 유한한 가상 세계를 가정하고 `SAT` witness, bound 안의 `UNSAT`, `UNKNOWN`을 구분하게 하는 것이다.

## 실패 시나리오와 원인

기존 field constraint는 주어진 runtime record 하나의 값을 검사했다. 따라서 다음 질문에는 답할 수 없었다.

- 프로젝트마다 Owner가 존재할 수 있는가?
- 같은 프로젝트의 Owner가 둘일 수 있는가?
- 두 분류 relation은 겹치면 안 되는가, 겹쳐도 되는가?
- 실제 record가 하나도 없을 때 전체 선언이 함께 만족 가능한가?

원인은 record 인스턴스를 논리 대상으로 다루는 relation, 존재 조건과 전역 불변식이 Canonical IR에 없었던 것이다. 문자열 owner ID field나 구체 JSON fixture는 이 공백의 대체물이 아니다.

## Canonical 의미 모델

`M`은 record model이 정의하는 entity sort이고 `R`은 하나 또는 두 model을 parameter로 갖는 relation이다.

```text
RelationDefinition {
    id
    parameter_model_ids: [M] | [M1, M2]
}
```

이항 관계 `Owner(Project, User)`에서 첫 parameter `Project`는 `required`와 `unique`의 anchor다. Parameter 순서는 의미에 참여한다.

현재 entity sort는 하나 이상의 field를 가진 record model에서만 생긴다.

```rspdl
프로젝트(project)는 다음 필드들로 구성되어 있다.
    이름(name): 필수 문자열

사용자(user)는 다음 필드들로 구성되어 있다.
    이름(name): 필수 문자열
```

빈 `DataModelDefinition`은 공통 analyzer가 `RSPDL-DATA-007`로 거부한다. 추상 sort가 실제 제품 시나리오에 필요해지면 빈 record로 우회하지 않고 record와 구분되는 독립 construct, 표면 문법과 runtime 의미를 별도 RFC로 정한다.

모든 참인 relation tuple은 존재하는 endpoint만 참조해야 한다.

\[
R(x_1,\ldots,x_n) \Rightarrow
Exists_{M_1}(x_1)\land\cdots\land Exists_{M_n}(x_n)
\]

이는 현재 create/delete transition을 구현한다는 뜻이 아니다. 가상 모델 안의 참조 무결성만 정의한다.

## 명시적 메타 규칙

Solver는 어떤 overlap이 오류인지 추측하지 않는다. 다음 선언만 전체 이론 `T`에 들어간다.

### `nonempty M`

\[
\exists x.\ Exists_M(x)
\]

선언이 없으면 model의 empty interpretation을 허용한다.

### `required R`

이항 `R(A, B)`에 대해:

\[
\forall a.\ Exists_A(a)\Rightarrow
\exists b.\ R(a,b)
\]

### `unique R`

이항 `R(A, B)`에 대해:

\[
\forall a,b_1,b_2.\
R(a,b_1)\land R(a,b_2)\Rightarrow b_1=b_2
\]

`required`와 `unique`를 함께 선언하면 anchor마다 정확히 하나의 target이 존재한다.

표면 문장은 anchor model을 relation 이름과 함께 직접 적는다. 공통 analyzer는 이 model이 relation의 첫 parameter와 같은지 검사하므로, 문장이 읽히는 방식과 Canonical relation 방향이 어긋날 수 없다.

### `exclusive {R1, ..., Rn}`

같은 signature의 relation들은 같은 tuple에서 둘 이상 참일 수 없다.

\[
\forall \bar{x}.\ \bigwedge_{i<j}\neg(R_i(\bar{x})\land R_j(\bar{x}))
\]

### `exhaustive {R1, ..., Rn}`

존재하는 endpoint의 모든 유효 tuple은 relation 중 적어도 하나에 속한다.

\[
\forall \bar{x}.\
ValidEndpoints(\bar{x})\Rightarrow
R_1(\bar{x})\lor\cdots\lor R_n(\bar{x})
\]

unary 분류에서 `exclusive`와 `exhaustive`를 함께 쓰면 각 entity가 정확히 한 분류에 속한다. Binary exhaustive는 존재하는 두 sort의 Cartesian product 전체를 분류한다는 강한 의미이므로 작성자가 명시할 때만 사용한다.

### `coexistent {R1, ..., Rn}`

같은 tuple에서 relation들이 함께 참이어도 제품상 compatible하다는 의도를 보존한다. 이는 overlap의 존재를 강제하지 않는다. 같은 relation pair가 `exclusive`와 `coexistent` group에 각각 포함되면 그룹 크기가 달라도 `RSPDL-REL-004` 오류다. 아무 선언도 없으면 analyzer는 overlap을 conflict 또는 compatible overlap으로 추측하지 않는다.

## 한국어 표면 문법

```rspdl
프로젝트(project)는 다음 필드들로 구성되어 있다.
    이름(name): 필수 문자열

사용자(user)는 다음 필드들로 구성되어 있다.
    이름(name): 필수 문자열

프로젝트는 사용자를 소유자(owner)로 가질 수 있다.
사용자는 내부 사용자(internal)에 해당할 수 있다.
사용자는 외부 사용자(external)에 해당할 수 있다.

프로젝트는 하나 이상 존재해야 한다.
모든 프로젝트는 소유자를 하나 이상 가져야 한다.
각 프로젝트는 소유자를 최대 하나만 가질 수 있다.
내부 사용자, 외부 사용자 중 둘 이상은 동시에 성립할 수 없다.
내부 사용자, 외부 사용자 중 하나 이상은 항상 성립해야 한다.
소유자, 검토자는 동시에 성립할 수 있다.
```

관계와 meta-rule의 한국어 문형은 `rspdl-ko`에만 존재한다. `@`는 이 문법에 사용하지 않는다. Frontend는 표시 이름을 stable ID reference로 낮추고, 공통 analyzer가 arity, anchor, signature와 compatibility를 검사한다. Meta-rule ID는 정규화된 kind와 관련 stable ID에서 생성되며 표시 이름, source 위치와 목록 입력 순서에 의존하지 않는다.

## Bounded grounding

`rspdl model <file> --scope N`은 각 model에 최대 `N`개의 가상 slot을 만든다. Slot의 실제 존재 여부와 각 relation tuple의 참/거짓은 Boolean solver 변수다.

```text
exists(M, i)          : Bool
tuple(R, i1, ..., in) : Bool
```

Attribute는 entity slot마다 typed 함수값 variable로 grounding한다. 필수 field는 존재하는 entity의 witness에 항상 값을 가지며, 선택 field는 별도 presence Boolean을 갖는다.

```text
value(field, i)   : field.type
present(field, i) : Bool  # optional only
```

기존 record constraint는 entity가 존재하고 constraint가 참조하는 모든 선택 field가 present일 때 적용한다. 따라서 `nonempty Item`, `value > 0`, `value < 0`은 함께 `UNSAT_WITHIN_BOUND`이고, 같은 두 제약이 선택 field에만 걸리면 field absence를 선택하는 `SAT` 모델이 가능하다. 이는 runtime check의 “선택 field가 없으면 해당 constraint를 적용하지 않음”과 같다.

유한 scope 위의 `forall`은 conjunction으로, `exists`는 disjunction으로 grounding한다. 따라서 이번 slice는 새롭고 불완전한 predicate 해석을 Z3에 위임하지 않고 기존 typed Boolean IR과 Solver 계약을 재사용한다.

`scope`는 모델 선언의 일부가 아니라 분석 command의 경계다. `N = 0`은 허용하지 않는다. 현재 eager grounding 구현은 모델별 scope `1..=32`를 지원하며 이를 넘으면 `RSPDL-MODEL-001` configuration error다. 이는 제품 세계의 최대 크기에 대한 의미 선언이 아니라 grounding 자원 사용을 제한하는 구현 capability다.

## 결과와 증거 계약

- `SAT`: scope 안에서 지원되는 data rule을 만족하는 가상 entity, field value와 relation tuple witness를 반환한다.
- `UNSAT_WITHIN_BOUND`: 해당 scope 안에 모델이 없음을 뜻하며 전역 `UNSAT`으로 표현하지 않는다. Endpoint 무결성과 함께 여전히 모순인 deletion-minimal stable Rule ID 집합을 반환한다.
- `UNKNOWN`: `RSPDL-MODEL-004`, `model_finding.unknown`과 timeout 이유를 반환하고 성공으로 근사하지 않는다.
- backend/configuration error: 별도 stable Rule ID와 message key로 반환한다.
- `Unsupported`: `RSPDL-MODEL-003`, `model_finding.unsupported_construct`와 함께 sum derivation처럼 현재 가상 세계에 정확히 연결할 수 없는 data construct를 나열하고 `SAT`으로 근사하지 않는다.

Solver가 선택한 가상 atom은 실제 record ID, default 데이터 또는 유일한 해법이 아니다.

## 적합성 사례

### 정상

- nonempty Project와 required+unique Owner가 scope 안에서 정확히 한 Owner tuple을 갖는 witness를 만든다.
- unary relation group의 exclusive+exhaustive가 존재하는 entity를 정확히 하나로 분류한다.
- nonempty entity의 typed field constraint가 가상 field value에 적용된다.

### 실패

- 동일한 relation group을 exclusive이면서 coexistent로 선언하면 linking 단계에서 구조화된 오류가 난다.
- scope 1에서 같은 anchor가 두 required relation을 필요로 하고 둘이 exclusive이면 `UNSAT_WITHIN_BOUND`가 된다.
- cardinality 문장에 쓴 model이 relation의 첫 parameter와 다르면 `RSPDL-REL-002` 오류가 난다.

### 경계

- 앞의 scope 1 이론은 scope 2에서 서로 다른 target을 선택해 `SAT`일 수 있다. 작은 bound의 UNSAT을 전역 모순으로 승격하지 않는다.
- relation이 하나도 없는 module도 empty virtual world로 `SAT`일 수 있다. `nonempty`가 있을 때만 entity 존재를 요구한다.
- field가 없는 record model은 한국어 parser뿐 아니라 공통 analyzer에서도 거부한다.

### 오탐 방지

- 두 required relation을 `coexistent`로 선언한 scope 1 모델은 둘의 overlap을 conflict로 보고하지 않는다.
- 서로 모순처럼 보이는 optional field constraint는 field absence가 허용되면 conflict가 아니다.
- signature가 다른 relation을 같은 exclusive/exhaustive/coexistent group으로 묶지 못한다.
- 선언이 없는 overlap의 제품 의미를 analyzer가 임의로 판정하지 않는다.

## 의도적으로 지원하지 않는 범위

- source에 직접 쓰는 임의의 `forall`, `exists`, `AND`, `OR`, `NOT`
- 3항 이상 relation, relation attribute와 transitive closure
- field function을 relation으로 자동 변환하는 규칙
- 실제 JSON relation binding과 CRUD transition
- relation join, projection과 aggregation 실행
- sum derivation과 계산 dependency의 symbolic 실행; 존재하면 `Unsupported`를 반환함
- unbounded satisfiability 증명, 최적 scope 탐색과 symmetry breaking
- UNSAT core의 유일성 또는 최소 cardinality 보장

## References

- [정규화 타입·도메인과 논리 IR 코어](0002-typed-domains-and-logic-core.md)
- [Korean Domain Frontend Language Specification](0004-natural-korean-domain-grammar.md)
- [Total Policy Condition Spaces and SMT-First Consistency Analysis](0006-total-policy-condition-space-analysis.md)
- [Policy Consistency Blind Spots](../problems/0002-policy-consistency-blind-spots.md)
