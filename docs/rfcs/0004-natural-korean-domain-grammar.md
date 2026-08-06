---
id: natural-korean-domain-grammar
title: Korean Domain Frontend Language Specification
type: rfc
status: implemented
version: "0.1"
summary: Defines sparse annotation-led blocks, natural Korean data headers, indentation-based CFG items, and controlled constraint and policy sentences implemented by rspdl-ko.
topics:
  - ko-KR
  - controlled-language
  - data-model
  - constraints
  - policies
  - cfg
related:
  - controlled-korean-surface-grammar
  - typed-domains-and-logic-core
  - rspdl-compiler-architecture
problem_refs:
  - data-lifecycle-modeling-gap
  - policy-consistency-blind-spots
last_updated: "2026-08-02"
owners:
  - rspdl-maintainers
target_spec: "0.1.0"
---

# Korean Domain Frontend Language Specification

## 1. 범위

이 문서는 `rspdl-ko` 0.1 frontend의 규범 문법과 의미를 정의한다. 데이터와 열거형의 header는 자연스러운 한국어 문장이고, 들여쓴 field와 enum value는 별도 `@` 없이 CFG 항목으로 작성한다. 제약과 정책은 이름이나 source ID가 없는 독립적인 최상위 문장이다.

관계, 컬렉션, 유저 플로우, 조건부 정책, 일반 `AND`/`OR`/`NOT`, 모듈 import와 자유 한국어 해석은 0.1 범위에 포함하지 않는다.

## 2. 전체 예시

```text
@모듈 비용 승인(expense)

@열거형 비용 상태(status)는 다음 값 중 하나다.
    작성 중(draft)
    제출됨(submitted)
    승인됨(approved)

비용 신청(request)은 다음 필드들로 구성되어 있다.
    식별자(id): 필수 문자열
    신청자(applicant): 필수 문자열
    승인자(approver): 선택 문자열
    금액(amount): 필수 정수
    승인 상태(status): 필수 비용 상태

비용 신청의 금액은 0보다 커야 한다.
비용 신청의 신청자와 승인자는 달라야 한다.

@역할 회계 관리자(accounting_manager)
@행동 변경(change)

회계 관리자는 비용 신청의 승인 상태를 변경할 수 있다.
```

`expense`는 이 문서에서 처음 선언되는 module ID다. 그 아래의 짧은 ID `request`, `status`, `change`는 lowering할 때 각각 `expense.request`, `expense.status`, `expense.change`가 된다. field와 enum value는 부모 ID 아래에서 한 단계 더 한정된다.

완전한 문장인 데이터·열거형 header와 제약·정책 문장에는 마침표가 필요하다. 단순 annotation 선언과 들여쓴 CFG 항목에는 마침표를 붙이지 않는다. 제약과 정책의 canonical ID는 정규화된 문장 의미에서 내부 생성한다.

## 3. 어휘 구조

### 3.1 입력 문자와 줄

source는 UTF-8 text다. 줄바꿈은 LF 또는 CRLF를 허용하며 parser 내부에서는 논리적 `NEWLINE`으로 취급한다. 들여쓰기는 ASCII space만 허용하고 tab은 `RSPDL-KO-LEX-001` 오류다.

빈 줄과 첫 non-space 문자가 `#`인 주석 줄은 들여쓰기 계산에서 제외한다. 유효한 줄의 들여쓰기 열이 증가하면 `INDENT`, 감소하면 하나 이상의 `DEDENT`를 생성한다. 기존 들여쓰기 열과 일치하지 않는 감소는 `RSPDL-KO-LEX-002` 오류다.

### 3.2 어휘 token

```ebnf
annotation-keyword =
      "@모듈" | "@열거형"
    | "@역할" | "@행동" ;

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

surface-reference = surface-name ;
model-reference = surface-reference ;
field-reference = surface-reference ;
role-reference = surface-reference ;
action-reference = surface-reference ;
enum-value-reference = surface-reference ;
```

