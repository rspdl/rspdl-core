---
id: document-frontmatter-structure
title: 문서 머리말 구조 선언 - 정보구조, 화면 레이아웃과 화면 흐름
type: rfc
status: implemented
created: 2026-09-20
version: "1"
summary: Defines a YAML-subset document frontmatter that declares information architecture, semantic screen layout, and element-anchored screen paths, checked against sentence-declared screen operations.
topics:
  - information-architecture
  - screen-layout
  - wireframe
  - screen-flow
  - frontmatter
  - canonical-ir
related:
  - field-provenance-and-sum-derivation
  - natural-korean-domain-grammar
  - controlled-korean-surface-grammar
  - core-application-boundary
  - frontend-semantic-analysis-contract
  - rspdl-language-prd
  - workflow-completion-data-availability
problem_refs:
  - screen-structure-spec-divergence
  - data-lifecycle-modeling-gap
  - semantic-source-provenance-loss
last_updated: "2026-09-20"
owners:
  - rspdl-maintainers
target_spec: "0.5.0"
---

# 문서 머리말 구조 선언: 정보구조, 화면 레이아웃과 화면 흐름

## 상태와 목적

이 RFC는 문장형 문법을 넓히지 않는다. 문서 맨 앞에 **머리말 블록**을 두고 거기에
정보구조, 화면 레이아웃과 화면 흐름을 선언한다. 언어에 선을 하나 긋는다.

> **문장은 "무엇을 한다"** — 의미이며 검증 대상이다.
> **머리말은 "어디 있고 무엇으로 이루어져 있다"** — 구조이며 의미와 대조된다.

- 화면의 존재와 데이터 조작은 지금처럼 문장이 선언한다. 머리말은 그 화면을 stable ID로 참조만 한다.
- 머리말은 같은 사실을 다시 선언하지 않는다. 한 사실이 두 곳에서 선언되면 그것이 곧 모호함이다.
- 머리말이 참조하는 화면, 필드, 모델과 행동은 전부 선언된 stable ID여야 한다.
- 레이아웃은 **의미 단위 고정 어휘**만 쓴다. 좌표, 크기, 색, 간격과 시각 상태는 포함하지 않는다.
- 화면 흐름은 화면이 아니라 **화면 안의 요소**에서 출발한다.

이것은 `screen-structure-spec-divergence`의 직접 결과이며, 화면 경로가 데이터 가용성과 맞물리므로
`data-lifecycle-modeling-gap`을, 대조된 양쪽의 원문 위치를 진단이 함께 가리켜야 하므로
`semantic-source-provenance-loss`를 함께 연결한다.

## 실패 시나리오

장바구니 항목 입력 화면을 만든다. 명세에는 수량과 금액을 입력할 수 있다고 쓰여 있고, 와이어프레임에는
수량·금액·메모 세 칸이 그려져 있다. 메모는 어떤 선언에도 없다. 개발자는 메모를 빼야 할지, 필드를
추가해야 할지 물어보고 기다린다.

반대 방향도 같은 빈도로 일어난다. 명세가 필수로 선언한 금액이 어떤 화면에도 놓이지 않아 사용자가
채울 자리가 없다. 컴파일은 통과하고, 화면 설계도 통과하고, QA에서 드러난다.

흐름도 마찬가지다. 담기 버튼이 장바구니 상세로 간다고 그려 두었는데 그 화면의 stable ID가 바뀌었다.
그림은 여전히 그럴듯하고 아무것도 깨지지 않는다.

## 가장 작은 vertical slice

첫 slice는 한 문서의 머리말에서 정보구조, 화면 레이아웃과 화면 흐름을 선언하고, 이미 구현된
화면 조작(`RFC-0005`)과 대조한다.

