---
id: controlled-korean-surface-grammar
title: Controlled Korean Surface Grammar
type: rfc
status: proposed
version: "0.1"
summary: Proposes a deterministic Korean surface grammar that treats particles and endings as structural markers rather than morphology.
topics:
  - ko-KR
  - controlled-language
  - surface-grammar
  - cfg
  - parser
  - diagnostics
related:
  - rspdl-language-prd
  - rust-korean-first-frontend
  - rspdl-compiler-architecture
last_updated: "2026-07-27"
owners:
  - rspdl-maintainers
target_spec: "0.1.0"
---

# Controlled Korean Surface Grammar

## 상태와 범위

이 RFC는 Proposed 상태다.

다음 상위 결정은 이미 승인됐다.

- 한국어는 최초 구현 Locale이다.
- 자유 한국어를 분석하지 않고 RSPDL이 정의한 표면 형식만 인식한다.
- 형태소 분석기와 품사 분석기를 compiler correctness 경로에 사용하지 않는다.
- 조사와 종결 표현은 한국어 문법 판정 대상이 아니라 RSPDL 구조 marker다.

이 RFC는 최초 정책 문형, scanner와 parser의 경계, lint와 formatter의 책임을 제안한다. Actor, Entity, Screen 같은 전체 선언 문법은 후속 RFC에서 정의한다.

## 목표

- 사람이 읽었을 때 한국어 문장에 가까운 제한 문형을 제공한다.
- 입력을 하나의 슬롯 구조로 결정론적으로 해석한다.
- CFG로 문법을 기술하고 Rust parser로 구현할 수 있게 한다.
- 문법 오류, 표현 품질과 의미 오류를 서로 다른 진단 계층으로 분리한다.
- 형태소 분석 없이 정확한 source span과 오류 복구를 제공한다.

## 비목표

- 자유 형식 한국어 문장 이해
- 생략된 주체나 목적어 추론
- 동의어, 활용형 또는 문맥상 의미 추측
- 국문법 전체의 적절성 판정
- AI를 이용한 문장 보정

## 용어

### Surface marker

RSPDL 문법 슬롯의 경계를 표시하는 고정 문자열이다. 이름은 실제 국어 품사보다 RSPDL 내부 역할을 기준으로 한다.

| Marker | 허용 표면형 | 역할 |
| --- | --- | --- |
| `SubjectMarker` | `은`, `는` | Subject slot 종료 |
| `ObjectMarker` | `이`, `가` | Object slot 종료 |
| `PossessiveMarker` | `의` | Object와 Resource 연결 |
| `ResourceMarker` | `을`, `를` | Resource slot 종료 |
| `ActionMarker` | `할` | Action slot 종료 |
| `AllowEnding` | `수 있다` | `ALLOW` effect |
| `DenyEnding` | `수 없다` | `DENY` effect |

`은`과 `는` 중 국문법상 어떤 표현이 자연스러운지는 marker 종류에 영향을 주지 않는다.

### Surface reference

문장에서 Actor, Object, Resource 또는 Action을 가리키는 표면 이름이다. Locale AST까지는 `SurfaceRef`로 보존하며, 별도의 이름 해석을 거쳐 stable machine ID에 연결한다.

이 RFC는 stable machine ID를 선언하고 한국어 표시 이름에 연결하는 구체 문법을 정하지 않는다.

## 제안 문형

초기 정책은 다음 두 문형을 후보로 둔다.

```text
관리자는 문서가 내용을 수정할 수 있다.
관리자는 주문의 배송지를 수정할 수 없다.
```

두 문형은 모두 다음 Locale AST로 정규화한다.

```text
PolicyAst {
    subject,
    object,
    resource,
    action,
    effect,
}
```

제안 EBNF는 다음과 같다.

```ebnf
policy-statement =
    subject-clause,
    (marked-object-clause | possessive-object-clause),
    action-clause,
    effect-clause,
    period ;

subject-clause =
    surface-reference, subject-marker ;

marked-object-clause =
    surface-reference, object-marker,
    surface-reference, resource-marker ;

possessive-object-clause =
    surface-reference, possessive-marker,
    surface-reference, resource-marker ;

action-clause =
    surface-reference, action-marker ;

effect-clause =
    "수", ("있다" | "없다") ;

subject-marker =
    "은" | "는" ;

object-marker =
    "이" | "가" ;

possessive-marker =
    "의" ;

resource-marker =
    "을" | "를" ;

action-marker =
    "할" ;

period =
    "." ;
```

초기 제안에서는 문장 경계를 명확히 하기 위해 마침표를 필수로 둔다. 줄바꿈 종결 또는 선택적 마침표가 필요한지는 fixture를 통해 별도로 결정한다.

## Scanner와 parser의 경계

한국어 frontend는 일반적인 형태소 token을 만들지 않는다.

Scanner는 다음 원시 token만 만든다.

- `RawWord`
- `QuotedIdentifier`
- `Period`
- 주석과 공백을 포함한 trivia
- 알 수 없는 문자

Parser가 특정 슬롯을 기대할 때 해당 위치에서만 접미 marker를 분리한다.

```text
RawWord("관리자는")
  -> SurfaceRef("관리자") + SubjectMarker("는")
```

```text
RawWord("문서가")
  -> SurfaceRef("문서") + ObjectMarker("가")
```

```text
RawWord("수정할")
  -> SurfaceRef("수정") + ActionMarker("할")
```

이 방식은 동일한 문자열을 모든 위치에서 형태소처럼 분해하지 않는다. Parser의 grammar state와 허용 marker 집합이 유일한 분해를 결정한다.

