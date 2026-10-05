---
id: unified-statements
title: 데이터, 불변 조건과 자연 한국어 통합 문장
type: rfc
status: proposed
version: "1"
summary: Proposes a shared typed statement model and staged natural Korean CFG for explicit permissions, automatic attempts, effects, and data invariants.
topics:
  - unified-statement
  - natural-korean-cfg
  - data-invariant
  - condition-expression
  - selector
  - pre-post-state
related:
  - project-schedule-validation
  - natural-korean-domain-grammar
  - total-policy-condition-space-analysis
  - finite-relational-model-finding
  - conditional-data-production
  - typed-action-outcomes-and-recovery
  - frontend-semantic-analysis-contract
problem_refs:
  - data-lifecycle-modeling-gap
  - policy-consistency-blind-spots
  - semantic-source-provenance-loss
  - semantic-reference-direction-loss
last_updated: "2026-10-05"
owners:
  - rspdl-maintainers
---

# 데이터, 불변 조건과 자연 한국어 통합 문장

## 상태와 실패 시나리오

이 RFC는 통합 의미 모델과 단계별 구현 계획이다. 아래 전체 의미 모델과 후속 selector·전후 상태 검증은 구현 완료 선언이 아니다.
마지막 구현 범위 절에 적은 A/B만 전달 완료이며, CFG의 현재 compile-only 범위와 후속 계획을 구별한다. 현재 구현의 권위는 README, PRD와 frontend contract에
있으며, 이 문서의 마지막 구현 범위 표에 실제 전달한 slice만 별도로 기록한다.

