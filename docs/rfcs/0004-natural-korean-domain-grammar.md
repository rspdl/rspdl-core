---
id: natural-korean-domain-grammar
title: Korean Domain Frontend Language Specification
type: rfc
status: implemented
version: "0.8"
summary: Defines Korean record, relation, constraint and policy grammar and its deterministic lowering to the locale-neutral Unlinked IR contract.
topics:
  - ko-KR
  - controlled-language
  - data-model
  - constraints
  - policies
  - relations
  - bounded-model-finding
  - cfg
related:
  - controlled-korean-surface-grammar
  - typed-domains-and-logic-core
  - rspdl-compiler-architecture
  - frontend-semantic-analysis-contract
  - executable-frontend-grammar-compiler
problem_refs:
  - data-lifecycle-modeling-gap
  - policy-consistency-blind-spots
  - frontend-grammar-implementation-drift
last_updated: "2026-08-13"
owners:
  - rspdl-maintainers
target_spec: "0.4.0"
---

# Korean Domain Frontend Language Specification

## 1. 범위

이 문서는 `rspdl-ko` frontend의 규범 문법과 공통 Unlinked IR로의 lowering을 정의한다. 데이터, 열거형, 역할, 행동, 관계와 규칙은 읽을 수 있는 한국어 문장 또는 블록으로 작성한다. 들여쓴 field와 enum value에는 별도 `@`를 사용하지 않는다. 제약과 정책은 이름이나 source ID가 없는 독립적인 최상위 문장이다. 한국어 표시 이름을 선언 stable ID로 연결하는 일은 frontend가, stable ID linking·타입 검사와 의미 규칙은 [Frontend and Semantic Analysis Contract](../specs/frontend-semantic-analysis-contract.md)의 공통 analyzer가 소유한다.

`@`는 문서 수준 metadata에만 예약한다. 현재 whitelist는 module identity를 선언하는 `@모듈` 하나다. Domain declaration이나 rule을 위한 다른 annotation은 문법 오류다. 새 annotation은 문장/블록으로 의미를 결정적이고 읽기 쉽게 보존할 수 없다는 RFC 근거가 있을 때만 추가할 수 있으며 parser 구현 편의나 입력 길이는 근거가 아니다.

유저 플로우, 조건부 정책, 일반 `AND`/`OR`/`NOT`, 3항 이상 관계, 모듈 import와 자유 한국어 해석은 현재 범위에 포함하지 않는다. Field value collection은 지원한다. Unary/binary relation과 제한된 관계 메타 규칙은 [Finite Relational Rules and Bounded Model Finding RFC](0007-finite-relational-model-finding.md)의 구현 범위를 따른다.

## 2. 전체 예시

```text
@모듈 비용 승인(expense)

비용 상태(status)는 다음 값 중 하나다.
    작성 중(draft)
    제출됨(submitted)
    승인됨(approved)

비용 신청(request)은 다음 필드들로 구성되어 있다.
    식별자(id): 필수 문자열
    신청자(applicant): 필수 문자열
    승인자(approver): 선택 문자열
    금액(amount): 필수 정수
    승인 상태(status): 필수 비용 상태

사용자(user)는 다음 필드들로 구성되어 있다.
    이름(name): 필수 문자열

비용 신청의 금액은 0보다 커야 한다.
비용 신청의 신청자와 승인자는 달라야 한다.

비용 신청은 사용자를 검토자(reviewer)로 가질 수 있다.
비용 신청은 하나 이상 존재해야 한다.
모든 비용 신청은 검토자를 하나 이상 가져야 한다.
각 비용 신청은 검토자를 최대 하나만 가질 수 있다.

회계 관리자(accounting_manager)는 역할이다.
변경(change)은 행동이다.

회계 관리자는 비용 신청의 승인 상태를 변경할 수 있다.
```

`expense`는 이 문서에서 처음 선언되는 module ID다. 그 아래의 짧은 ID `request`, `status`, `change`는 lowering할 때 각각 `expense.request`, `expense.status`, `expense.change`가 된다. field와 enum value는 부모 ID 아래에서 한 단계 더 한정된다.

완전한 문장인 선언과 규칙에는 마침표가 필요하다. `@모듈`과 들여쓴 CFG 항목에는 마침표를 붙이지 않는다. 제약, 정책과 관계 규칙의 canonical ID는 정규화된 문장 의미에서 내부 생성한다.

## 3. 어휘 구조

### 3.1 입력 문자와 줄

