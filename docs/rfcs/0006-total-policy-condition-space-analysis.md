---
id: total-policy-condition-space-analysis
title: Total Policy Condition Spaces and SMT-First Consistency Analysis
type: rfc
status: proposed
version: "0.3"
summary: Defines closed policy vocabulary, exhaustive condition-space coverage, explicit override semantics, and SMT-first consistency analysis.
topics:
  - policy-analysis
  - smt
  - condition-coverage
  - totality
  - override
  - closed-vocabulary
related:
  - rspdl-language-prd
  - typed-domains-and-logic-core
problem_refs:
  - policy-consistency-blind-spots
  - data-lifecycle-modeling-gap
last_updated: "2026-08-12"
owners:
  - rspdl-maintainers
target_spec: "0.2.0"
---

# Total Policy Condition Spaces and SMT-First Consistency Analysis

## 상태와 목적

이 RFC는 Proposed 상태다. 조건부 정책과 상태별 데이터 요구사항을 구현하기 전에 필요한 의미 계약을 정의한다. 표면 문법, Canonical IR 구조와 Rule ID는 이 계약을 만족하는 가장 작은 vertical slice를 설계할 때 별도로 확정한다.

RSPDL은 구체적인 runtime record가 없어도 선언된 유효 입력 공간 전체에서 다음 문제를 찾아야 한다.

- 결과가 필요한데 어떤 분기도 적용되지 않는 `gap`
- 둘 이상의 분기가 같은 입력에 적용되는 `overlap`
- 함께 적용된 결과가 양립할 수 없는 `conflict`
- 전제조건 또는 상위 분기 때문에 절대 적용되지 않는 `unreachable`

정적 조건 공간 분석은 typed SMT solving을 우선 사용한다. 현재 runtime policy match는 선언된 무조건 allow/deny 정책을 직접 결정적으로 대조한다. Datalog evaluator는 제거했으며, 유한 관계의 재귀적 폐쇄가 필요한 구체적인 제품 시나리오가 생길 때에만 별도 RFC로 재검토한다.

### 구현된 기반 slice

`rspdl-domain`은 solver 구현에 의존하지 않는 `TotalDecisionPoint` 분석 API를 제공한다. 현재 slice는 의도적으로 다음 범위만 지원한다.

- 선언된 variant 전체를 포함하는 단일 닫힌 enum 변수
- stable ID, typed Boolean condition과 `allow` 또는 `deny` effect를 가진 독립 branch
- variant별 gap query와 canonical witness
- 같은 effect의 compatible overlap과 allow/deny conflict를 구분하는 branch pair query
- branch stable ID 순서에 따른 결정적 결과
- solver `UNKNOWN`과 backend error를 성공으로 근사하지 않는 결과 계약

`rspdl-solver-z3` 통합 테스트가 실제 SMT lowering과 witness를 검증한다. 이 기반 API는 아직 Controlled Korean 조건식, `UnlinkedModule`/`SemanticModule`, compiler diagnostic 또는 CLI에 연결되지 않았다. 또한 default, override, 순서 있는 `else-if`, effective condition, unreachable, 다중 변수와 일반 effect compatibility를 구현하지 않는다. 특히 독립 branch 조건만으로 임의의 우선순위를 만들어 unreachable을 보고하지 않는다.

## 제품 문제와 실패 시나리오

자연어 기획은 한 조건이 참인 경우만 설명하고 그 밖의 상태를 암묵적으로 남기기 쉽다.

```text
프로젝트 상태 = {진행, 종료, 예정, 중지}

작성된 정책:
    상태가 진행이면 수정할 수 있다.
```

`종료`, `예정`, `중지`에서 수정 가능 여부가 필요한데 결과가 없다면 개발자가 이를 추측하거나 결정을 기다려야 한다. runtime 요청을 검사하는 방식은 우연히 해당 상태의 데이터가 들어오지 않으면 이 공백을 발견하지 못한다.

또한 다음 두 문장은 서로 다른 문제다.

```text
임시 등록에서는 식별자가 선택이다.
정식 등록에서는 식별자가 필수다.
```