- 정보구조는 분류 트리와 화면 소속만 다룬다. 분류에 정책이나 권한을 걸지 않는다.
- 레이아웃 어휘는 `머리말`, `구역`, `제목`, `폼`, `입력`, `목록`, `버튼`, `자리` 여덟 개로 닫는다.
- 흐름은 `출발 요소 → 도착 화면` 한 방향과 사람이 읽을 설명만 가진다.
- 조건에 따라 갈리는 경로는 **설명 문자열**로만 적는다. 조건식의 의미는 이 slice의 범위가 아니다.
- 머리말은 문서마다 최대 하나이고 문서 맨 앞에만 올 수 있다.

`글`, `표`, `카드`, `링크`, `필터` 어휘, 반응형 분기, 조건식 lowering, 화면 간 데이터 전달과
분기 노드(`decision point`)의 의미는 이 slice에서 지원하지 않는다. lower할 수 없는 구조는 빈 값이나
성공이 아니라 진단으로 남긴다.

## 머리말 블록

문서 맨 앞에 `---` 로 열고 `---` 로 닫는다. Markdown frontmatter와 같은 모양이며, 기획자가 이미
아는 표기를 그대로 쓴다.

```rspdl
---
모듈: 장바구니(shopping)

정보구조:
  주문(order):
    결제(payment): [장바구니 작성 화면, 장바구니 항목 입력 화면]
    조회(inquiry): [장바구니 상세 화면, 장바구니 항목 화면]

화면:
  장바구니 항목 입력 화면:
    유형: page
    레이아웃:
      - 머리말:
          - 제목: "항목 추가"
      - 구역:
          - 폼:
              - 입력: 수량
              - 입력: 금액
          - 버튼: { id: submit, 이름: "담기", 행동: 항목_담기 }
          - 버튼: { id: cancel, 이름: "취소" }

흐름:
  - 출발: 장바구니 항목 입력 화면.submit
    도착: 장바구니 상세 화면
    설명: "담기 성공"
  - 출발: 장바구니 항목 입력 화면.cancel
    도착: 장바구니 상세 화면
---

장바구니 항목(item)은 다음 필드들로 구성되어 있다.
    수량(quantity): 필수 정수
    금액(amount): 필수 정수

장바구니 항목 입력 화면(create_item)에서는 장바구니 항목을 생성할 수 있다.
장바구니 항목 입력 화면(create_item)에서는 장바구니 항목의 수량, 금액을 입력할 수 있다.
```

### 참조는 문장과 같은 방식으로 적는다

머리말이 화면·모델·필드·행동을 가리킬 때는 **문장이 그것을 가리키는 방식 그대로** 적는다. 문장이
`장바구니 항목의 수량을 입력할 수 있다`라고 쓰면 머리말도 `- 입력: 수량`이라고 쓴다. 한 언어에
이름 짓는 방법이 둘이면, 기획자는 한국어 문서를 쓰다가 머리말에서만 영어 식별자로 갈아타야 한다.

머리말에 적힌 이름은 파싱과 lowering을 거치는 동안 `SurfaceRef`로 실려 다니다가, 문장의 참조를
푸는 것과 **같은 해석기**가 Locale frontend 안에서 푼다. 분석기에는 이미 stable ID가 된 뒤에
도달한다. 따라서 없는 이름과 모호한 이름의 진단도 문장과 같다
(`ko.reference.not_found`, `ko.reference.ambiguous`). 이름이 어떤 모양인지는 Locale 지식이므로
해석이 frontend에 있는 것이 맞고, 분석기는 Locale과 무관한 stable ID만 다룬다는 경계는 그대로다.

버튼의 `id`는 예외다. 그것은 문서가 처음 만드는 이름이므로 다른 선언을 가리키는 참조가 아니다.

이름을 푸는 자리는 **Locale frontend**다. 분석기가 아니다. `CanonicalId`는 세그먼트를 ASCII
소문자로 제한하므로 한국어 이름은 애초에 분석기에 도달할 수 없고, 문장의 참조도 `lowering.rs`의
`resolve_symbols`가 stable ID로 바꾼 뒤에야 넘어간다. 이름이 어떤 모양인지는 Locale 지식이므로
이 배치가 맞다 — 분석기가 "Locale과 분리된 stable ID linking"인 이유가 이것이다.