source는 UTF-8 text다. 줄바꿈은 LF 또는 CRLF를 허용하며 parser 내부에서는 논리적 `NEWLINE`으로 취급한다. 들여쓰기는 ASCII space만 허용하고 tab은 `RSPDL-KO-LEX-001` 오류다.

빈 줄과 첫 non-space 문자가 `#`인 주석 줄은 들여쓰기 계산에서 제외한다. 유효한 줄의 들여쓰기 열이 증가하면 `INDENT`, 감소하면 하나 이상의 `DEDENT`를 생성한다. 기존 들여쓰기 열과 일치하지 않는 감소는 `RSPDL-KO-LEX-002` 오류다.

### 3.2 어휘 token

```ebnf
annotation-keyword = "@모듈" ;

canonical-id =
    "(", id-character, { id-character }, ")" ;

bare-name =
    name-character, { name-character | " " } ;

quoted-name =
    "`", quoted-character, { quoted-character }, "`" ;

surface-name =
    bare-name | quoted-name ;

integer-literal =
    "0" | ["-"], nonzero-digit, { digit } ;

boolean-literal =
    "참" | "거짓" ;

string-literal =
    JSON-string ;

id-character =
    "a".."z" | "0".."9" | "_" | "." ;

name-character =
    any Unicode scalar value except control characters,
    whitespace, "`", "(", ")", ":", ".", "#", "[", or "]" ;

quoted-character =
    any Unicode scalar value except control characters or "`" ;

digit = "0".."9" ;
nonzero-digit = "1".."9" ;
uppercase-ascii-letter = "A".."Z" ;

surface-reference = surface-name ;
model-reference = surface-reference ;
field-reference = surface-reference ;
role-reference = surface-reference ;
action-reference = surface-reference ;
relation-reference = surface-reference ;
enum-value-reference = surface-reference ;
```

`canonical-id`는 선언에만 나타난다. module을 제외한 짧은 ID는 module-local ID이며 compiler가 module ID로 한정한다. 이미 점을 포함한 qualified ID도 호환을 위해 허용한다. 여러 어절의 일반 표시 이름은 그대로 쓸 수 있다. 괄호, 콜론, 마침표, `#`처럼 어휘 경계와 충돌하는 문자가 포함된 표시 이름은 backtick으로 감싼다. multi-word bare name의 marker는 마지막 어절에 붙어야 하며, quoted name의 marker는 닫는 backtick 뒤의 별도 token으로 쓴다. 각 reference는 문법상 같은 `surface-reference`지만 lowering 시 해당 위치의 model, field, role, action 또는 enum value namespace에서 정확히 하나의 선언으로 해석되어야 한다.

## 4. Keyword

### 4.1 선언 형식과 annotation 제한

| 형식 | 선언 대상 | 블록 |
| --- | --- | --- |
| `@모듈` | 문서의 단일 module | 없음 |
| 열거형 문장 | 사용자 정의 enum | 무표식 enum value 한 개 이상 |
| 데이터 모델 header | JSON record model | 무표식 field 한 개 이상 |
| 역할·행동 문장 | policy role과 field action | 없음 |
| 관계 문장 | 방향을 명시한 unary/binary typed relation | 없음 |
| 관계 규칙 문장 | 존재, cardinality와 같은 signature의 relation group 의미 | 없음 |

`@모듈` 외 `@열거형`, `@개체`, `@역할`, `@행동`, `@관계`, `@필수`와 그 밖의 모든 `@...` 최상위 표현은 문법 오류다. 특히 빈 데이터 모델을 `@개체`로 우회할 수 없다. 현재 data model은 field를 하나 이상 가져야 하며, 추상 sort가 필요하면 별도 construct와 RFC를 먼저 정의해야 한다.

### 4.2 타입과 한정자 keyword

`필수`와 `선택`은 field cardinality를 나타낸다. 내장 단순 타입 keyword는 `문자열`, `정수`, `불리언`, `소수`, `날짜`, `시간`, `날짜시간`, `기간`, `위도`, `경도`, `백분율`/`비율`, `좌표`, `지역 날짜시간`, `시간대 날짜시간`, `달력 기간`, `UUID`, `이메일`, `URL`, `전화번호`, `IP`, `CIDR`, `국가 코드`, `언어 코드`, `통화 코드`다. `통화(KRW)`, `수량(kg)`, `목록(문자열)`, `집합(문자열)`, `맵(문자열, 문자열)`, `참조(payment)`는 parameter가 타입 동일성에 참여한다. 그 밖의 단순 type reference는 선언된 열거형의 표시 이름이어야 한다.