이는 서로 배타적인 상태에 따른 조건부 요구사항이다. 반면 같은 입력에서 두 규칙이 함께 적용되고 결과가 다를 때 어느 규칙이 이기는지를 정하는 것은 priority 또는 override 문제다. 두 의미를 하나의 "더 구체적인 규칙 우선" 추측으로 합치면 작성하지 않은 제품 의도가 생긴다.

## 의미 모델

정책 분석 단위인 decision point는 다음 정보를 가진다.

- `scope`: actor, resource, action과 lifecycle state 등 정책을 비교할 범위
- `domain`: 해당 scope에서 유효한 typed input assignment의 집합
- `slot`: 하나의 결정을 내려야 하는 결과 자리
- `branch`: typed condition과 선언된 effect의 쌍
- `totality`: 모든 유효 입력에서 결과가 필요한지 여부
- `override`: 겹치는 branch 사이에 작성자가 명시한 우선 관계

아래 표기에서 `D`는 유효 입력 domain, `C_i`는 branch `i`의 조건, `E_i`는 그 branch의 effect다.

SMT는 `NOT(C_i)`를 포함한 논리식을 정확하게 다룰 수 있다. SMT의 한계는 조건의 반대 영역을 계산하지 못한다는 데 있지 않다. Solver는 다음 제품 의미를 스스로 만들지 못한다.

- `C_i` 바깥도 반드시 다른 정책으로 덮여야 한다는 요구
- 생략된 enum variant에서 수행할 effect
- effect 둘이 제품상 양립 가능한지 여부
- 더 구체적으로 보이는 문장이 자동으로 이긴다는 우선순위

따라서 domain, totality, effect compatibility와 override는 RSPDL 의미로 명시해야 한다.

## 닫힌 vocabulary와 선언된 domain

모든 의미 기호는 사용 전에 stable ID, 타입과 소유 scope를 선언해야 한다.

- data model, field와 enum variant
- actor와 role
- resource와 action
- condition에서 참조하는 predicate
- effect와 effect가 쓰는 decision slot 또는 state
- state transition의 이전 상태와 다음 상태

Frontend나 analyzer는 오타 또는 알려지지 않은 단어를 새 vocabulary로 자동 등록하지 않는다. 표시 이름이 유사해도 선언 stable ID와 연결되지 않으면 structured link error다. 사용자 정의 확장을 허용하더라도 typed signature, symbolic support와 effect contract를 먼저 선언해야 한다.

Enum은 선언된 variant만 갖는 닫힌 유한 domain이다.

```text
project.status = {active, ended, scheduled, paused}
```

정수나 문자열처럼 무한한 domain도 사용할 수 있지만, 유효 범위와 backend의 symbolic support를 확인해야 한다. 무한 domain이라는 이유만으로 unsupported가 되지는 않는다.

## 전체성 및 명시적 나머지 영역

Total decision point에서는 모든 유효 입력이 적어도 하나의 effective branch로 덮여야 한다. 나머지 영역을 아무 동작 없이 인정하려면 `no_change`, `not_applicable` 같은 결과도 vocabulary와 branch에 명시해야 한다. 정확한 표면 문법과 built-in effect 목록은 이 RFC에서 결정하지 않는다.

```text
gap query:
    D
    AND NOT(C_1 OR C_2 OR ... OR C_n)
```

이 query가 `SAT`이면 모델은 누락된 입력의 witness다. `UNSAT`이면 선언된 domain 전체가 덮인다. `UNKNOWN` 또는 unsupported lowering은 coverage 성공으로 취급하지 않는다.

앞의 프로젝트 상태 예에서 `active` branch만 있으면 `ended`, `scheduled`, `paused`는 모두 공백이다. SMT 모델 하나는 gap의 존재만 증명하므로, 진단은 다음 원칙을 따른다.

- 유한 enum 축은 누락 variant를 canonical 순서로 열거하거나 동등한 compact region으로 보고한다.
- 무한 수치·문자열 축은 재현 가능한 witness를 제공하되 한 모델이 전체 gap 모양을 설명한다고 주장하지 않는다.
- 서로 다른 gap region을 추가로 열거할 때는 이미 보고한 region을 차단하는 조건과 종료 기준을 명시한다.