그래서 **없는 이름·모호한 이름은 의미 진단이 아니라 `ko.reference.not_found` ·
`ko.reference.ambiguous`로 보고된다.** 머리말도 문장과 정확히 같다. 이 RFC 초안이 두었던
`화면을 찾을 수 없음` 계열 의미 규칙 넷(`RSPDL-IA-002`, `RSPDL-LAYOUT-001`, `RSPDL-FLOW-002`,
`RSPDL-FLOW-003`)은 그래서 도달할 수 없어 삭제했다. 번호는 다시 매기지 않는다 — rule_id는 안정된
식별자이고, 빈 번호가 재사용된 번호보다 낫다.

`@모듈` 문장형 metadata는 머리말의 `모듈` 키와 같은 사실이다. 둘 다 있으면 거부한다. 머리말을 쓰는
문서는 `모듈` 키를 쓰고, 쓰지 않는 문서는 지금처럼 `@모듈`을 쓴다.

### 받아들이는 YAML 부분집합

YAML 전체를 받지 않는다. 결정적 파싱이 RSPDL의 계약이고, YAML 전체에는 그 계약을 지킬 수 없는
구석이 있다.

| 받는다 | 받지 않는다 |
| --- | --- |
| block mapping, block sequence | anchor `&` 와 alias `*` |
| 평문 스칼라, 큰따옴표 문자열 | 명시적 tag `!` |
| flow sequence `[a, b]`, flow mapping `{k: v}` | 여러 문서(`---` 반복, `...`) |
| `#` 주석 | 복합 키 `?`, 블록 스칼라 `&#124;` `>` |
| 공백 들여쓰기 | 탭 들여쓰기, 작은따옴표 문자열 |

스칼라의 타입을 추론하지 않는다. YAML 1.1의 `yes`/`no`/`on`/`off` 같은 함정을 피하기 위해, 값은
**그 자리에 기대되는 타입으로만** 해석한다. 기대와 다르면 `ko.frontmatter.invalid_scalar_type`이다.

머리말은 Locale frontend가 소유한다. 키 이름이 한국어이므로 `rspdl-ko`가 읽고 Locale 중립
Unlinked IR로 lower한다. 다른 Locale frontend는 자기 언어의 키를 쓰되 같은 Unlinked IR을 만든다.

## 정보구조

분류는 stable ID를 가지며 중첩 매핑으로 계층을 만든다. 잎에는 화면 stable ID 목록을 둔다.

- 분류 ID는 화면·모델·역할·행동과 **같은 최상위 이름 공간**을 쓴다. 겹치면 기존
  `semantic.declaration.duplicate_id`로 보고된다. 이름 공간을 따로 두려던 초안은 철회했다 —
  IR에 이름 공간을 표현할 자리가 없어 둘이 똑같은 `CanonicalId`를 갖게 되고, ID로 선언을
  가리키는 소비자가 둘을 구별할 수 없다. 분류 하나를 다시 이름 짓는 비용보다 크다.
- 분류 계층은 중첩으로만 적으므로 순환이 표현될 수 없다. 순환 진단을 두지 않는다 —
  도달할 수 없는 규칙은 썩는다.
- 한 화면은 최대 하나의 분류에 속한다.
- 어떤 분류에도 없는 화면은 오류가 아니다. `미분류`로 남고 `info` 진단이 그 사실만 알린다.
- 깊이는 강제하지 않는다. 관례(depth 3)를 넘으면 `warning`으로 알린다 — 분류 깊이는 사용자
  조사로 정해지는 설계 결정이지 의미 규칙이 아니기 때문이다.

## 화면 레이아웃