### 4.3 문장 marker

`의`, `은`/`는`, `와`/`과`, `을`/`를`은 문장 안에서 참조의 경계를 결정하는 구조 marker다. `보다`, `이상이어야`, `이하여야`, `이어야`, `같아야`, `달라야`, `할 수 있다`, `할 수 없다`는 지원되는 연산과 정책 효과를 결정한다.

### 4.4 독립 문장 readability

각 최상위 문장은 다른 선언의 IR을 열어 보지 않아도 그 문장의 의도를 식별할 수 있어야 한다. 특히 binary relation 선언은 source model, target model, relation 이름과 방향을 모두 포함한다. `required`와 `unique` 문장은 cardinality가 적용되는 anchor model과 relation을 함께 적는다. `소유자(owner): 프로젝트, 사용자`처럼 endpoint 역할과 문장 의미를 추측하게 만드는 signature 표기는 지원하지 않는다.

## 5. 구문 문법

### 5.1 선언

```ebnf
document =
    module-declaration, newline,
    { newline | declaration } ;

module-declaration =
    "@모듈", surface-name, canonical-id ;

local-id =
    canonical-id ;

declaration =
      enum-declaration
    | data-model-declaration
    | relation-declaration
    | relational-constraint-declaration
    | constraint-declaration
    | role-declaration
    | action-declaration
    | policy-declaration ;

enum-declaration =
    surface-name, local-id, ("은" | "는"),
    "다음", "값", "중", "하나다", ".", newline,
    indent, enum-value, { enum-value }, dedent ;

enum-value =
    surface-name, local-id, newline ;

data-model-declaration =
    surface-name, local-id, ("은" | "는"),
    "다음", "필드들로", "구성되어", "있다", ".", newline,
    indent, field-declaration, { field-declaration }, dedent ;

field-declaration =
    surface-name, local-id, ":",
    ("필수" | "선택"), type-reference, newline ;

type-reference =
      simple-type
    | "통화", "(", currency-code, ")"
    | "수량", "(", unit-id, ")"
    | "참조", "(", model-reference, ")"
    | ("목록" | "집합"), "(", simple-type, ")"
    | "맵", "(", map-key-type, ",", simple-type, ")"
    | surface-name ;

simple-type =
      "문자열" | "정수" | "불리언" | "소수"
    | "날짜" | "시간" | "날짜시간" | "기간"
    | "위도" | "경도" | "백분율" | "비율" | "좌표"
    | "지역 날짜시간" | "시간대 날짜시간" | "달력 기간"
    | "UUID" | "이메일" | "URL" | "전화번호" | "IP" | "CIDR"
    | "국가 코드" | "언어 코드" | "통화 코드" ;

map-key-type =
      "문자열" | "UUID" | "이메일" | "URL" | "전화번호"
    | "IP" | "CIDR" | "국가 코드" | "언어 코드" | "통화 코드" ;

currency-code = uppercase-ascii-letter, uppercase-ascii-letter, uppercase-ascii-letter ;
unit-id = "kg" | "g" | "m" | "km" | "s" | "ms" ;

relation-declaration =
      model-reference, ("은" | "는"),
      model-reference, ("을" | "를"),
      surface-name, local-id, ("로" | "으로"),
      "가질", "수", "있다", ".", newline
    | model-reference, ("은" | "는"),
      surface-name, local-id, "에", "해당할", "수", "있다", ".", newline ;

relational-constraint-declaration =
      model-reference, ("은" | "는"),
      "하나", "이상", "존재해야", "한다", ".", newline
    | "모든", model-reference, ("은" | "는"),
      relation-reference, ("을" | "를"),
      "하나", "이상", "가져야", "한다", ".", newline
    | "각", model-reference, ("은" | "는"),
      relation-reference, ("을" | "를"),
      "최대", "하나만", "가질", "수", "있다", ".", newline
    | relation-list, "중", "둘", "이상은",
      "동시에", "성립할", "수", "없다", ".", newline
    | relation-list, "중", "하나", "이상은",
      "항상", "성립해야", "한다", ".", newline
    | topic-relation-list,
      "동시에", "성립할", "수", "있다", ".", newline ;

relation-list =
    relation-reference, ",", relation-reference,
    { ",", relation-reference } ;

topic-relation-list =
    relation-reference, ",", relation-reference,
    { ",", relation-reference }, ("은" | "는") ;

constraint-declaration =
    constraint-statement, newline ;

role-declaration =
    surface-name, local-id, ("은" | "는"), "역할이다", ".", newline ;

action-declaration =
    surface-name, local-id, ("은" | "는"), "행동이다", ".", newline ;

policy-declaration =
    policy-statement, newline ;
```

