# rspdl-grammar-compiler

RSPDL Locale frontend를 위한 작은 실행 가능 EBNF compiler다. 범용 parser generator가 아니라 규범 production과 실행 parser의 중복을 없애는 workspace 내부 기반이다.

## 지원 문법

```ebnf
pub policy_statement =
    role: @marked_ref("은", "는")
    action: @marked_ref("할")
    "수"
    effect: ("있다" | "없다")
    "."
    ;
```

- `pub name = expression ;`: 외부에서 parse할 수 있는 entry production
- `name = expression ;`: 내부 production
- `"literal"`: Locale adapter가 token과 비교하는 terminal
- `rule_name`: 다른 production 참조
- `left right`: sequence
- `left | right`: alternative
- `[expression]` 또는 `expression?`: optional
- `{expression}` 또는 `expression*`: zero-or-more
- `expression+`: one-or-more
- `capture: expression`: 이름 있는 capture와 source range
- `@matcher("argument")`: Locale adapter가 구현하는 contextual matcher

Grammar compiler는 duplicate/undefined rule, 등록되지 않은 matcher, nullable repetition과 left recursion을 거부한다. Runtime은 선언 순서로 alternative를 고르지 않으며 완전 parse가 둘 이상이면 ambiguity를 반환한다.

## Locale production 이관 절차

1. 현재 handwritten parser의 정상, 실패, 경계와 오탐 방지 corpus를 먼저 고정한다.
2. 같은 production을 Locale crate의 `.ebnf` 파일에 선언한다.
3. build script에서 허용할 contextual matcher를 명시하고 Rust parser를 생성한다.
4. Locale token adapter가 literal과 contextual matcher의 value/source range를 반환하게 한다.
5. generated capture 또는 Locale AST와 기존 parser 결과를 differential test로 비교한다.
6. 동등성 gate가 안정된 production만 production parse path로 전환한다.

현재 `rspdl-ko`의 policy, constraint/literal, declaration/block item, screen/provenance, relation/meta-rule 문형이 shadow production으로 이관되어 있다. 이 단계에서는 generated parser가 기존 handwritten parser와 differential test로 비교되며 production parse 결과를 바꾸지 않는다. Lexer 생성, formatter/lowering 생성, general left-recursive expression grammar와 자동 오류 복구는 아직 지원하지 않는다.