`화면` 아래에 화면 stable ID별로 선언한다. `유형`은 `page`, `popup`, `tab`, `link` 중 하나이며
**선택이다.** 적지 않으면 기본값으로 채우지 않고 미지정으로 남긴다 — 선언되지 않은 의도를 추측하지
않는다는 제약이 여기에도 적용된다.

| 어휘 | 자식 | 참조 | 뜻 |
| --- | --- | --- | --- |
| `머리말` | 요소 목록 | | 화면 위쪽의 고정 영역 |
| `구역` | 요소 목록 | | 의미상 묶이는 영역 |
| `제목` | | 문자열 | 사람이 읽는 제목 |
| `폼` | `입력` 목록 | | 값을 채워 넣는 묶음 |
| `입력` | | 필드 ID | 한 필드를 채우는 자리 |
| `목록` | | `{ 모델: <모델 ID>, 필드: [<필드 ID>, ...] }` | 여러 record 를 늘어놓는 자리 |
| `버튼` | | `id`, `이름`, 선택적 `행동` ID | 누를 수 있는 자리 |
| `자리` | | 문자열 | 지도·차트처럼 선언할 수 없는 자리의 이름표 |

`자리`가 이 어휘의 안전판이다. 표현할 수 없는 것을 억지로 다른 어휘로 옮겨 적는 대신, 이름을 달고
비워 둔다. 채우는 것은 디자인의 일이다.

대조 규칙은 다음과 같다.

- `입력`의 필드는 그 화면의 `입력` 또는 `수정` 조작에 선언되어 있어야 한다.
- `목록`의 모델과 필드는 그 화면의 `조회` 조작에 선언되어 있어야 한다.
- `버튼`의 `행동`은 선언된 행동이어야 한다.
- 내부용·비표시(`FieldIntent`)로 선언된 필드는 어떤 요소에도 놓을 수 없다.
- 화면의 조작이 선언한 필수 필드가 어떤 화면에도 놓이지 않으면 알린다. 화면 하나만 보고는 알 수
  없으므로 문서 전체를 본 뒤 판정한다.

## 화면 흐름

`출발`은 `화면ID.요소ID` 형태이고 `도착`은 화면 ID다. `설명`은 사람이 읽는 문자열이다.

- 출발 요소는 누를 수 있는 어휘여야 한다. 첫 slice에서는 `버튼`뿐이다.
- 같은 `출발`·`도착` 쌍이 여러 번 나오면 `설명`이 달라야 한다. 조건이 갈리는 경로를 적는 방법이다.
- 어떤 경로로도 닿을 수 없는 화면은 `warning`으로 알린다. 진입점이 하나도 없어도 알린다.
- 경로는 방향만 가진다. 뒤로 가기, 모달 닫기 같은 암묵 경로를 추측해 넣지 않는다.

## Canonical IR

`SemanticModule`에 세 목록을 더한다. 전부 stable ID와 span을 가진다.

```
CategoryDefinition        { id, name, parent_id?, span }
ScreenCategoryAssignment  { screen_id, category_id, span }
ScreenLayoutDefinition    { screen_id, kind?, elements: Vec<LayoutElement>, span }
ScreenPathDefinition      { source_screen_id, source_element_id, target_screen_id, label?, span }

LayoutElement  (enum)
  Header      { children, span }
  Section     { children, span }
  Heading     { text, span }
  Form        { inputs, span }
  Input       { field_id, span }
  List        { model_id, field_ids, span }
  Button      { id, name, action_id?, span }
  Placeholder { name, span }
```

경로에는 stable ID가 없다. 아무도 이름 붙이지 않는 것에 ID를 지어 주면 없는 사실을 만드는
것이고, 소비자는 양 끝점으로 가리키면 된다.

`LayoutElement`는 Option 투성이 평평한 구조체가 아니라 **enum**이다. 어휘마다 가지는 것이 다르므로
불가능한 조합을 타입이 막게 한다.