parser는 같은 블록에서 동일한 들여쓰기 열을 요구한다. 유효한 들여쓰기 폭은 임의로 정할 수 있지만 formatter는 공백 네 칸으로 정규화한다.

현재 한국어 표면 문법의 `목록`, `집합`, `맵` parameter는 위의 비매개변수 단순 타입 한 단계만 받는다. Canonical IR은 중첩 collection과 typed reference element를 표현할 수 있지만, 이를 한국어 source가 이미 지원하는 것으로 해석하지 않는다.

```rspdl
결제(payment)는 다음 필드들로 구성되어 있다.
    금액(amount): 필수 통화(KRW)

고객(customer)은 다음 필드들로 구성되어 있다.
    외부 ID(external_id): 필수 UUID
    태그(tags): 필수 집합(문자열)
    설정(settings): 필수 맵(문자열, 문자열)
    기본 결제(default_payment): 필수 참조(payment)
```

### 5.2 제약 문장

```ebnf
constraint-statement =
    model-reference, "의", constraint-expression, "." ;

constraint-expression =
      field-reference, ("은" | "는"), ordered-comparison
    | field-reference, ("은" | "는"), equality-comparison
    | field-reference, ("와" | "과"),
      field-reference, ("은" | "는"), field-relation ;

ordered-comparison =
      ordered-literal, "보다", ("커야" | "작아야"), "한다"
    | ordered-literal, ("이상이어야" | "이하여야"), "한다" ;

ordered-literal =
    integer-literal | string-literal ;

equality-comparison =
    literal, "이어야", "한다" ;

field-relation =
    ("같아야" | "달라야"), "한다" ;

literal =
      string-literal
    | integer-literal
    | boolean-literal
    | enum-value-reference ;
```

지원 예시는 다음과 같다.

```text
비용 신청의 금액은 0보다 커야 한다.
비용 신청의 금액은 0 이상이어야 한다.
비용 신청의 금액은 100보다 작아야 한다.
비용 신청의 금액은 100 이하여야 한다.
비용 신청의 승인 상태는 승인됨이어야 한다.
비용 신청의 신청자와 승인자는 같아야 한다.
비용 신청의 신청자와 승인자는 달라야 한다.
예약의 시작일은 "2026-08-13" 이상이어야 한다.
예약의 시작 시각은 "09:00:00"보다 커야 한다.
장소의 위도는 "37.5665"이어야 한다.
```

확장 scalar literal은 JSON string 안에 [Typed Domains and Logic Core](0002-typed-domains-and-logic-core.md)의 representation을 쓴다. 왼쪽 field의 resolved type에 따라 같은 string token을 소수·시간·위치·통화·비율·수량 또는 refinement canonical value로 변환한다. 이를 일반 문자열과 자동 형변환하는 것으로 해석하지 않는다. 예를 들어 `날짜` field의 `"2026-02-30"`은 string equality가 아니라 잘못된 date literal이고, `통화(KRW)` field의 `"10 USD"`는 currency type mismatch다.

### 5.3 정책 문장

```ebnf
policy-statement =
    role-reference, ("은" | "는"),
    model-reference, "의",
    field-reference, ("을" | "를"),
    action-reference, "할", "수",
    ("있다" | "없다"), "." ;
```

`있다`는 allow, `없다`는 deny를 뜻한다.

## 6. 정적 의미

### 6.1 이름과 ID