[추적 이슈 #43](https://github.com/rspdl/rspdl-core/issues/43)의 일곱 사례는 typed field,
무조건 권한, 관계 cardinality와 producer를 선언해도 제품 의도가 끊기는 지점을 보여준다.
예를 들어 출근보다 이른 퇴근 시각이 기존 검사에서 통과하고, 정확한 수신자가 아닌 사용자에게도
ExactlyOne 알림을 생성할 수 있다. 원인은 사례마다 새 문법 이름이 부족해서만이 아니라,
같은 데이터와 조건을 권한, 자동 동작, 파생값과 이후 상태에서 공유할 계약이 없기 때문이다.

외부에서 계산된 `서명 가능`, `사용일수`, `수신자`를 입력받는 우회는 판단 근거를 검증하지 않는다.
목표는 기획자가 데이터, 항상 지킬 조건, 상황에 따라 할 수 있거나 시도할 일을 같은 개념으로
작성하고, compiler가 명시한 의도와 미결정을 구분하게 하는 것이다.

## 작성자의 mental model

1. **데이터**는 field, 관계, 식별자와 존재 상태를 선언한다. 실제 값이 없는 typed record도
   유효 데이터나 관계 연결이 증명된 것은 아니다.
2. **불변 조건**은 유효 데이터 또는 명시한 전이에 항상 필요한 조건이다. 권한 거절과 구별한다.
3. **문장**은 어떤 계기에 누가, 어떤 입력과 대상을 가지고, 어떤 조건에서 행동을 할 수 있는지,
   할 수 없는지, 또는 자동으로 시도하는지와 그 결과의 데이터 효과를 명시한다.

`trigger`, `actor`, `bindings`, `condition`, `modality`, `effects`는 compiler 내부 의미 축이다.
작성자가 `계기:`, `행위자:`, `조건:` 같은 label 목록을 채우는 새 정책 포맷을 요구하지 않는다.
표면 언어는 조사, 연결어, 마침표와 제한된 문장/들여쓰기 블록을 가진 자연 한국어 CFG다.
자유 한국어를 LLM으로 추측해 실행 의미를 만드는 경로는 없다.

## 공통 의미 계약

제안하는 locale 독립 IR은 다음 구조를 가진다. 구현 시 기존 public construct를 바로 삭제하거나
이 구조로 직렬화 형식을 교체하지 않는다. 초기에는 additive lowering 또는 명시적 adapter를 사용하고,
원래 stable ID, 의미 종류와 source span을 보존한다.

```text
Statement {
  id, span,
  trigger: Request(action) | Event(event) | TimeCondition(clock),
  actor: ActorBinding | DeclaredSystem,
  bindings: [TypedBinding],
  condition: Condition,
  modality: Can | Cannot | DoAttempt,
  action: ActionRef,
  effects: [TypedEffect]
}
Condition = Atom | And([Condition]) | Or([Condition]) | Not(Condition)
Invariant { id, span, scope: State | Transition, condition: Condition }
Context { actor, request, eventPayload, clock, preState, postState }
```

필수 의미 축을 모르는 경우 이름, 화면 환경이나 문장 순서로 채우지 않는다. Action 요청과 Event,
clock 계기를 구별하며, action 성공 outcome과 action 요청을 같은 사건으로 합치지 않는다.
현재의 모델 수준 mutation은 대상 record나 값까지 지정한 effect로 승격하지 않는다.

### 공유 selector와 expression

권한 조건, 파생값, 수신자 선정, 자동 동작의 target과 invariant는 같은 typed selector/expression
계약을 소비한다. selector는 출발 binding, 관계 방향, 필터, 결과 cardinality와 state phase를 가진다.
단일 record가 필요한 곳에 집합을 넣으면 first-record를 추측하지 않고 거부한다. fan-out은 명시한
각 원소에 대해 effect를 만든다. 집합 중복 제거와 정렬/동점 처리도 서로 다른 연산이다.

expression은 literal, field/reference, typed 비교, 제한된 산술과 명시한 함수로 단계별 확장한다.
통화, 백분율, 단위, 날짜와 날짜시간의 타입 의미를 지운 범용 숫자 비교로 구현하지 않는다.
서로 다른 통화, 비교 불가능한 시간 종류, 누락된 binding과 지원하지 않는 함수는 structured error다.
데이터 값의 `없음`은 solver의 `unknown`과 다르며, `아직 생성되지 않음`과 `삭제됨`도 구별한다.

### 전후 상태와 데이터 lifecycle

요청 조건은 명시한 pre-state에서, 성공한 변경 결과는 post-state에서 평가한다. Event payload는
immutable trigger snapshot이며, 그것이 삭제 전 record에서 왔다는 provenance는 별도 선언 없이는
증명하지 않는다. `변경 전`, `변경 후` selector는 같은 resource identity와 명시한 phase를 가진다.

State invariant는 선택한 상태의 데이터 정합성을 검사한다. Transition invariant는 같은 identity의
pre/post 관계를 검사한다. 생성, 조회, 수정, 삭제, 파생은 입력 가용성과 dependency impact를 가져야
하며, 생성되지 않았거나 삭제된 record를 사용하는 경로는 통과로 처리하지 않는다. 삭제 후
cascade/restrict/detach/retain 의미와 재계산 시점은 작성자가 정한다.

### 허용, 금지와 자동 시도

`Can`은 요청할 수 있다는 허용이며 실행이나 성공의 보장이 아니다. `Cannot`은 금지이며 관련
invariant를 대신하지 않는다. `DoAttempt`는 계기와 조건을 만족하면 명시한 action을 시도하는 의도다.
**자동 시도는 eventual success 보장이 아니다.** 실패, 취소, timeout, 재시도, 중복 실행 방지와 동시
요청 처리는 별도 outcome/의무 계약이 필요하다. 실제 clock scheduler와 메시지 전달은 application
책임이다. 선언한 trigger의 발생과 시스템 fairness를 가정하지 않은 채 liveness를 증명하지 않는다.

동시에 적용되는 Can/Cannot은 기본 conflict다. source 순서는 priority가 아니며 더 구체적인 조건,
관리자 역할 또는 Cannot이라는 이름에서 override를 추측하지 않는다. 작성한 override/default/totality만
분석한다. 서로 다른 field의 양립 가능한 effect 중첩과 같은 field의 모순된 write를 구별한다.

## 자연 한국어 CFG 계획

다음은 **제안 문법**이다. 문법별 지원은 단계 완료 후 conformance로 확정하며 현재 parser에 넣을 수
있다는 뜻이 아니다. 식별자와 표시 이름은 기존 선언 문법을 재사용한다. `@`는 `@모듈`에만 허용한다.
논리 연산자는 명시한 중첩 들여쓰기 블록으로 결합 범위를 정한다. bare name의 조사 경계, 다중 단어 이름,
quoted name 처리는 기존 lexer 규칙에 맞춰 CFG 충돌 검사로 확정해야 한다.

초기 통합 문장은 이름과 stable ID를 가진 문장 블록으로 작성한다. 각 항목은 완결된 한국어
문장이고, 들여쓰기는 binding, 조건 집합과 field assignment의 범위를 나타낸다.

```text
접수 처리(receive_request)는 다음과 같이 정한다.
    접수 행동이 실행될 때 적용한다.
    시스템이 수행한다.
    대상 요청(target)은 접수 행동의 요청 입력을 사용한다.
    다음 조건을 모두 만족할 때 적용한다.
        대상 요청의 상태가 "대기"와 같다.
    이 처리를 자동으로 시도한다.
    대상 요청을 수정한다.
        상태를 "처리"로 정한다.
```

계기·행위자·binding·modality·effect와 비교 atom의 실제 허용 문형은
[실행 문법 `statement.ebnf`](../../crates/rspdl-ko/src/grammar/statement.ebnf)를 기준으로 한다.
아래 EBNF는 문장과 블록의 구성 설명이며, 토큰 경계·공백·quoted name 처리는 실행 문법과 adapter를 따른다.
비교는 `같다`, `다르다`, `작다`, `작거나 같다`, `크다`, `크거나 같다`를 사용하며
input alias, alias field와 typed literal만 operand로 받는다. 임의 텍스트를 조건으로 저장하지 않는다.

```ebnf
statement = name, canonical-id, ("은" | "는"), " 다음과 같이 정한다.", NEWLINE,
            INDENT, body-item, { body-item }, DEDENT ;
body-item = trigger-sentence | actor-sentence | binding-sentence
          | condition-group | modality-sentence | effect-sentence ;
trigger-sentence = action-name, ("이" | "가"), " 실행될 때 적용한다."
                 | event-name, ("이" | "가"), " 발생할 때 적용한다." ;
actor-sentence = "시스템이 수행한다." | role-name, ("이" | "가"), " 수행한다." ;
binding-sentence = name, canonical-id, ("은" | "는"), " ", trigger-name, "의 ", input-name,
                   " 입력을 사용한다." ;
condition-group = group-sentence, NEWLINE, INDENT,
                  condition-item, { condition-item }, DEDENT ;
group-sentence = "다음 조건을 모두 만족할 때 적용한다."
               | "다음 조건 중 하나 이상을 만족할 때 적용한다."
               | "다음 조건을 만족하지 않을 때 적용한다." ;
condition-item = typed-comparison | condition-group ;
typed-comparison = operand, ("이" | "가"), operand, ("와" | "과"),
                   ("같다" | "다르다"), "."
                 | operand, ("이" | "가"), operand, "보다",
                   ("작다" | "작거나 같다" | "크다" | "크거나 같다"), "." ;
operand = binding-name | binding-name, "의", field-name | enum-variant-name
        | integer-literal | boolean-literal | string-literal ;
modality-sentence = "이 처리를 할 수 있다." | "이 처리를 할 수 없다."
                  | "이 처리를 자동으로 시도한다." ;
effect-sentence = binding-name, ("을" | "를"), " 조회한다."
                | binding-name, ("을" | "를"), " 삭제한다."
                | binding-name, ("을" | "를"), " 수정한다.", NEWLINE, INDENT,
                  assignment, { assignment }, DEDENT ;
assignment = field-name, ("을" | "를"), " ", operand, ("로" | "으로"), " 정한다." ;
```

Body item의 source 순서는 의미 순서나 priority가 아니다. Parser는 반복 item을 받아서
계기·행위자·modality가 각각 정확히 하나, 조건 root가 최대 하나, effect가 하나 이상인지 검사한다.
Binding과 effect 내부의 assignment는 여러 개를 허용한다. 이 cardinality 조건은 CFG 반복의 의미
검증이며 위 예시의 항목 순서만 유효하다는 뜻이 아니다.

공개 parse JSON의 statement operand·condition·effect는 기존 AST와 같은 `kind` discriminator와
snake_case variant 이름을 사용한다. Payload는 operand의 `value`, condition/effect의 `definition`에
들어간다. Semantic reference의 operand dependency는 조건과 assignment 모두 `operand.binding_id`와
`operand.value_type`를 사용하고 원래 참조 위치로 사용처를 구분한다. Binding 입력은 선언에서 한 번
해석하며, 같은 잘못된 입력을 여러 필드에서 사용해도 입력 참조 오류를 반복해서 추가하지 않는다.

표면 조사 변형은 generated CFG에 구체화한다. `모두`는 And, `중 하나 이상`은 Or,
`만족하지 않을 때`는 Not로 lowering한다. Not group은 자식 하나만 허용하고 빈 그룹은 거부한다.
Condition grouping과 precedence는 indentation AST에 명시적으로 남는다. 자연어 `하나 이상`은
inclusive Or이며 exclusive-one 조건으로 읽지 않는다.

별도 state invariant 문장은 다음과 같다. RHS는 같은 모델의 field 이름이며 strict/inclusive 비교를
`커야 한다`, `작아야 한다`, `크거나 같아야 한다`, `작거나 같아야 한다`로 명시한다.
선택 field가 없으면 기존 concrete constraint 계약대로 비교를 건너뛴다. 필수값 검증을 대체하지 않는다.

```text
프로젝트의 마감일은 시작일보다 커야 한다.
근태 기록의 퇴근 시각은 출근 시각보다 크거나 같아야 한다.
```

관계 path, clock trigger와 전후 상태 문장은 후속 계획이다. `오늘`, `해당`, `배정된`은
추후 context binding으로 정의하며 이름만 보고 의미를 추측하지 않는다. 새 statement의 creation은
이번 범위에 없고 기존 conditional production API를 계속 사용한다. 기존 screen, policy, event producer와
workflow를 이미 새 Statement로 완전히 정규화했다고 주장하지 않는다.

## 이슈 coverage와 반례 계약

이슈 원문은 2026-10-05에 읽기 전용으로 확인했다. #43의 컴파일 성공은 원문 정책 전체 보존의
증거가 아니며, 각 이슈의 main `7833186`/SDK `0.1.4` 보고와 새 구현 검증을 구별한다.

| 이슈 | 필요한 의미 | falsifiable conformance criterion | 원문 미결정 또는 적용 범위 |
| --- | --- | --- | --- |
| [#43](https://github.com/rspdl/rspdl-core/issues/43) 추적 | 사례별 source → IR → 진단 증거 | #44–50 각각 정상·실패·경계·오탐 방지 fixture와 지원 단계가 연결되고, unsupported 사례를 성공으로 세지 않는다 | 일곱 사례 전체를 한 slice 완료로 보고하지 않는다 |
| [#44](https://github.com/rspdl/rspdl-core/issues/44) 프로젝트 일정 | field 간 Date 순서 invariant, 입력/저장 계약 | `시작=10-03, 마감=10-02` 실패, `10-04` 통과, 같은 날은 선언한 strict/non-strict에 따라 다르게 판정 | 같은 날 허용 여부와 상충할 수 있는 날짜 picker 문구; UI 비활성 날짜는 별도 projection |
| [#45](https://github.com/rspdl/rspdl-core/issues/45) 마커 | typed 곱셈/나눗셈, provenance, transition invariant | 너비 1000, 클릭 300의 x=0.3 통과/x=0.6 실패; zoom-only 변경의 비율 변경 실패; 분모 0은 성공 금지 | 도면 내용 변경의 의미·안내; 실제 relation binding은 cardinality 선언과 별도 |
| [#46](https://github.com/rspdl/rspdl-core/issues/46) TBM | relation path 존재, actor identity, clock, 복합 유일성, 전후 invariant | 미배정 관리자/타인 명의/전일 TBM/중복 `(TBM,서명자)` 실패, 본인 배정 당일 최초 서명 통과; 이미지 재연결·삭제 우회도 불변성 범위에 포함 | timezone·오늘의 기준; 손글씨 실재/빈 캔버스 판단은 별도 입력 계약 |
| [#47](https://github.com/rspdl/rspdl-core/issues/47) 근태 | DateTime 순서, target selector, 시간 조건, field effect, location selector | 역전 시각 실패; 8시간 경과 시도 없음/9시간 경계 시도; 수동 퇴근 완료 record를 선택하지 않음; 위치 후보 0/1/2와 거리 동점 구분 | 자동 저장 시각, 동시 수동/자동 결과, 조직 인원의 프로젝트 연결 범위 |
| [#48](https://github.com/rspdl/rspdl-core/issues/48) 휴가 | calendar 함수, 그룹별 aggregate, conditional requiredness/최소 보장, 재계산 | fixture calendar의 10일 사용을 0일로 입력하면 실패; 실제 출산일 변경 후 45/60일 미달 실패; 유형별 missing-date와 연도 경계 검사 | 회사휴일·조직·연도·신청자 범위, 달력일/근무일과 경계 포함, 보정 충돌 정책 |
| [#49](https://github.com/rspdl/rspdl-core/issues/49) 자금 | Money/Percentage 산술, dependency, 표시 projection | 원가 10000 KRW/집행 5000 KRW/50%에서 목표 5000 KRW·100% 통과, 1 KRW·0% 실패; 데이터 250%를 gauge 200% 정책으로 거부하지 않음 | 목표 0 나눗셈, 반올림·정밀도·백분율 표현; gauge 상한은 데이터 상한 아님 |
| [#50](https://github.com/rspdl/rspdl-core/issues/50) 알림 | relation set, dedup/차집합/fan-out, pre-state snapshot, 접근 조건 | 두 작업에 중복 배정된 사용자에게 알림 1개; 행위자 0개; 잘못된 프로젝트 수신자 실패; 삭제 전 이름/관계 snapshot provenance 보존 | 기존 표/2026-08-19 지침 우선, 도면·인원 수신 범위, 삭제 후 상세/목록 이동 |

미결정을 채우기 전에는 해당 최종 제품 시나리오를 conformance로 확정하지 않는다. 다만 명시한 임시
fixture 계약을 검증하는 데 제품 결정을 기다릴 필요는 없다. fixture의 선택을 원문 정책으로 주장하지 않는다.

## 단계별 deliverable과 완료 기준

| 단계 | 전달물 | 정확한 완료 기준 | 아직 보장하지 않는 것 |
| --- | --- | --- | --- |
| S0 | 문제/이슈 matrix와 invariant 우선 slice | 기존 원인 ID 연결, #43–50의 반례·미결정 공개, 현재/제안 구별 | 새 문장 전체 구현 |
| S1 | field 간 ordered Date/DateTime invariant의 한국어 → IR → runtime check | #44/#47 순서 반례 검출, strict/non-strict와 optional field 부재 구분, type mismatch/link failure, 양쪽 reference 및 source span, 입력 순서 독립 | 시간 scheduler, 날짜 picker, 일반 숫자 산술, 전후 상태 |
| S2 | 공통 context/binding/selector/expression과 And/Or/Not CFG | 중첩 블록별 AST 예상 shape, Not truth table, relation 방향 오류/집합 단일값 혼동 거부, actor와 phase 참조 linking·type check, reordered canonical 결과 동일 | 모든 관계/산술/달력 함수 지원 |
| S3 | Can/Cannot의 조건부 policy lowering과 분석 | 명시한 닫힌 decision domain에서 conflict/gap/compatible overlap/unreachable별 witness 재검사; 독립 문장 순서 변경 결과 동일; override 없이는 충돌 유지 | runtime unmatched만으로 전역 gap 추정, 임의 양화식 완전성 |
| S4 | DoAttempt, target/field effect, typed outcome와 전후 invariant | 8h/9h/이미퇴근 조건별 attempt 선택, 실패/timeout에도 eventual 성공 claim 없음; 변경 전/후 identity와 write 충돌 증거; 삭제 후 입력 거부 | 실제 scheduler, 동시성/재시도 의미의 임의 기본값 |
| S5 | 산술·달력·집합 fan-out의 도메인별 extension | #45/#48/#49/#50 matrix의 각 반례를 source부터 검사하며 rounding/zero/dedup/calendar contract fixture 명시 | 단순 범위 검사나 payload 값만으로 provenance 증명 |

최초 전달은 S1과 S2의 직접 binding/복합 조건 subset을 함께 선택했다. S1은 이슈의 실제 통과 반례를 가장 작은 end-to-end 변경으로 막고,
새 policy/execution 체계를 한 번에 도입하기 전에 shared expression의 첫 typed 비교 기반을 검증한다.
각 단계는 정상·실패·경계·오탐 방지 사례를 가장 가까운 owning layer와 public conformance에 남긴다.
문서/knowledge index 검증과 전체 `./scripts/check.sh`가 통과해야 전달 완료다.

진단은 기존 진단 체계를 재사용하거나 명시한 새 stable Rule ID를 정의하고 원문 span, 관련 field/statement
ID, 실제 값 또는 solver witness와 분석 범위를 포함한다. 의존성 graph는 typed 참조를 사용하며 문자열을
재검색해 참조를 만들어내지 않는다. solver timeout/지원 밖 식은 structured error 또는 `unknown`이고,
bounded model의 UNSAT는 그 scope 밖의 불가능성 증명이 아니다. 이 계획은 일반 model checker의
완전성, 무한 상태 reachability 또는 모든 제품 의도의 보존을 주장하지 않는다.

## 초기 slice의 진단 계약과 증거 범위

아래 ID는 현재 코드의 진단 계약이며 2026-10-05 전체 harness에서 검증했다. 공통 analyzer는
참조·타입·ordering 지원 여부와 한 statement 안의 구조를 검사한다. 조건의 참/거짓을
평가하거나 서로 다른 statement의 conflict/gap/unreachable을 증명하지 않는다. Core API에서
지원하는 타입 검증이 한국어 source의 모든 값 표기와 실행 동작을 지원한다는 뜻도 아니다.

| 진단/결과 | 현재 검사하는 실패 |
| --- | --- |
| `RSPDL-KO-SYN-060` | 잘못된 statement 문장/들여쓰기 구조, 필수 항목 누락·중복 또는 조건 그룹 구조 |
| `RSPDL-KO-REF-003` | binding 문장에 쓴 입력 owner가 statement 계기와 다름 |
| `RSPDL-STMT-001` | statement/binding stable ID 누락·잘못된 ID 또는 statement ID 중복 |
| `RSPDL-STMT-002` | 선언한 종류의 Action/Event 계기를 찾을 수 없음 |
| `RSPDL-STMT-003` | 수행 역할을 찾을 수 없음 |
| `RSPDL-STMT-004` | binding ID 중복, 입력 owner/입력 참조 불일치 또는 없는 binding 사용 |
| `RSPDL-STMT-005` | scalar binding을 모델로 사용하거나 field가 binding 모델에 속하지 않음 |
| `RSPDL-STMT-006` | operand/assignment 타입 불일치, scalar 요구, literal 변환 실패 또는 지원하지 않는 비교/order |
| `RSPDL-STMT-007` | effect 대상이 기존 모델 입력이 아님, 빈 Update 또는 effect 없음 |
| `RSPDL-STMT-008` | 같은 입력·field에 중복 write; alias가 달라도 같은 input이면 검출 |
| `RSPDL-STMT-009` | 같은 입력의 중복 Delete 또는 Delete와 Read/Update가 함께 선언됨 |
| `RSPDL-STMT-010` | 공통 IR의 빈 And/Or 조건 그룹 |
| `RSPDL-STMT-011` | presence 의미가 정의되지 않은 선택 field를 operand로 사용 |
| `RSPDL-STMT-012` | immutable Event payload binding을 Update/Delete 대상으로 사용 |
| `RSPDL-STMT-090` | `check`/`check_files`에서 새 statement의 runtime 평가 미지원 |
| model finding `Unsupported` | statement 의미를 bounded finder가 평가하지 않음; satisfiable/unsatisfiable 판정 아님 |

진단의 span은 실패한 참조/항목 또는 statement 범위에 남긴다. `STMT-008`은 statement·input·field
ID를, binding/field 참조 실패는 관련 reference/model 정보를 추가한다. 모든 진단에 runtime witness가
있다고 주장하지 않는다. 필드 순서 invariant의 runtime 위반은 `constraint_id`, `model_id`, `record_id`, `left`, `right`를
제공한다. 양쪽 field ID는 해당 `constraint_id`가 가리키는 compiled IR 제약에서 조회하며,
violation 객체에 field ID가 직접 포함된다는 뜻은 아니다.

- [Statement conformance](../../conformance/ko-KR/statements/): 정상 중첩 조건/금지 문장, 0 경계,
  type/actor/중복 write 실패, 선택 operand 거부, 서로 다른 field/input의 오탐 방지. 이 fixture는
  compile 결과를 검사하며 조건 truth 평가나 실제 effect 실행의 증거가 아니다.
- [Field ordering conformance](../../conformance/ko-KR/field-ordering/): 정상 날짜 순서, 역전 위반과
  양쪽 값 evidence, strict/inclusive 같은 날짜 경계, optional 부재 오탐 방지, 타입·참조 실패.
  runtime evidence는 제공한 record에 대한 검사이며 전체 가능한 데이터의 완전성 증거가 아니다.
- [통합 문장 예제](../../examples/unified-statements.rspdl): 지원하는 자연 한국어 문장과 IR 전달의 예시.
  compiler/domain/frontend 단위 검사는 provenance, 순서 독립 정규화, 공통 core type/order validation과
  unsupported runtime/model 경계를 검증한다. 전체 harness 완료 여부는 다음 절에 별도로 기록한다.

## 실제 구현 범위와 남은 계획

최초 구현 A/B는 2026-10-05 전달 및 검증을 완료했다. 완료 범위는 다음 두 경로로 제한한다.

- A: 위 자연 한국어 문장 블록을 named Action/Event 계기, System/Role 행위자, 직접 typed trigger-input
  alias, And/Or/Not 비교, Can/Cannot/DoAttempt와 여러 Read/Delete/Update field assignment로 lowering한다.
  bound existing input만 대상으로 삼으며 common IR/link/type analysis와 provenance를 검증한다.
  Action 입력은 `PreMutation`, immutable Event payload는 `TriggerPayload` phase를 보존한다.
  Event binding의 Read만 허용하며 Update/Delete는 `RSPDL-STMT-012`로 거부한다.
  선택 field operand는 명시적 presence 처리 계약 전까지 `RSPDL-STMT-011`로 거부한다.
  **Compile-only** 계약이다. runtime에서 새 statement를 평가하거나 데이터를 변경하지 않고,
  전체 condition space의 conflict/gap/reachability를 증명하지 않는다.
  IR은 `LinkedAndTypeCheckedOnly` 분석 상태를 명시한다. 새 statement가 있는 module의
  `check`/`check_files`는 `RSPDL-STMT-090`으로 runtime 미지원 경계를 보고하고,
  domain `find_model`은 `Unsupported`를 반환한다. 계기가 실제 실행됐다는 주장을 하지 않는다.
- B: field-to-field ordered Date/DateTime invariant를 source → IR → runtime check로 연결하여
  #44/#47의 역전 반례를 검사한다. strict/non-strict operator, optional field 부재 skip과 type/link
  경계를 실제 fixture로 검증했다. 순서 비교는 동일한 ordered 타입에 적용되며 Date/DateTime 사례를
  포함한다.

Timer, relation join, 일반 산술, snapshot, post-state와 runtime mutation은 이번 범위 밖이다.
날짜 순서 제약의 application 연결은 후속 [프로젝트 일정 입력·저장 예제](../guides/project-schedule-validation.md)로
검증한다. 이 예제는 B의 IR과 concrete check 결과를 소비하며 A의 statement runtime을 구현하지 않는다.
S2의 직접 binding/복합 조건 일부를 A로 전달하지만 S2의 relation selector와 이후 단계 전체는 완료가 아니다.
기존 API는 단계적 migration 동안 공존한다. Wire 결과는 additive이며 빈 `statements`는 생략한다.
Rust `UnlinkedModule`/`SemanticModule` struct literal에는 새 `statements` field가 필요하므로
Rust source compatibility 전체를 보장하는 변경은 아니다. 전체 RFC의 `proposed` 상태와
이슈 #43–50의 미완료 범위는 유지한다.

## 검증 결과 — 2026-10-05

리뷰 수정 후 `./scripts/check.sh`가 exit 0으로 완료했다. Rust 48 result suite(빈 doctest suite 포함)에서
400 tests passed / 0 failed였고, Python 14개 테스트가 통과했다.
Workspace strict Clippy(`-D warnings`), 전체 formatting과 release metadata 동기화 검사도 통과했다.

실제 conformance integration은 statement 9 case에서 18 tests, field ordering 8 case에서
6 tests가 통과했다. Korean frontend의 121 unit tests와 AST JSON·binding 진단 회귀 검사 6개,
compiler operand reference 회귀 검사 1개는 위 Rust 400개에 포함되며 별도 합산하지 않는다.
이 결과는 위 A/B 경계를 검증하며 timer, runtime statement 실행, 전체 조건 공간 분석이나
일곱 제품 사례의 완료를 증명하지 않는다.