직렬화는 이 저장소가 이미 쓰는 규약을 따른다 — 데이터를 가진 enum은
`#[serde(tag = "kind", rename_all = "snake_case")]` 내부 태깅이다(`RelationalConstraintKind`,
`DerivationExpression`과 같다). 소비자가 `kind`로 갈라 읽고 있으므로 이 규약을 벗어나면 조용히
깨진다.

`LayoutElement`는 재귀 구조이며 선언 순서를 보존한다. 순서가 곧 화면에서 읽히는 순서이고, 그것은
좌표가 아니라 의미다. 직렬화는 기존 규칙대로 결정적이어야 한다.

## 진단

진단은 `rule_id`와 `message_key`를 **둘 다** 가진다(`Diagnostic { rule_id, severity, message_key, arguments, span }`).
`rule_id`는 안정된 식별자이고 `message_key`는 Locale별 문장을 고르는 열쇠다. 둘을 하나로 쓰지 않는다.

머리말을 읽는 단계의 진단은 Locale frontend 소유이므로 `RSPDL-KO-FM-*` 계열이다.

| rule_id | message_key | 심각도 | 조건 |
| --- | --- | --- | --- |
| `RSPDL-KO-FM-001` | `ko.frontmatter.unterminated_block` | error | 여는 `---` 뒤에 닫는 `---`가 없음 |
| `RSPDL-KO-FM-002` | `ko.frontmatter.tab_indentation` | error | 탭 들여쓰기 |
| `RSPDL-KO-FM-003` | `ko.frontmatter.inconsistent_indent` | error | 들여쓰기 단이 맞지 않음 |
| `RSPDL-KO-FM-004` | `ko.frontmatter.unsupported_yaml_feature` | error | anchor·alias·tag·복수 문서·블록 스칼라 |
| `RSPDL-KO-FM-005` | `ko.frontmatter.unknown_key` | error | 스키마에 없는 키 |
| `RSPDL-KO-FM-006` | `ko.frontmatter.duplicate_key` | error | 같은 매핑에 같은 키 |
| `RSPDL-KO-FM-007` | `ko.frontmatter.invalid_scalar_type` | error | 그 자리에 기대되는 타입과 다른 스칼라 |
| `RSPDL-KO-FM-008` | `ko.frontmatter.invalid_structure` | error | 모양 불일치 또는 필수 키 누락. `expected` 인자를 싣는다 |
| `RSPDL-KO-FM-009` | `ko.frontmatter.unknown_layout_element` | error | 고정 어휘 밖의 요소 |
| `RSPDL-KO-FM-010` | `ko.frontmatter.module_declared_twice` | error | `@모듈`과 `모듈` 키가 함께 있음 |
| `RSPDL-KO-FM-011` | `ko.frontmatter.key_indented_too_deep` | error | 인라인 값을 가진 키 아래로 한 단 더 들어가 앞 줄의 값에 딸려 버린 키. `key` 인자를 싣고 그 키를 짚는다 |

대조 단계의 진단은 Locale 중립 분석기 소유이므로 기존 의미 계열을 따른다.