Partial decision point와 의도된 미정의 범위를 미래에 지원하더라도 작성자가 명시해야 한다. 단순히 branch가 없다는 사실을 의도된 partial policy로 해석하지 않는다. 초기 조건부 decision point는 total을 기본 계약으로 삼는다.

## 조건부 불변식과 상태별 요구사항

상태별 field 요구사항은 priority 없이 서로 배타적인 branch로 표현할 수 있다.

```text
decision slot:
    registration.identifier.requirement

when registration.status = temporary:
    requirement = optional

when registration.status = registered:
    requirement = required
```

상태 enum에 다른 variant가 있다면 그 variant의 requirement도 명시해야 한다. `optional`은 field가 반드시 없어야 한다는 뜻이 아니라 absence를 허용한다는 선언된 결과다.

다른 데이터와의 정합성은 typed cross-field invariant로 표현한다.

```text
when registration.status = registered:
    identifier is present
    AND identifier.owner = registration.owner
```

조건부 requiredness, 값 제약과 action authorization은 서로 다른 decision slot 또는 invariant다. 같은 `if` 문형을 사용하더라도 effect 종류와 충돌 규칙을 공유한다고 가정하지 않는다.

### 값 부재와 분석 상태를 구분한다

조건부 requiredness를 분석하려면 다음 상태를 하나의 `false` 값으로 합치지 않아야 한다.

- field가 존재하지 않음
- field가 존재하며 typed value를 가짐
- 해당 decision point에서 명시적인 `no_change` 또는 `not_applicable` 결과가 적용됨
- backend가 결론을 내리지 못한 `UNKNOWN`

외부 JSON의 `null`을 field absence로 binding할지 별도 nullable value로 지원할지는 입력 계약이 결정한다. 어느 경우에도 `null`, absence, boolean `false`, policy gap과 solver `UNKNOWN`은 서로 대체할 수 없다.

### 현재 상태와 상태 전이를 구분한다

Action은 현재 상태에 대한 condition만이 아니라 pre-state와 post-state의 관계를 가질 수 있다.

```text
action:
    formalize registration

pre-state:
    registration.status = temporary

post-state:
    registration.status = registered
    AND identifier is present
```

`temporary`와 `registered`에서 requiredness가 다른 것은 같은 시점의 충돌이 아니다. 그러나 같은 pre-state와 action에서 서로 양립할 수 없는 post-state가 둘 이상 effective라면 transition conflict다. 생성되지 않았거나 삭제된 field를 postcondition이나 다른 데이터 정합성 검사의 입력으로 사용할 수 있는지도 lifecycle availability와 함께 검사해야 한다.

## Priority, default와 override

Priority는 condition의 대체물이 아니다. 두 branch가 같은 유효 입력에서 함께 적용될 때만 결과 해석에 관여한다.

다음 의미는 금지한다.

- source에 나중에 쓴 정책이 자동으로 이김
- 조건이 더 길거나 구체적으로 보이는 정책이 자동으로 이김
- `deny`, `allow` 또는 특정 effect가 선언 없이 자동 우선함
- 숫자 priority가 같을 때 입력 순서로 결과를 선택함

Override를 지원할 때는 stable branch ID 사이의 명시적인 우선 관계로 Canonical IR에 보존한다. 우선 관계는 순환할 수 없으며, cycle은 solver 실행 전 semantic error다.

Effective condition은 개념적으로 다음과 같다.

```text
effective(C_i) =
    C_i
    AND 이 입력에서 C_i를 override하는 상위 branch가 적용되지 않음
```

명시적 default는 유효 domain 중 다른 branch가 덮지 않는 나머지 영역을 담당한다. Default도 source 순서에 의존하지 않는 선언이어야 한다. Override 이후에도 양립 불가능한 effective effect가 둘 이상 남으면 conflict다.

정확한 priority 표면 문법, 정수 rank를 허용할지와 explicit override edge만 허용할지는 후속 RFC에서 결정한다.