`canonical-id`는 선언에만 나타난다. module을 제외한 짧은 ID는 module-local ID이며 compiler가 module ID로 한정한다. 이미 점을 포함한 qualified ID도 호환을 위해 허용한다. 여러 어절의 일반 표시 이름은 그대로 쓸 수 있다. 괄호, 콜론, 마침표, `#`처럼 어휘 경계와 충돌하는 문자가 포함된 표시 이름은 backtick으로 감싼다. multi-word bare name의 marker는 마지막 어절에 붙어야 하며, quoted name의 marker는 닫는 backtick 뒤의 별도 token으로 쓴다. 각 reference는 문법상 같은 `surface-reference`지만 lowering 시 해당 위치의 model, field, role, action 또는 enum value namespace에서 정확히 하나의 선언으로 해석되어야 한다.

## 4. Keyword

### 4.1 선언 keyword

| Keyword | 선언 대상 | 블록 |
| --- | --- | --- |
| `@모듈` | 문서의 단일 module | 없음 |
| `@열거형` | 사용자 정의 enum | 무표식 enum value 한 개 이상 |
| 데이터 모델 header | JSON record model | 무표식 field 한 개 이상 |
| `@역할` | policy subject role | 없음 |
| `@행동` | field action | 없음 |

`@`는 module, 열거형, 역할, 행동 선언에만 사용한다. 데이터 모델은 자연스러운 header 문장으로 선언하며 `@데이터`, `@필드`, `@값`, `@제약`, `@정책`은 keyword가 아니다.

### 4.2 타입과 한정자 keyword

`필수`와 `선택`은 field cardinality를 나타낸다. 내장 타입 keyword는 `문자열`, `정수`, `불리언`이다. 그 밖의 type reference는 선언된 열거형의 표시 이름이어야 한다.

### 4.3 문장 marker

`의`, `은`/`는`, `와`/`과`, `을`/`를`은 문장 안에서 참조의 경계를 결정하는 구조 marker다. `보다`, `이상이어야`, `이하여야`, `이어야`, `같아야`, `달라야`, `할 수 있다`, `할 수 없다`는 지원되는 연산과 정책 효과를 결정한다.

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
    | constraint-declaration
    | role-declaration
    | action-declaration
    | policy-declaration ;

enum-declaration =
    "@열거형", surface-name, local-id, ("은" | "는"),
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
    "문자열" | "정수" | "불리언" | surface-name ;

constraint-declaration =
    constraint-statement, newline ;

role-declaration =
    "@역할", surface-name, local-id, newline ;

action-declaration =
    "@행동", surface-name, local-id, newline ;

policy-declaration =
    policy-statement, newline ;
```

parser는 같은 블록에서 동일한 들여쓰기 열을 요구한다. 유효한 들여쓰기 폭은 임의로 정할 수 있지만 formatter는 공백 네 칸으로 정규화한다.

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
      integer-literal, "보다", ("커야" | "작아야"), "한다"
    | integer-literal, ("이상이어야" | "이하여야"), "한다" ;

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
```

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

- module 아래의 enum, model, role과 action은 짧은 local ID를 사용할 수 있다.
- 짧은 top-level ID `request`는 `<module-id>.request`로 lowering한다. 점이 포함된 ID는 이미 qualified된 것으로 취급한다.
- lowering된 top-level canonical ID는 module 안에서 중복될 수 없다.
- constraint와 policy는 source ID나 표시 이름을 갖지 않는다.
- constraint와 policy의 local ID는 아래 canonical serialization의 UTF-8 byte sequence에 FNV-1a 64-bit를 적용한 `constraint_<hex>`, `policy_<hex>` 형식이다. `<hex>`는 leading zero를 포함한 16자리 소문자 hexadecimal이며 canonical ID는 다시 module ID로 한정한다.
- v0.1 canonical serialization은 lowering 전의 정확한 display name byte sequence를 사용한다. Unicode normalization은 적용하지 않으며, NFC와 NFD처럼 다른 byte sequence는 다른 이름이다. lexer가 무시하는 공백, 들여쓰기와 source 위치는 serialization에 포함하지 않는다.
- constraint는 `model NUL operand NUL operator NUL operand` 순서다. field operand는 `field:<display-name>`, string literal은 `string:<JSON-string>`, integer literal은 `integer:<canonical-decimal>`, boolean literal은 `boolean:true|false`, named literal은 `named:<display-name>`이다. operator는 `equal`, `not_equal`, `less_than`, `less_than_or_equal`, `greater_than`, `greater_than_or_equal` 중 하나다.
- policy는 `role NUL model NUL field NUL action NUL effect` 순서며 effect는 `allow` 또는 `deny`다. `NUL`은 U+0000 byte 하나를 뜻한다.
- Known vectors: `항목 NUL field:값 NUL greater_than NUL integer:0`은 `constraint_cc0c5c741f5a3664`이고, `관리자 NUL 항목 NUL 값 NUL 변경 NUL allow`는 `policy_c9929d0f292dc92b`이다. 같은 token sequence를 만드는 공백 변화와 future Locale이 동일 serialization을 사용하면 같은 ID를 만든다.
- 내부 rule ID는 공백, 들여쓰기와 source 위치가 바뀌어도 동일하고, 규칙의 의미가 달라지면 함께 달라진다.
- enum value ID는 해당 enum 안에서, field ID는 해당 model 안에서 고유해야 한다.
- 자연어 문장의 참조는 선언된 표시 이름과 정확히 일치해야 한다.
- 같은 namespace에 표시 이름이 중복되면 참조가 모호하므로 compile 오류다.
- lowering 후 enum value와 field canonical ID는 각각 `<enum-id>.<local-value-id>`, `<model-id>.<local-field-id>`다.