| rule_id | message_key | 심각도 | 조건 |
| --- | --- | --- | --- |
| `RSPDL-IA-003` | `semantic.information_architecture.screen_multiple_categories` | error | 한 화면이 두 분류에 속함 |
| `RSPDL-IA-W001` | `semantic.information_architecture.depth_exceeds_convention` | warning | 분류 깊이가 3을 넘음 |
| `RSPDL-IA-I001` | `semantic.information_architecture.screen_uncategorized` | info | 분류에 없는 화면 |
| `RSPDL-LAYOUT-002` | `semantic.layout.duplicate_screen` | error | 한 화면의 레이아웃이 둘 |
| `RSPDL-LAYOUT-003` | `semantic.layout.duplicate_element_id` | error | 한 화면 안 요소 ID 중복 |
| `RSPDL-LAYOUT-004` | `semantic.layout.field_not_in_screen_operation` | error | 화면이 다루지 않는 필드를 놓음 |
| `RSPDL-LAYOUT-005` | `semantic.layout.model_not_read_by_screen` | error | 조회 선언 없는 모델을 목록에 놓음 |
| `RSPDL-LAYOUT-006` | `semantic.layout.action_not_found` | error | 없는 행동을 버튼에 검 |
| `RSPDL-LAYOUT-007` | `semantic.layout.hidden_field_exposed` | error | 내부용·비표시 필드를 노출 |
| `RSPDL-LAYOUT-W001` | `semantic.layout.required_input_without_slot` | warning | 필수 필드를 채울 자리가 어디에도 없음 |
| `RSPDL-FLOW-001` | `semantic.screen_flow.source_element_not_found` | error | 없는 요소에서 출발 |
| `RSPDL-FLOW-004` | `semantic.screen_flow.duplicate_path` | error | 같은 쌍·같은 설명의 경로 중복 |
| `RSPDL-FLOW-W001` | `semantic.screen_flow.unreachable_screen` | warning | 어떤 경로로도 닿을 수 없음 |
| `RSPDL-FLOW-W002` | `semantic.screen_flow.no_entry_point` | warning | 진입점이 하나도 없음 |

`semantic.information_architecture.screen_uncategorized`를 위해 `Severity`에 `Info`를 더한다.
아무 말도 하지 않으면 분류 밖 화면이 감춰지고, `warning`으로 부르면 저자가 하지 않은 실수를 했다고
주장하게 된다. 소비자에게는 severity 문자열이 하나 늘어나는 순수 추가이며 `wire_schema_version`은
`1`로 둔다.

대조에서 나온 진단은 **양쪽 span을 함께** 준다. 요소 쪽만 가리키면 사람은 무엇과 어긋났는지 다시
찾아야 한다.

없는 것을 없다고 말하기 전에, 그 이름이 **바로 한 단 아래**에 적혀 있지는 않은지 본다. 들여쓰기가
한 칸 더 들어간 줄은 앞선 형제의 값에 딸려 들어가므로 매핑에서는 사라지지만 쓴 사람 눈에는 분명히
적혀 있다. 그때 "없다"는 거짓이고, 거짓인 진단은 막연한 진단보다 나쁘다 — 읽는 사람이 믿고 엉뚱한
곳을 뒤지기 때문이다. 대신 그 줄을 짚어 너무 깊이 들어갔다고 말한다. 한 단만 내려다보며, 이름이
확실히 이 항목 안의 것이 아니면 원래대로 "없다"를 낸다.

## 포매터 왕복

`rspdl fmt`는 문서를 다시 써 내므로 머리말을 **읽고 그대로 내보낼 수 있어야 한다.** 내보내기가 없으면
포매터가 사용자의 정보구조·레이아웃·흐름을 통째로 지운다. 이것은 선택 기능이 아니라 데이터 손실
방지이며, 머리말을 쓰는 문서가 공개되기 전에 구현되어야 한다.

- 머리말이 있는 문서를 포맷하면 머리말이 **의미적으로 같게** 남는다.
- 포맷은 멱등이다. 두 번 돌린 결과가 한 번 돌린 결과와 같다.
- 내보낼 수 없는 것을 만나면 **한 번 실패한다.** 반쪽짜리 블록을 쓰고 나서 그것을 다시 나무라지
  않는다. 조용히 지우는 것은 어떤 경우에도 하지 않는다.
- 따옴표는 필요한 자리에만 붙인다. 판단은 위치별 특례가 아니라 기계적인 하나의 규칙이어야 한다 —
  규칙이 사람 머릿속에만 있으면 다음 사람이 어긴다.

## 구현 상태