## 독립 정책과 순서 있는 분기

독립 정책 두 개는 조건이 겹치면 둘 다 적용된다.

```text
if role = manager: allow
if amount >= 1000: deny
```

순서 있는 `if / else-if`의 두 번째 effective condition에는 앞 branch의 부정이 포함된다.

```text
if role = manager: allow
else if amount >= 1000: deny

second condition:
    role != manager
    AND amount >= 1000
```

표면 문법은 둘을 구분해 Canonical IR에 보존해야 한다. 독립 정책을 source 순서가 있는 분기로 lowering하거나 그 반대 방식으로 lowering할 수 없다.

## Effect와 동작 충돌

Condition overlap만으로 conflict라고 판정하지 않는다. Effect가 양립 불가능해야 conflict다.

Effect declaration은 최소한 다음 정보를 제공할 수 있어야 한다.

- effect가 값을 쓰는 exclusive decision slot 또는 post-state
- 함께 적용 가능한 독립 side effect인지 여부
- action의 precondition과 postcondition
- 보존해야 하는 cross-field 또는 lifecycle invariant

예를 들어 같은 authorization slot의 `allow`와 `deny`, 같은 next-state slot의 `approved`와 `rejected`는 양립 불가능하다. `allow`와 `audit_log_created`는 서로 다른 slot을 쓴다면 함께 적용할 수 있다.

제품상 `shipped`와 `refunded`가 동시에 참일 수 없다면 그 사실을 state invariant로 선언해야 한다. Solver는 effect 이름의 자연어 의미만 보고 충돌을 추측하지 않는다.

## SMT 분석 질의

같은 scope와 decision slot의 branch를 대상으로 다음 query를 구성한다.

```text
unreachable(i):
    D AND effective(C_i)
    UNSAT이면 unreachable

overlap(i, j):
    D AND effective(C_i) AND effective(C_j)
    SAT이면 overlap witness 존재

conflict(i, j):
    overlap(i, j)
    AND E_i와 E_j가 양립 불가능함

gap:
    D AND NOT(effective(C_1) OR ... OR effective(C_n))
    SAT이면 uncovered witness 존재
```

동작의 postcondition 자체를 SMT 식으로 표현하는 경우, 두 effect의 postcondition을 함께 만족할 수 없는지도 conflict의 근거가 될 수 있다. 단순한 effect ID 비교와 symbolic postcondition 검사의 경계는 후속 IR 설계에서 확정한다.

각 query는 scope의 domain constraint와 전역 invariant를 반드시 포함한다. 유효하지 않은 입력에서 찾은 모델은 제품 정책의 gap 또는 conflict witness가 아니다.

## Solver 결과와 진단 계약

- `SAT`: finding을 재현하는 canonical witness assignment를 반환한다.
- `UNSAT`: 해당 query의 finding이 존재하지 않음을 뜻한다.
- `UNKNOWN`: timeout 또는 solver가 결론 내리지 못한 이유를 구조화해 반환한다.
- `Unsupported`: 사용한 domain, predicate 또는 effect를 의미 손실 없이 lower할 수 없음을 solver 실행 전에 반환한다.

진단은 최소한 다음을 포함해야 한다.

- stable Rule ID와 message key
- decision point, scope와 관련 branch stable ID
- 원문 source span
- canonical 순서의 witness assignment 또는 uncovered finite variants
- explicit override와 effect incompatibility 등 판정 근거

Solver가 우연히 선택한 모델 값은 default나 유일한 해결책으로 표현하지 않는다. 같은 source와 spec version에서 진단과 evidence 순서는 결정적이어야 한다.

## Incremental solving과 rule switch

`push/pop`과 assumption literal 기반 switch는 반복 query의 성능과 설명 가능성을 위한 backend 전략으로 허용한다.

- 공통 domain과 invariant는 base solver context에 둔다.
- query별 임시 조건은 `push/pop`으로 격리할 수 있다.
- branch stable ID를 assumption switch와 연결해 선택적 활성화와 UNSAT core 추적에 사용할 수 있다.
- `pop` 누락이나 이전 model/core 재사용으로 서로 다른 query의 상태가 섞여서는 안 된다.

