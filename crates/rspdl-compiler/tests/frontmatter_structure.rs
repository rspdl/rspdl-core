//! End-to-end checks for the document frontmatter: information architecture,
//! screen layout and screen flow, compared against the sentences.
//!
//! These run through `compile_ko` rather than hand-built Unlinked literals
//! because the point of this feature is the seam between two surfaces — the
//! frontmatter and the sentences — and a literal cannot exercise a seam.

use rspdl_compiler::compile_ko;
use rspdl_domain::{Diagnostic, Severity};

const MODELS_AND_SCREENS: &str = r#"
장바구니 항목(item)은 다음 필드들로 구성되어 있다.
    수량(quantity): 필수 정수
    금액(amount): 필수 정수

장바구니 항목 입력 화면(create_item)에서는 장바구니 항목을 생성할 수 있다.
장바구니 항목 입력 화면(create_item)에서는 장바구니 항목의 수량, 금액을 입력할 수 있다.
장바구니 상세 화면(cart_detail)에서는 장바구니 항목의 수량을 조회할 수 있다.
"#;

fn compile(frontmatter: &str) -> Vec<Diagnostic> {
    let source = format!("---\n모듈: 장바구니(shopping)\n{frontmatter}---\n{MODELS_AND_SCREENS}");
    compile_ko(&source).diagnostics
}

fn keys(diagnostics: &[Diagnostic]) -> Vec<&str> {
    diagnostics
        .iter()
        .map(|diagnostic| diagnostic.message_key.as_str())
        .collect()
}

fn errors(diagnostics: &[Diagnostic]) -> Vec<&str> {
    diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .map(|diagnostic| diagnostic.message_key.as_str())
        .collect()
}

const WHOLE_SCREEN: &str = r#"화면:
  장바구니 항목 입력 화면:
    유형: page
    레이아웃:
      - 폼:
          - 입력: 수량
          - 입력: 금액
      - 버튼: { id: submit, 이름: "담기" }
"#;

#[test]
fn a_layout_matching_the_sentences_is_accepted() {
    let diagnostics = compile(WHOLE_SCREEN);
    assert!(
        errors(&diagnostics).is_empty(),
        "unexpected errors: {:?}",
        keys(&diagnostics)
    );
}

#[test]
fn a_field_the_screen_never_declared_is_rejected() {
    // The sentences say this screen inputs 수량 and 금액. 금액 is removed from the
    // layout's screen by pointing the layout at a screen that only reads 수량.
    let diagnostics = compile(
        r#"화면:
  장바구니 상세 화면:
    레이아웃:
      - 폼:
          - 입력: 금액
"#,
    );
    assert_eq!(
        errors(&diagnostics),
        vec!["semantic.layout.field_not_in_screen_operation"]
    );
}

#[test]
fn a_mismatch_points_at_both_the_layout_and_the_sentence() {
    let diagnostics = compile(
        r#"화면:
  장바구니 상세 화면:
    레이아웃:
      - 폼:
          - 입력: 금액
"#,
    );
    let mismatch = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.message_key == "semantic.layout.field_not_in_screen_operation"
        })
        .expect("the mismatch is reported");
    // The span is the layout side; the sentence side rides along as an argument,
    // so the reader is not left hunting for the other half.
    assert!(mismatch.arguments.contains_key("declared_at"));
    assert_eq!(
        mismatch.arguments.get("screen_id").map(String::as_str),
        Some("shopping.cart_detail")
    );
}

#[test]
fn a_screen_named_by_a_typo_is_reported_once() {
    // One root cause, one diagnostic: the unresolved name is reported by the
    // frontend and nothing downstream invents a second complaint about it.
    let diagnostics = compile(
        r#"흐름:
  - 출발: 장바구니 항목 입력 화면.submit
    도착: 없는 화면
"#,
    );
    assert_eq!(errors(&diagnostics), vec!["ko.reference.not_found"]);
}

#[test]
fn a_path_leaving_an_element_that_does_not_exist_is_rejected() {
    let diagnostics = compile(&format!(
        "{WHOLE_SCREEN}흐름:\n  - 출발: 장바구니 항목 입력 화면.없는버튼\n    도착: 장바구니 상세 화면\n"
    ));
    assert_eq!(
        errors(&diagnostics),
        vec!["semantic.screen_flow.source_element_not_found"]
    );
}

#[test]
fn a_duplicate_element_id_in_one_screen_is_rejected() {
    let diagnostics = compile(
        r#"화면:
  장바구니 항목 입력 화면:
    레이아웃:
      - 버튼: { id: submit, 이름: "담기" }
      - 버튼: { id: submit, 이름: "또 담기" }
"#,
    );
    assert_eq!(
        errors(&diagnostics),
        vec!["semantic.layout.duplicate_element_id"]
    );
}

#[test]
fn a_screen_in_two_categories_is_rejected() {
    let diagnostics = compile(
        r#"정보구조:
  주문(order): [장바구니 항목 입력 화면]
  조회(inquiry): [장바구니 항목 입력 화면]
"#,
    );
    assert_eq!(
        errors(&diagnostics),
        vec!["semantic.information_architecture.screen_multiple_categories"]
    );
}

#[test]
fn a_screen_outside_the_architecture_is_information_not_a_fault() {
    let diagnostics = compile(
        r#"정보구조:
  주문(order): [장바구니 항목 입력 화면]
"#,
    );
    assert!(errors(&diagnostics).is_empty());
    let uncategorized = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.message_key == "semantic.information_architecture.screen_uncategorized"
        })
        .expect("the uncategorized screen is mentioned");
    assert_eq!(uncategorized.severity, Severity::Info);
    assert_eq!(
        uncategorized.arguments.get("screen_id").map(String::as_str),
        Some("shopping.cart_detail")
    );
}

#[test]
fn category_depth_past_the_convention_warns_and_is_allowed() {
    let diagnostics = compile(
        r#"정보구조:
  일(a):
    이(b):
      삼(c):
        사(d): [장바구니 항목 입력 화면]
"#,
    );
    assert!(errors(&diagnostics).is_empty());
    let deep = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.message_key == "semantic.information_architecture.depth_exceeds_convention"
        })
        .expect("the depth is mentioned");
    assert_eq!(deep.severity, Severity::Warning);
}

#[test]
fn document_wide_warnings_are_withheld_while_an_error_stands() {
    // The shape being judged is not the shape the author wrote, so a judgement
    // drawn from it would be invented.
    let diagnostics = compile(
        r#"화면:
  장바구니 상세 화면:
    레이아웃:
      - 폼:
          - 입력: 금액
"#,
    );
    assert!(!keys(&diagnostics).contains(&"semantic.layout.required_input_without_slot"));
    assert!(!keys(&diagnostics).contains(&"semantic.screen_flow.unreachable_screen"));
    assert!(
        !keys(&diagnostics).contains(&"semantic.information_architecture.screen_uncategorized")
    );
}

#[test]
fn an_unstated_screen_kind_stays_unstated() {
    let module = compile_ko(&format!(
        "---\n모듈: 장바구니(shopping)\n화면:\n  장바구니 항목 입력 화면:\n    레이아웃:\n      - 버튼: {{ id: submit, 이름: \"담기\" }}\n---\n{MODELS_AND_SCREENS}"
    ))
    .module
    .expect("the module compiles");
    let layout = module
        .screen_layouts
        .first()
        .expect("the layout is recorded");
    assert!(layout.kind.is_none(), "an unstated kind must not default");
}