고정 종결 표현은 scanner에서 하나의 거대 token으로 만들지 않는다. Parser가 action에 붙은 `할`을 분리한 뒤 `수`, `있다|없다` token sequence를 조합해 중간 단어가 누락된 위치를 정확히 진단한다.

## 식별자

초기 문법은 bare identifier와 backtick quoted identifier를 제안한다.

```ebnf
surface-reference =
    bare-identifier
  | quoted-identifier ;
```

```text
관리자는 주문의 배송지를 수정할 수 있다.
`현장 관리자`는 `작업 문서`의 `승인 상태`를 수정할 수 있다.
```

인용된 내용 내부에서는 marker를 분리하지 않는다. 닫는 backtick 뒤에 붙은 marker만 현재 문법 위치에 따라 인식한다.

인용 identifier의 escape 규칙과 Unicode normalization은 후속 lexical grammar에서 확정한다.

## Parser, lint, formatter의 책임

### Parser

Parser는 다음을 검사한다.

- 문형과 슬롯 순서
- 필수 marker
- 허용된 effect ending
- 문장 종결
- 각 슬롯의 유일한 추출 가능성

다음 입력은 parse 성공이다.

```text
사용자은 문서가 내용을 수정할 수 있다.
```

`사용자은`은 `SurfaceRef("사용자") + SubjectMarker("은")`으로 유일하게 분해된다.

### Surface linter

Surface linter는 의미를 바꾸지 않는 한국어 표현 품질을 검사한다.

```text
RSPDL-KO-W001
'사용자은'보다 '사용자는'이 자연스럽습니다.
```

받침 판별은 한글 음절 Unicode 범위의 종성 index로 계산할 수 있으며 형태소 분석이 아니다. 이 경고는 compile 성공 여부와 Canonical IR에 영향을 주지 않는다.

### Formatter

Formatter는 CST의 의미를 보존하면서 권장 marker와 공백을 출력한다.

```text
사용자은 문서가 내용을 수정할 수 있다.
```

위 입력은 다음과 같이 정규화할 수 있다.

```text
사용자는 문서가 내용을 수정할 수 있다.
```

Formatter 전후의 Canonical IR은 동일해야 한다.

## Lowering

Locale AST는 표면 표현을 제거하고 공통 구조로 lowering한다.

```text
PolicyAst
  subject: SurfaceRef("관리자")
  object: SurfaceRef("주문")
  resource: SurfaceRef("배송지")
  action: SurfaceRef("수정")
  effect: Deny
```

이름 해석 후 Canonical IR은 Locale에 독립적인 stable ID를 사용한다.

```json
{
  "subject": "actor.admin",
  "object": "entity.order",
  "resource": "field.order.shipping_address",
  "action": "action.update",
  "effect": "deny"
}
```

동일 의미의 미래 `en-US` 문형도 같은 IR을 생성해야 한다.

## 진단 계층

| 계층 | 예시 ID | 책임 |
| --- | --- | --- |
| Scanner | `RSPDL-KO-LEX-*` | 닫히지 않은 인용, 알 수 없는 문자 |
| Parser | `RSPDL-KO-SYN-*` | marker 누락, 잘못된 슬롯 순서, 종결 누락 |
| Surface lint | `RSPDL-KO-W*` | 부자연스러운 조사, 비권장 표면 표현 |
| Name resolution | `RSPDL-NAME-*` | 알 수 없는 Actor, Entity, Field, Action |
| Semantic analysis | `RSPDL-SEM-*` | 타입 오류와 영역 간 모순 |
| Policy rules | `RSPDL-POLICY-*` | 허용·금지 충돌과 정책 누락 |

모든 진단은 Rule ID, severity, message key, primary span과 관련 symbol을 구조화해 반환한다.

## Conformance fixture

각 production과 규칙에는 다음 사례가 필요하다.

- 정상 문형
- 각 marker 누락
- 잘못된 marker 위치
- bare identifier와 quoted identifier
- identifier 자체가 marker 문자로 끝나는 경우
- `수정할`처럼 Action과 `할`이 붙어 있는 경우
- `할 수 있다`와 `할 수 없다`의 부분 누락
- 부자연스러운 조사지만 parse 가능한 경우
- format 후 의미가 유지되는 경우
- 같은 입력을 반복 처리했을 때 동일한 AST, IR과 진단 순서
- 동일 의미의 미래 `en-US` fixture와 IR 동등성

예시 fixture 묶음은 다음과 같다.

```text
conformance/ko-KR/policy/capability-basic/
├── case.yaml
├── input.rspdl
├── expected.ast.json
├── expected.ir.json
└── expected.diagnostics.json
```

## 구현 권고

v0.x에서는 token 기반 handwritten recursive-descent parser를 우선 권고한다.

- 문형별 오류와 복구 지점을 직접 제어할 수 있다.
- parser가 기대하는 marker 집합으로 suffix를 분리할 수 있다.
- 문법 변경이 잦은 동안 생성 parser의 제약을 피할 수 있다.

EBNF는 규범 문법으로 유지하되 EBNF와 구현의 일치를 conformance fixture로 검증한다. 문법이 안정된 뒤 parser generator 도입을 다시 평가할 수 있다.

## 미결정 사항

- 두 후보 정책 문형을 모두 0.1에 포함할지 여부
- 마침표를 필수로 유지할지 여부
- quoted identifier의 escape와 Unicode normalization
- stable machine ID와 한국어 표시 이름의 선언·참조 문법
- 여러 어절로 된 Action의 허용 범위
- 주석과 줄바꿈 규칙
- 선언문, 데이터 모델 문장과 유저 플로우 문형