Incremental 실행과 매 query마다 새 solver를 만드는 실행은 관찰 가능한 의미 결과가 같아야 한다. 성능 최적화가 Canonical IR, finding 분류, witness 유효성과 정렬 순서를 바꿀 수 없다.

## 제거된 Datalog 경계

정적 policy consistency 분석의 기준 경로는 typed Boolean IR에서 SMT query를 생성하는 것이다. Datalog evaluator는 제거했다. 다음 요구가 구체화될 때에만 별도 RFC에서 도입 여부를 재검토한다.

- 선언된 유한 조직·역할 관계의 재귀적 상속
- 데이터 또는 모듈 의존성의 transitive closure
- active-domain 사실에서 파생 사실 전체를 materialize해야 하는 기능

현재 runtime request별 직접 policy match 결과는 전체 조건 공간의 totality를 증명하지 않는다. 매칭 결과가 없다는 사실을 SMT의 logical negation으로 자동 변환하지 않는다.

### 호환성 기록

SMT-first 분석으로 범위를 좁히면서 `rspdl-datalog` crate와 Datalog 전용 public rule IR(`LogicProgram`, `DerivationRule`, `RuleLiteral`, `Fact`, `PredicateApplication`) 및 `Backend::Datalog`를 제거했다. 이는 `0.x` public API 변경이다.

Compiler와 CLI의 runtime 입력 및 `allowed`, `denied`, `conflict`, `unmatched` 결과 JSON은 유지한다. 현재 무조건 정책의 매칭 결과와 canonical policy ID 정렬은 제거 전과 의미상 동일해야 한다.

## 적합성 사례

### 정상

- 모든 enum variant가 정확히 하나의 effective effect로 덮인다.
- 서로 겹치는 두 branch가 서로 다른 독립 slot에 compatible effect를 기록한다.
- `temporary -> optional`, `registered -> required`처럼 상태별 요구사항이 모든 유효 상태를 덮는다.

### 실패

- 네 개의 project status 중 하나만 effect를 선언해 세 variant가 uncovered다.
- 같은 입력에서 `allow`와 `deny`가 override 없이 함께 effective다.
- 선언되지 않은 action, effect, field 또는 enum variant를 참조한다.
- override graph에 cycle이 있다.

### 경계

- 숫자 구간 `< 100`과 `> 100` 사이에서 정확히 `100`만 gap이다.
- explicit default가 다른 모든 branch의 나머지 영역만 덮는다.
- 앞선 `if`가 `else-if` 조건을 완전히 가려 unreachable을 만든다.

### 오탐 방지

- invalid domain에만 존재하는 모델을 gap으로 보고하지 않는다.
- compatible effect의 overlap을 conflict로 보고하지 않는다.
- partial 또는 no-op을 명시적으로 지원하게 된 경우 그 선언된 영역을 gap으로 보고하지 않는다.
- 조건이 서로 배타적인 상태별 requiredness를 priority conflict로 보고하지 않는다.

## 의도적으로 결정하지 않은 사항

- 조건부 정책과 effect declaration의 Controlled Korean 표면 문법
- decision slot과 symbolic postcondition의 구체 Canonical IR 타입
- priority rank와 explicit override edge 중 지원할 표면 모델
- finite gap region을 하나의 진단으로 압축하는 표현식 형식
- 외부 API, 현재 시각, 확률, 동시성과 같은 동적 환경의 symbolic 모델
- 가장 단순한 witness 또는 최소 수정안을 찾는 optimization 기준

이 항목들은 작성되지 않은 제품 의도를 solver 전략으로 대신 결정하지 않는다.

## References

- [RSPDL Product Requirements](../prd.md)
- [Policy Consistency Blind Spots](../problems/0002-policy-consistency-blind-spots.md)
- [Data Lifecycle Modeling Gap](../problems/0001-data-lifecycle-modeling-gap.md)
- [Typed Domains and Logic Core](0002-typed-domains-and-logic-core.md)