- module 아래의 enum, record model, relation, role과 action은 짧은 local ID를 사용할 수 있다.
- 짧은 top-level ID `request`는 공통 analyzer가 `<module-id>.request`로 qualification한다. 점이 포함된 ID는 이미 qualified된 것으로 취급한다.
- lowering된 top-level canonical ID는 module 안에서 중복될 수 없다.
- constraint, policy와 relational meta-rule은 source ID나 표시 이름을 갖지 않는다.
- `rspdl-ko`는 이 규칙들에 Locale별 ID를 만들지 않고 anonymous declaration으로 lowering한다.
- 한국어 frontend가 표시 이름 reference를 선언 stable ID로 연결하고, 공통 linker가 이를 Canonical ID로 검증·qualification한 뒤 semantic identity의 UTF-8 byte sequence에 FNV-1a 64-bit를 적용해 `constraint_<hex>`, `policy_<hex>`, `relation_rule_<hex>`를 만든다. `<hex>`는 leading zero를 포함한 16자리 소문자 hexadecimal이며 canonical ID는 module ID로 한정한다.
- constraint identity는 `model-id NUL operand NUL operator NUL operand` 순서다. field operand에는 canonical field ID를 사용하고 literal은 canonical value representation을 사용한다.
- policy identity는 `role-id NUL model-id NUL field-id NUL action-id NUL effect` 순서며 effect는 `allow` 또는 `deny`다.
- relation rule identity는 정규화된 rule kind와 canonical model 또는 정렬·중복 제거된 relation ID 목록으로 구성된다.
- `expense.request`의 `expense.request.amount > 0`은 `expense.constraint_72fbbd5f8aa621cb`, `expense.manager`가 같은 필드를 `expense.change`하도록 허용하는 정책은 `expense.policy_45439f1d15749ca3`인 known vector다.
- 내부 rule ID는 Locale display text, 공백, 들여쓰기와 source 위치에 의존하지 않고 stable ID와 의미가 달라질 때만 함께 달라진다.
- enum value ID는 해당 enum 안에서, field ID는 해당 model 안에서 고유해야 한다.
- 자연어 문장의 참조는 선언된 표시 이름과 정확히 일치해야 한다.
- 같은 namespace에 표시 이름이 중복되면 참조가 모호하므로 compile 오류다.
- lowering 후 enum value와 field canonical ID는 각각 `<enum-id>.<local-value-id>`, `<model-id>.<local-field-id>`다.

### 6.2 공통 타입 검사

- ordered comparison은 `정수`, `소수`, `날짜`, `시간`, `날짜시간`, `기간`, `위도`, `경도`, `지역 날짜시간`, `시간대 날짜시간`, `통화`, `백분율`, `수량`의 같은 타입 operand 사이에만 적용한다. `통화`는 같은 통화 코드끼리만, `수량`은 같은 차원끼리만 비교한다 — 다르면 `RSPDL-TYPE-001` 오류다.
- equality literal의 타입은 field 타입과 같아야 한다.
- enum literal은 field가 참조하는 enum에 선언된 표시 이름이어야 한다.
- field-to-field equality는 양쪽 field 타입이 같아야 한다.
- `문자열`, `불리언`, enum에 ordered comparison을 적용하거나 서로 다른 ordered type을 섞으면 `RSPDL-TYPE-001` 오류다.
- 위도와 경도는 서로 다른 타입이며 바꿔 쓸 수 없다. 각각 `[-90, 90]`, `[-180, 180]` 밖의 literal은 `RSPDL-TYPE-001` 오류다.
- policy가 참조하는 role, model, field와 action은 모두 선언되어야 한다.
- relation parameter는 선언된 model이어야 하며 현재 arity는 1 또는 2다.
- `required`와 `unique` 문장은 anchor model을 직접 이름으로 적어야 하며, 그 model은 binary relation의 첫 parameter와 같아야 한다.
- `exclusive`, `exhaustive`, `coexistent` group의 relation은 parameter model과 순서가 같아야 한다.
- 동일 group에 `exclusive`와 `coexistent`를 함께 선언할 수 없다.

위 규칙은 한국어 frontend가 아니라 공통 analyzer가 모든 `UnlinkedModule`에 동일하게 적용한다.

조사 선택이 부자연스러우면 `RSPDL-KO-W001` warning을 만들지만 compile을 막지 않는다.

## 7. 실행 의미

### 7.1 JSON binding

`rspdl-compiler`는 외부 JSON의 `records`, `role_assignments`, `action_requests`를 `SemanticModule`에 연결한다. 필수 field의 누락 또는 `null`은 backend 실행 전 입력 오류다. 선택 field가 없거나 `null`이면 그 field를 참조하는 제약은 해당 record에 적용하지 않는다.

`날짜`, `시간`, `날짜시간`, `기간`, `지역 날짜시간`, `시간대 날짜시간`, `달력 기간` field는 canonical text를 JSON string으로 받는다. 시간대 날짜시간은 explicit offset과 IANA zone을 함께 받아 pinned time-zone data와 일치하지 않는 DST offset을 거부한다. `소수`, `위도`, `경도`는 정밀도를 보존하기 위해 JSON string을 기준 형식으로 사용한다. JSON number는 정수만 허용한다 — 소수 자릿수를 가진 JSON number는 파서가 이미 배정밀도로 반올림한 뒤라 정확한 값이 아니므로 입력 오류다. 어느 형식이든 canonical 표기여야 하며 `+42`, `042`, `-0` 은 하나의 값에 여러 바이트 표현을 만들므로 거부한다.