### 6.2 타입 검사

- `보다`, `이상`, `이하` 비교는 정수 field에만 적용한다.
- equality literal의 타입은 field 타입과 같아야 한다.
- enum literal은 field가 참조하는 enum에 선언된 표시 이름이어야 한다.
- field-to-field equality는 양쪽 field 타입이 같아야 한다.
- policy가 참조하는 role, model, field와 action은 모두 선언되어야 한다.

조사 선택이 부자연스러우면 `RSPDL-KO-W001` warning을 만들지만 compile을 막지 않는다.

## 7. 실행 의미

### 7.1 JSON binding

`rspdl-compiler`는 외부 JSON의 `records`, `role_assignments`, `action_requests`를 `SemanticModule`에 연결한다. 필수 field의 누락 또는 `null`은 backend 실행 전 입력 오류다. 선택 field가 없거나 `null`이면 그 field를 참조하는 제약은 해당 record에 적용하지 않는다.

### 7.2 제약 backend

각 record와 constraint 조합마다 constraint의 부정을 Z3 문제로 만든다. 부정이 `SAT`이면 record는 제약을 위반하며 반례를 finding에 포함한다. `UNSAT`이면 통과하고 `Unknown` 또는 timeout은 backend 오류다.

### 7.3 정책 backend

role assignment와 action request를 fact로, policy를 allow 또는 deny Datalog rule로 변환한다. 각 action request는 다음 중 하나로 분류한다.

| 상태 | 의미 |
| --- | --- |
| `allowed` | allow policy만 일치 |
| `denied` | deny policy만 일치 |
| `conflict` | allow와 deny policy가 모두 일치 |
| `unmatched` | 일치하는 policy가 없음 |

allow와 deny 사이의 우선순위는 0.1에서 정의하지 않는다. 결과에는 내부 생성된 policy canonical ID를 결정적인 순서로 포함한다.

## 8. 진단과 적합성

잘못된 dedent가 발생하면 해당 블록을 진단하고 다음 최상위 선언이나 규칙 문장에서 복구한다. parse, compile, check 결과는 source span과 안정적인 Rule ID를 포함하고 canonical ID와 source offset 기준의 결정적인 순서로 직렬화한다.

CLI 계약은 다음과 같다.

```text
rspdl parse <file>... --json
rspdl compile <file>... --json
rspdl check <file>... --data <file> --json
rspdl format <file>...
```

정상 통과는 종료 코드 `0`, constraint 또는 policy finding은 `2`, 문법·입력·backend 오류는 `1`이다.

복수 source는 파일 경로의 정렬 순서로 처리하며 각 파일은 독립된 `@모듈` 선언을 가진다. 같은 module ID가 여러 파일에 선언되면 linker 오류다. 단일 파일의 JSON 출력 계약은 유지하고, 복수 파일의 parse·compile·check 결과에는 진단 위치를 구분할 수 있도록 source 경로가 포함된다.

적합한 구현은 다음 성질을 만족해야 한다.

- `IR(parse(format(source))) == IR(parse(source))`
- source 입력 순서와 반복 실행에 대해 IR 및 진단 직렬화가 결정적임
- parser가 임의 UTF-8 입력에서 panic하지 않음
- formatter가 공백 네 칸, 자연어 header와 무표식 CFG 항목 형식으로 멱등 정규화함