2026-09-20 기준으로 이 문서의 내용은 구현되었다. 머리말 리더(`ko.frontmatter.*`), Unlinked IR과
lowering, 의미 IR과 대조 분석(`RSPDL-IA-*` · `RSPDL-LAYOUT-*` · `RSPDL-FLOW-*`), 포매터 왕복,
그리고 `conformance/ko-KR/frontmatter-structure` 의 정상·실패·경계·오탐 방지 사례가 모두 들어 있다.

아직 구현하지 않은 것은 다음과 같다. 목표 범위이지만 이 slice에 없다.

- `글`, `표`, `카드`, `링크`, `필터` 레이아웃 어휘
- 경로 조건의 의미. 지금 `설명`은 사람이 읽는 불투명한 문자열이며 분석 대상이 아니다
- 분기 노드(`decision point`)와 동시 결과(`concurrent set`)의 의미
- 화면 간 데이터 전달
- 반응형 분기와 뷰포트별 레이아웃 차이

분류, 화면 소속, 레이아웃과 경로는 **선언 순서 그대로** IR에 실린다. 분류는 중첩의 전위 순회
순서다. 정보구조에서 형제의 순서는 곧 메뉴 순서이고 그것은 기획자가 그 순서로 적어서 내린 설계
결정이므로, 정렬해 버리면 선언된 의도를 버리는 것이 된다. 정의가 아니라 참조 목록
(한 화면의 `field_ids` 같은)은 authored order가 없으므로 계속 정렬한다.

## 범위 밖

- 좌표, 크기, 색, 간격, 폰트와 시각 상태. 보드의 노드 위치도 여기 속하며 application이 소유한다.
- 특정 UI 프레임워크의 component 이름, 렌더링 결과와 디자인 토큰.
- 조건식의 의미. 경로의 조건은 문자열 설명이며 분석 대상이 아니다.
- 화면 간 데이터 전달, 뒤로 가기·모달 스택 같은 암묵 경로.
- 반응형 분기와 뷰포트별 레이아웃 차이.
- 다국어 문구. 머리말의 문자열은 표시 문구가 아니라 구조를 읽기 위한 이름이다.

## 기존 결정에 대한 영향

이 RFC는 두 문서의 당시 문장과 정면으로 어긋났고, 구현과 함께 그 둘을 고쳤다.

- [`ADR-0002`](../adr/0002-core-application-boundary.md)는 *"화면 배치, widget, navigation과 시각
  상태는 application projection이다"* 와 *"특정 UI component 구조를 core IR에 포함하지 않는다"* 를
  두고 있다. 같은 ADR이 예외 경로도 두었다 — *"projection이 언어 의미 자체가 될 때만 별도 RFC와
  ADR로 core 편입을 재검토한다."* 이 RFC가 그 재검토다. 경계를 지우는 것이 아니라 **옮긴다**:
  의미 단위 구조는 core로, 시각 표현과 배치는 application에 그대로 남긴다.
- [`PRD`](../prd.md)는 *"특정 제품 UI는 초기 언어 범위에 포함하지 않는다"* 와 *"IA, UI projection은
  application 책임"* 을 제약으로 둔다. 전자는 유지된다 — 특정 제품의 UI가 아니라 의미 단위
  어휘이기 때문이다. 후자는 IA와 구조에 한해 수정이 필요하다.
- `유저 플로우`는 PRD의 미구현 목록에 있다. 이 RFC가 그 항목의 첫 slice다.

## References

- [Screen Structure and Flow Spec Divergence](../problems/0006-screen-structure-spec-divergence.md)
- [Field Provenance, Screen Usage, Action Data Mutations, and Sum Derivation Grammar](0005-field-provenance-and-sum-derivation.md)
- [Korean Domain Frontend Language Specification](0004-natural-korean-domain-grammar.md)
- [Controlled Korean Surface Grammar](0001-controlled-korean-surface-grammar.md)
- [Core와 Application Projection 경계](../adr/0002-core-application-boundary.md)
- [Frontend Semantic Analysis Contract](../specs/frontend-semantic-analysis-contract.md)
- [RSPDL Product Requirements](../prd.md)