`통화`, `백분율`/`비율`, `수량`, `좌표`와 문자열 refinement도 canonical string을 받는다. 통화와 수량은 field의 currency/dimension과 다른 값을 거부하고, UUID·IP·CIDR 등은 타입별 lexical/canonical validation을 적용한다. `목록`과 `집합`은 JSON array, `맵`은 JSON object, `참조`는 target record ID string을 받는다. 모든 원소·key·value를 재귀적으로 타입 검사하고 집합 중복을 거부한다. Typed reference의 target model 존재는 compile 시 common analyzer가 검사하고, runtime 에서는 **같은 입력에 실린 target model의 record ID 목록**과 대조해 없는 대상을 `RSPDL-INPUT-026` 으로 거부한다 (`action_requests` 의 record 확인과 같은 규칙이다). 입력 밖의 데이터베이스나 외부 저장소를 조회하지는 않는다.

### 7.2 제약 backend

Runtime constraint는 concrete canonical value의 equality와 type-specific ordered comparison을 직접 정확하게 실행한다. 확장 scalar를 finite active domain 없이 symbolic model finding에 사용하면 backend가 지원하지 않는 construct로 보고하며 integer나 string theory로 근사하지 않는다.

### 7.3 정책 backend

role assignment, action request와 선언된 무조건 policy를 직접 대조한다. 각 action request는 다음 중 하나로 분류한다.

| 상태 | 의미 |
| --- | --- |
| `allowed` | allow policy만 일치 |
| `denied` | deny policy만 일치 |
| `conflict` | allow와 deny policy가 모두 일치 |
| `unmatched` | 일치하는 policy가 없음 |

allow와 deny 사이의 우선순위는 0.1에서 정의하지 않는다. 결과에는 내부 생성된 policy canonical ID를 결정적인 순서로 포함한다.

### 7.4 관계 bounded model finder

`rspdl model <file> --scope <n>`은 실제 JSON record 없이 model별 최대 `n`개의 가상 entity slot과 relation tuple을 가정한다. `SAT`은 가상 witness, `UNSAT_WITHIN_BOUND`는 해당 scope에 한정된 최소 규칙 증거, `UNKNOWN`은 이유를 반환한다. 의미 손실 없이 lowering할 수 없는 construct는 solver 실행 전에 `UNSUPPORTED`로 반환한다. Scope 한정 UNSAT을 전역 모순으로 표현하지 않는다.

`--scope`는 eager grounding 안전 한계 안의 `1..=32`만 허용한다. `0`, `33` 이상 또는 정수가 아닌 값은 solver 실행 전에 configuration error가 된다. 이 상한은 제품 데이터 세계의 의미적 최대 크기가 아니라 구현 안전 한계다.

## 8. 진단과 적합성

잘못된 dedent가 발생하면 해당 블록을 진단하고 다음 최상위 선언이나 규칙 문장에서 복구한다. parse, compile, check 결과는 source span과 안정적인 Rule ID를 포함하고 canonical ID와 source offset 기준의 결정적인 순서로 직렬화한다.

CLI 계약은 다음과 같다.

```text
rspdl parse <file>... --json
rspdl compile <file>... --json
rspdl model <file> --scope <n> --json
rspdl check <file>... --data <file> --json
rspdl format <file>...
```

정상 통과와 `SAT`은 종료 코드 `0`, constraint/policy finding 또는 `UNSAT_WITHIN_BOUND`는 `2`, 문법·입력·backend 오류와 `UNKNOWN`, `UNSUPPORTED`는 `1`이다.

복수 source는 파일 경로의 정렬 순서로 처리하며 각 파일은 독립된 `@모듈` 선언을 가진다. 같은 module ID가 여러 파일에 선언되면 linker 오류다. 단일 파일의 JSON 출력 계약은 유지하고, 복수 파일의 parse·compile·check 결과에는 진단 위치를 구분할 수 있도록 source 경로가 포함된다.

적합한 구현은 다음 성질을 만족해야 한다.

- `IR(parse(format(source))) == IR(parse(source))`
- source 입력 순서와 반복 실행에 대해 IR 및 진단 직렬화가 결정적임
- parser가 임의 UTF-8 입력에서 panic하지 않음
- formatter가 공백 네 칸, 자연어 header와 무표식 CFG 항목 형식으로 멱등 정규화함
