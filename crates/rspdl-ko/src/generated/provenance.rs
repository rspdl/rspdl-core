//! Shadow parsers for sentence-shaped screen and data-provenance declarations.
//!
//! The public parser remains the behavior oracle.  These wrappers deliberately
//! expose captures instead of changing parser dispatch during the migration.

use rspdl_grammar_compiler::{
    Capture, Grammar, InputAdapter, ParseError, ParseMatch, TerminalMatch,
};

use crate::scanner::{Token, TokenKind};

use super::adapter::{match_literal, match_marked_ref};
use super::required_capture;

include!(concat!(env!("OUT_DIR"), "/provenance_grammar.rs"));

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GeneratedScreen {
    pub screen_name: Capture,
    pub screen_id: Capture,
    pub model: Capture,
    pub fields: Vec<Capture>,
    pub operation: Capture,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GeneratedActionDataMutation {
    pub action: Capture,
    pub model: Capture,
    pub operation: Capture,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GeneratedActionInput {
    pub action: Capture,
    pub existing: Option<Capture>,
    pub input_type: Capture,
    pub input_name: Capture,
    pub input_id: Capture,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GeneratedEventInput {
    pub event: Capture,
    pub input_type: Capture,
    pub input_name: Capture,
    pub input_id: Capture,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GeneratedSumDerivation {
    pub target_model: Capture,
    pub target_field: Capture,
    pub source_model: Capture,
    pub source_field: Capture,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GeneratedRecalculation {
    pub source_model: Capture,
    pub source_field: Capture,
    pub target_model: Capture,
    pub target_field: Capture,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GeneratedFieldIntent {
    pub model: Capture,
    pub field: Capture,
    pub intent: Capture,
}

pub(crate) fn parse_screen(tokens: &[Token]) -> Result<GeneratedScreen, ParseError> {
    let parsed = parse("screen_statement", tokens)?;
    Ok(GeneratedScreen {
        screen_name: required_capture(&parsed, "screen_name"),
        screen_id: required_capture(&parsed, "screen_id"),
        model: required_capture(&parsed, "model"),
        fields: captures(&parsed, "field"),
        operation: required_capture(&parsed, "operation"),
    })
}

pub(crate) fn parse_action_data_mutation(
    tokens: &[Token],
) -> Result<GeneratedActionDataMutation, ParseError> {
    let parsed = parse("action_data_mutation_statement", tokens)?;
    Ok(GeneratedActionDataMutation {
        action: required_capture(&parsed, "action"),
        model: required_capture(&parsed, "model"),
        operation: required_capture(&parsed, "operation"),
    })
}

pub(crate) fn parse_action_input(tokens: &[Token]) -> Result<GeneratedActionInput, ParseError> {
    let parsed = parse("action_input_statement", tokens)?;
    Ok(GeneratedActionInput {
        action: required_capture(&parsed, "action"),
        existing: tokens.iter().find_map(|token| match &token.kind {
            TokenKind::Word(value) if value == "기존" => Some(Capture {
                value: value.clone(),
                start: token.span.start,
                end: token.span.end,
            }),
            _ => None,
        }),
        input_type: required_capture(&parsed, "input_type"),
        input_name: required_capture(&parsed, "input_name"),
        input_id: required_capture(&parsed, "input_id"),
    })
}

#[allow(dead_code)]
pub(crate) fn parse_event_input(tokens: &[Token]) -> Result<GeneratedEventInput, ParseError> {
    let parsed = parse("event_input_statement", tokens)?;
    Ok(GeneratedEventInput {
        event: required_capture(&parsed, "event"),
        input_type: required_capture(&parsed, "input_type"),
        input_name: required_capture(&parsed, "input_name"),
        input_id: required_capture(&parsed, "input_id"),
    })
}

pub(crate) fn parse_sum_derivation(tokens: &[Token]) -> Result<GeneratedSumDerivation, ParseError> {
    let parsed = parse("sum_derivation_statement", tokens)?;
    Ok(GeneratedSumDerivation {
        target_model: required_capture(&parsed, "target_model"),
        target_field: required_capture(&parsed, "target_field"),
        source_model: required_capture(&parsed, "source_model"),
        source_field: required_capture(&parsed, "source_field"),
    })
}

pub(crate) fn parse_recalculation(tokens: &[Token]) -> Result<GeneratedRecalculation, ParseError> {
    let parsed = parse("recalculation_statement", tokens)?;
    Ok(GeneratedRecalculation {
        source_model: required_capture(&parsed, "source_model"),
        source_field: required_capture(&parsed, "source_field"),
        target_model: required_capture(&parsed, "target_model"),
        target_field: required_capture(&parsed, "target_field"),
    })
}

pub(crate) fn parse_field_intent(tokens: &[Token]) -> Result<GeneratedFieldIntent, ParseError> {
    let parsed = parse("field_intent_statement", tokens)?;
    Ok(GeneratedFieldIntent {
        model: required_capture(&parsed, "model"),
        field: required_capture(&parsed, "field"),
        intent: required_capture(&parsed, "intent"),
    })
}

fn parse(entry: &str, tokens: &[Token]) -> Result<ParseMatch, ParseError> {
    let grammar: Grammar = generated_provenance_grammar();
    grammar.parse(entry, tokens, &ProvenanceTokenAdapter)
}

fn captures(parsed: &ParseMatch, name: &str) -> Vec<Capture> {
    parsed.captures.get(name).cloned().unwrap_or_default()
}

struct ProvenanceTokenAdapter;

impl InputAdapter<Token> for ProvenanceTokenAdapter {
    fn match_literal(
        &self,
        tokens: &[Token],
        position: usize,
        literal: &str,
    ) -> Option<TerminalMatch> {
        match_literal(tokens, position, literal)
    }

    fn match_contextual(
        &self,
        tokens: &[Token],
        position: usize,
        matcher: &str,
        arguments: &[String],
    ) -> Vec<TerminalMatch> {
        match matcher {
            "marked_ref" => match_marked_ref(tokens, position, arguments),
            "action_input_type" => action_input_type(tokens, position, arguments),
            "screen_model_ref" => screen_model_reference(tokens, position, arguments),
            "surface_name" => surface_name_prefixes(tokens, position),
            "canonical_id" => canonical_id(tokens, position),
            "comma_ref" => comma_reference(tokens, position),
            _ => Vec::new(),
        }
    }
}

fn action_input_type(tokens: &[Token], position: usize, markers: &[String]) -> Vec<TerminalMatch> {
    if matches!(
        tokens.get(position).map(|token| &token.kind),
        Some(TokenKind::Word(value)) if value == "기존"
    ) {
        Vec::new()
    } else {
        match_marked_ref(tokens, position, markers)
    }
}

fn screen_model_reference(
    tokens: &[Token],
    position: usize,
    markers: &[String],
) -> Vec<TerminalMatch> {
    let first_marker = tokens
        .iter()
        .enumerate()
        .skip(position)
        .find_map(|(index, token)| match &token.kind {
            TokenKind::Word(value)
                if matches!(value.as_str(), "의" | "을" | "를")
                    && index > position
                    && matches!(tokens[index - 1].kind, TokenKind::QuotedIdentifier(_)) =>
            {
                Some(value.as_str())
            }
            TokenKind::Word(value) => ["의", "을", "를"].into_iter().find(|marker| {
                value
                    .strip_suffix(marker)
                    .is_some_and(|base| !base.is_empty())
            }),
            _ => None,
        });
    if first_marker != Some("을") && first_marker != Some("를") {
        Vec::new()
    } else {
        match_marked_ref(tokens, position, markers)
    }
}

fn surface_name_prefixes(tokens: &[Token], position: usize) -> Vec<TerminalMatch> {
    let Some(first) = tokens.get(position) else {
        return Vec::new();
    };
    let mut parts = Vec::new();
    let mut matches = Vec::new();
    for (index, token) in tokens.iter().enumerate().skip(position) {
        let value = match &token.kind {
            TokenKind::Word(value) | TokenKind::QuotedIdentifier(value) => value,
            _ => break,
        };
        parts.push(value.clone());
        matches.push(TerminalMatch::new(
            index + 1,
            parts.join(" "),
            first.span.start,
            token.span.end,
        ));
    }
    matches
}

fn canonical_id(tokens: &[Token], position: usize) -> Vec<TerminalMatch> {
    match tokens.get(position) {
        Some(Token {
            kind: TokenKind::CanonicalId(value),
            span,
        }) if !value.is_empty() => vec![TerminalMatch::new(
            position + 1,
            value,
            span.start,
            span.end,
        )],
        _ => Vec::new(),
    }
}

fn comma_reference(tokens: &[Token], position: usize) -> Vec<TerminalMatch> {
    let Some(first) = tokens.get(position) else {
        return Vec::new();
    };
    let mut parts = Vec::new();
    for (index, token) in tokens.iter().enumerate().skip(position) {
        match &token.kind {
            TokenKind::Word(value) | TokenKind::QuotedIdentifier(value) => {
                parts.push(value.clone());
            }
            TokenKind::Comma if !parts.is_empty() => {
                return vec![TerminalMatch::new(
                    index + 1,
                    parts.join(" "),
                    first.span.start,
                    tokens[index - 1].span.end,
                )];
            }
            _ => return Vec::new(),
        }
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use crate::ast::{
        ActionInputKindAst, DataMutationKindAst, DeclarationAst, FieldIntentKindAst,
        ScreenOperationKindAst, TypeReferenceAst,
    };
    use crate::scanner::TokenKind;
    use crate::{Diagnostic, parse as parse_document, scan};

    use super::*;

    type RejectParser = fn(&[Token]) -> Result<(), ParseError>;

    fn sentence_tokens(sentence: &str) -> Vec<Token> {
        let scanned = scan(sentence);
        assert!(
            scanned.diagnostics.is_empty(),
            "{sentence}: {:?}",
            scanned.diagnostics
        );
        scanned
            .tokens
            .into_iter()
            .filter(|token| {
                !matches!(
                    token.kind,
                    TokenKind::Newline | TokenKind::Indent | TokenKind::Dedent
                )
            })
            .collect()
    }

    fn oracle(sentence: &str) -> Result<DeclarationAst, Vec<Diagnostic>> {
        let source = format!("@모듈 검증(check)\n{sentence}\n");
        let parsed = parse_document(&source);
        if parsed.diagnostics.iter().any(Diagnostic::is_error) {
            return Err(parsed.diagnostics);
        }
        parsed
            .document
            .expect("valid source has a document")
            .declarations
            .into_iter()
            .next()
            .ok_or(parsed.diagnostics)
    }

    fn values(captures: &[Capture]) -> Vec<String> {
        captures
            .iter()
            .map(|capture| capture.value.clone())
            .collect()
    }

    #[test]
    fn generated_screens_match_handwritten_operations_and_fields() {
        let cases = [
            (
                "항목 작성 화면(create_item)에서는 항목을 생성할 수 있다.",
                ScreenOperationKindAst::Create,
                Vec::<&str>::new(),
            ),
            (
                "항목 상세 화면(detail)에서는 항목을 조회할 수 있다.",
                ScreenOperationKindAst::Read,
                Vec::<&str>::new(),
            ),
            (
                "항목 입력 화면(input)에서는 항목의 수량, 금액을 입력할 수 있다.",
                ScreenOperationKindAst::Input,
                vec!["수량", "금액"],
            ),
            (
                "항목 수정 화면(update)에서는 항목의 금액을 수정할 수 있다.",
                ScreenOperationKindAst::Update,
                vec!["금액"],
            ),
            (
                "항목 삭제 화면(delete)에서는 항목을 삭제할 수 있다.",
                ScreenOperationKindAst::Delete,
                Vec::<&str>::new(),
            ),
            (
                "배송 입력 화면(input)에서는 배송의 `배송 주소`, `수령인 이름`을 입력할 수 있다.",
                ScreenOperationKindAst::Input,
                vec!["배송 주소", "수령인 이름"],
            ),
        ];
        for (sentence, operation, expected_fields) in cases {
            let DeclarationAst::Screen(handwritten) =
                oracle(sentence).unwrap_or_else(|error| panic!("{sentence}: {error:?}"))
            else {
                panic!("oracle did not return a screen");
            };
            let generated = parse_screen(&sentence_tokens(sentence))
                .unwrap_or_else(|error| panic!("{sentence}: {error:?}"));
            assert_eq!(
                generated.screen_name.value, handwritten.declaration.name,
                "{sentence}"
            );
            assert_eq!(
                generated.screen_id.value, handwritten.declaration.id,
                "{sentence}"
            );
            assert_eq!(generated.model.value, handwritten.model, "{sentence}");
            assert_eq!(
                generated.operation.value,
                operation_word(operation),
                "{sentence}"
            );
            assert_eq!(values(&generated.fields), handwritten.fields, "{sentence}");
            assert_eq!(handwritten.operation, operation, "{sentence}");
            assert_eq!(values(&generated.fields), expected_fields, "{sentence}");
        }
    }

    #[test]
    fn generated_provenance_matches_handwritten_ast_captures() {
        for (sentence, expected) in [
            (
                "주문 등록이 실행되면 주문을 생성한다.",
                DataMutationKindAst::Create,
            ),
            (
                "주문 변경이 실행되면 주문을 수정한다.",
                DataMutationKindAst::Update,
            ),
            (
                "주문 취소가 실행되면 주문을 삭제한다.",
                DataMutationKindAst::Delete,
            ),
        ] {
            let DeclarationAst::ActionDataMutation(handwritten) = oracle(sentence).unwrap() else {
                panic!("oracle did not return an action data mutation")
            };
            let generated = parse_action_data_mutation(&sentence_tokens(sentence)).unwrap();
            assert_eq!(generated.action.value, handwritten.action, "{sentence}");
            assert_eq!(generated.model.value, handwritten.model, "{sentence}");
            assert_eq!(
                generated.operation.value,
                data_mutation_word(expected),
                "{sentence}"
            );
            assert_eq!(handwritten.mutation, expected, "{sentence}");
        }

        let sum = "장바구니의 결제 예정 금액은 장바구니 항목의 금액의 합계로 계산한다.";
        let DeclarationAst::SumDerivation(handwritten) = oracle(sum).unwrap() else {
            panic!("oracle did not return derivation")
        };
        let generated = parse_sum_derivation(&sentence_tokens(sum)).unwrap();
        assert_eq!(generated.target_model.value, handwritten.target_model);
        assert_eq!(generated.target_field.value, handwritten.target_field);
        assert_eq!(generated.source_model.value, handwritten.source_model);
        assert_eq!(generated.source_field.value, handwritten.source_field);

        let recalculation =
            "장바구니 항목의 금액이 바뀔 때 장바구니의 결제 예정 금액을 다시 계산한다.";
        let DeclarationAst::Recalculation(handwritten) = oracle(recalculation).unwrap() else {
            panic!("oracle did not return recalculation")
        };
        let generated = parse_recalculation(&sentence_tokens(recalculation)).unwrap();
        assert_eq!(generated.source_model.value, handwritten.source_model);
        assert_eq!(generated.source_field.value, handwritten.source_field);
        assert_eq!(generated.target_model.value, handwritten.target_model);
        assert_eq!(generated.target_field.value, handwritten.target_field);

        for (sentence, expected) in [
            (
                "감사 기록의 내부 메모는 내부 관리에만 사용한다.",
                FieldIntentKindAst::Internal,
            ),
            (
                "감사 기록의 위험 점수는 사용자 화면에서 조회하지 않는다.",
                FieldIntentKindAst::Hidden,
            ),
        ] {
            let DeclarationAst::FieldIntent(handwritten) = oracle(sentence).unwrap() else {
                panic!("oracle did not return intent")
            };
            let generated = parse_field_intent(&sentence_tokens(sentence)).unwrap();
            assert_eq!(generated.model.value, handwritten.model, "{sentence}");
            assert_eq!(generated.field.value, handwritten.field, "{sentence}");
            assert_eq!(generated.intent.value, intent_words(expected), "{sentence}");
            assert_eq!(handwritten.intent, expected, "{sentence}");
        }
    }

    #[test]
    fn generated_action_inputs_match_handwritten_ast_captures() {
        for sentence in [
            "주문 취소는 주문 상태를 상태(status)로 입력받는다.",
            "주문 취소는 문자열을 취소 사유(reason)로 입력받는다.",
            "주문 취소는 기존 주문을 대상 주문(target_order)으로 입력받는다.",
        ] {
            let DeclarationAst::ActionInput(handwritten) = oracle(sentence).unwrap() else {
                panic!("oracle did not return an action input")
            };
            let generated = parse_action_input(&sentence_tokens(sentence)).unwrap();
            assert_eq!(generated.action.value, handwritten.action, "{sentence}");
            assert_eq!(
                generated.input_name.value, handwritten.declaration.name,
                "{sentence}"
            );
            assert_eq!(
                generated.input_id.value, handwritten.declaration.id,
                "{sentence}"
            );
            match handwritten.kind {
                ActionInputKindAst::ExistingModel { model } => {
                    assert_eq!(
                        generated
                            .existing
                            .as_ref()
                            .map(|value| value.value.as_str()),
                        Some("기존")
                    );
                    assert_eq!(generated.input_type.value, model);
                }
                ActionInputKindAst::Value { value_type } => {
                    assert!(generated.existing.is_none());
                    let expected = match value_type {
                        TypeReferenceAst::String => "문자열".to_owned(),
                        TypeReferenceAst::Integer => "정수".to_owned(),
                        TypeReferenceAst::Boolean => "불리언".to_owned(),
                        TypeReferenceAst::Named(name) => name,
                    };
                    assert_eq!(generated.input_type.value, expected);
                }
            }
        }
    }

    #[test]
    fn generated_captures_preserve_attached_and_quoted_boundaries() {
        let screen =
            "`주문 화면`(order)에서는 `주문 항목`의 `상품 이름`, `수량 값`을 조회할 수 있다.";
        let generated = parse_screen(&sentence_tokens(screen)).unwrap();
        assert_eq!(
            &screen[generated.screen_name.start..generated.screen_name.end],
            "`주문 화면`"
        );
        assert_eq!(
            &screen[generated.model.start..generated.model.end],
            "`주문 항목`"
        );
        assert_eq!(
            &screen[generated.fields[1].start..generated.fields[1].end],
            "`수량 값`"
        );

        let sum = "주문의 총액은 주문 항목의 금액의 합계로 계산한다.";
        let generated = parse_sum_derivation(&sentence_tokens(sum)).unwrap();
        assert_eq!(
            &sum[generated.target_field.start..generated.target_field.end],
            "총액"
        );
        assert_eq!(
            &sum[generated.source_field.start..generated.source_field.end],
            "금액"
        );
    }

    #[test]
    fn generated_entries_reject_handwritten_failures_and_cross_productions() {
        let cases: &[(&str, RejectParser, bool)] = &[
            (
                "항목 작성 화면에서는 항목을 생성할 수 있다.",
                reject_screen,
                true,
            ),
            (
                "항목 입력 화면(input)에서는 항목의 수량, 을 입력할 수 있다.",
                reject_screen,
                true,
            ),
            (
                "항목 입력 화면(input)에서는 항목의 수량을 생성할 수 있다.",
                reject_screen,
                true,
            ),
            (
                "주문 취소가 실행되면 주문을 조회한다.",
                reject_action_data_mutation,
                true,
            ),
            (
                "항목의 합계는 항목의 금액의 합계로 계산한다",
                reject_sum,
                true,
            ),
            (
                "항목의 합계는 항목의 금액을 다시 계산한다.",
                reject_sum,
                true,
            ),
            (
                "항목의 금액이 바뀔 때 항목의 합계를 계산한다.",
                reject_recalculation,
                true,
            ),
            ("항목의 메모는 외부 공개에 사용한다.", reject_intent, true),
            (
                "항목의 합계는 항목의 금액의 합계로 계산한다.",
                reject_screen,
                false,
            ),
            (
                "항목 입력 화면(input)에서는 항목을 생성할 수 있다.",
                reject_sum,
                false,
            ),
            (
                "항목의 메모는 내부 관리에만 사용한다.",
                reject_recalculation,
                false,
            ),
        ];
        for (sentence, generated, oracle_rejects) in cases {
            assert_eq!(oracle(sentence).is_err(), *oracle_rejects, "{sentence}");
            assert!(generated(&sentence_tokens(sentence)).is_err(), "{sentence}");
        }
    }

    #[test]
    fn oracle_keeps_sentence_block_diagnostics_while_generated_shapes_match() {
        let source = "@모듈 검증(check)\n항목 입력 화면(input)에서는 항목의 금액을 입력할 수 있다.\n    잘못된 블록\n";
        let output = parse_document(source);
        assert!(
            output
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.rule_id == "RSPDL-KO-SYN-060")
        );
        assert!(
            parse_screen(&sentence_tokens(
                "항목 입력 화면(input)에서는 항목의 금액을 입력할 수 있다."
            ))
            .is_ok()
        );

        let source =
            "@모듈 검증(check)\n항목의 합계는 항목의 금액의 합계로 계산한다.\n    잘못된 블록\n";
        let output = parse_document(source);
        assert!(
            output
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.rule_id == "RSPDL-KO-SYN-063")
        );
        assert!(
            parse_sum_derivation(&sentence_tokens(
                "항목의 합계는 항목의 금액의 합계로 계산한다."
            ))
            .is_ok()
        );
    }

    fn reject_screen(tokens: &[Token]) -> Result<(), ParseError> {
        parse_screen(tokens).map(|_| ())
    }
    fn reject_action_data_mutation(tokens: &[Token]) -> Result<(), ParseError> {
        parse_action_data_mutation(tokens).map(|_| ())
    }
    fn reject_sum(tokens: &[Token]) -> Result<(), ParseError> {
        parse_sum_derivation(tokens).map(|_| ())
    }
    fn reject_recalculation(tokens: &[Token]) -> Result<(), ParseError> {
        parse_recalculation(tokens).map(|_| ())
    }
    fn reject_intent(tokens: &[Token]) -> Result<(), ParseError> {
        parse_field_intent(tokens).map(|_| ())
    }

    fn operation_word(operation: ScreenOperationKindAst) -> &'static str {
        match operation {
            ScreenOperationKindAst::Create => "생성할",
            ScreenOperationKindAst::Read => "조회할",
            ScreenOperationKindAst::Input => "입력할",
            ScreenOperationKindAst::Update => "수정할",
            ScreenOperationKindAst::Delete => "삭제할",
        }
    }

    fn data_mutation_word(mutation: DataMutationKindAst) -> &'static str {
        match mutation {
            DataMutationKindAst::Create => "생성한다",
            DataMutationKindAst::Update => "수정한다",
            DataMutationKindAst::Delete => "삭제한다",
        }
    }

    fn intent_words(intent: FieldIntentKindAst) -> &'static str {
        match intent {
            FieldIntentKindAst::Internal => "내부 관리에만 사용한다",
            FieldIntentKindAst::Hidden => "사용자 화면에서 조회하지 않는다",
        }
    }
}
