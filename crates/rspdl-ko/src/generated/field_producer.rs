//! Executable grammar shadow for unconditional field-producer sentences.
use super::adapter::{match_literal, match_marked_ref};
use super::required_capture;
use crate::scanner::Token;
use rspdl_grammar_compiler::{Capture, Grammar, InputAdapter, ParseError, TerminalMatch};
include!(concat!(env!("OUT_DIR"), "/field_producer_grammar.rs"));

#[derive(Debug)]
pub(crate) struct GeneratedFieldProducer {
    pub producer_name: Capture,
    pub producer_id: Capture,
    pub action: Capture,
    pub output_model: Capture,
    pub output_field: Capture,
    pub input: Option<Capture>,
    pub existing_input: Option<Capture>,
    pub existing_field: Option<Capture>,
    pub constant: Option<Capture>,
}
pub(crate) fn parse_field_producer(tokens: &[Token]) -> Result<GeneratedFieldProducer, ParseError> {
    let grammar: Grammar = generated_field_producer_grammar();
    let parsed = grammar.parse("field_producer_statement", tokens, &Adapter)?;
    Ok(GeneratedFieldProducer {
        producer_name: required_capture(&parsed, "producer_name"),
        producer_id: required_capture(&parsed, "producer_id"),
        action: required_capture(&parsed, "action"),
        output_model: required_capture(&parsed, "output_model"),
        output_field: required_capture(&parsed, "output_field"),
        input: parsed.capture("input").cloned(),
        existing_input: parsed.capture("existing_input").cloned(),
        existing_field: parsed.capture("existing_field").cloned(),
        constant: parsed.capture("constant").cloned(),
    })
}
struct Adapter;
impl InputAdapter<Token> for Adapter {
    fn match_literal(&self, t: &[Token], p: usize, l: &str) -> Option<TerminalMatch> {
        match_literal(t, p, l)
    }
    fn match_contextual(&self, t: &[Token], p: usize, m: &str, a: &[String]) -> Vec<TerminalMatch> {
        match m {
            "marked_ref" => match_marked_ref(t, p, a),
            "source_direct" => direct_source(t, p, a),
            "surface_name" => surface_name_prefixes(t, p),
            "canonical_id" => canonical_id(t, p),
            "literal" => literal(t, p, a),
            _ => Vec::new(),
        }
    }
}
fn surface_name_prefixes(tokens: &[Token], position: usize) -> Vec<TerminalMatch> {
    let Some(first) = tokens.get(position) else {
        return Vec::new();
    };
    let mut parts = Vec::new();
    let mut matches = Vec::new();
    for (i, t) in tokens.iter().enumerate().skip(position) {
        let value = match &t.kind {
            crate::TokenKind::Word(v) | crate::TokenKind::QuotedIdentifier(v) => v,
            _ => break,
        };
        parts.push(value.clone());
        matches.push(TerminalMatch::new(
            i + 1,
            parts.join(" "),
            first.span.start,
            t.span.end,
        ));
    }
    matches
}
fn canonical_id(tokens: &[Token], position: usize) -> Vec<TerminalMatch> {
    match tokens.get(position) {
        Some(Token {
            kind: crate::TokenKind::CanonicalId(v),
            span,
        }) if !v.is_empty() => vec![TerminalMatch::new(position + 1, v, span.start, span.end)],
        _ => Vec::new(),
    }
}
fn literal(tokens: &[Token], position: usize, markers: &[String]) -> Vec<TerminalMatch> {
    let Some(token) = tokens.get(position) else {
        return Vec::new();
    };
    match &token.kind {
        crate::TokenKind::StringLiteral(value) | crate::TokenKind::QuotedIdentifier(value) => {
            match tokens.get(position + 1) {
                Some(next) if matches!(&next.kind,crate::TokenKind::Word(marker) if markers.iter().any(|expected|expected==marker)) =>
                {
                    vec![TerminalMatch::new(
                        position + 2,
                        value,
                        next.span.start,
                        next.span.end,
                    )]
                }
                _ => Vec::new(),
            }
        }
        _ => match_marked_ref(tokens, position, markers),
    }
}
fn direct_source(tokens: &[Token], position: usize, markers: &[String]) -> Vec<TerminalMatch> {
    match_marked_ref(tokens, position, markers)
        .into_iter()
        .filter(|matched| {
            !matches!(tokens.get(position).map(|token| &token.kind), Some(crate::TokenKind::Word(word)) if word == "상수")
                && !tokens[position..matched.end].iter().any(
                |token| matches!(&token.kind, crate::TokenKind::Word(word)
                    if word.ends_with('의') || word.ends_with("이면") || word.ends_with("라면")),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::parse_field_producer;
    use crate::ast::{DeclarationAst, FieldProducerSourceAst};
    use crate::{TokenKind, parse as parse_document, scan};
    #[test]
    fn executable_field_producer_grammar_accepts_the_three_source_shapes() {
        for (sentence, kind) in [
            (
                "알림 제목 기록(title_binding)은 점검 요청 전달이 실행될 때 알림 제목을 점검 요청 전달 알림의 제목으로 기록한다.",
                0,
            ),
            (
                "요청 제목 기록(request_title_binding)은 점검 요청 전달이 실행될 때 대상 요청의 제목을 점검 요청 전달 알림의 요청 제목으로 기록한다.",
                1,
            ),
            (
                "재시도 횟수 기록(retry_binding)은 점검 요청 전달이 실행될 때 상수 0을 점검 요청 전달 알림의 재시도 횟수로 기록한다.",
                2,
            ),
        ] {
            let tokens = scan(sentence)
                .tokens
                .into_iter()
                .filter(|token| !matches!(token.kind, TokenKind::Newline))
                .collect::<Vec<_>>();
            let parsed = parse_field_producer(&tokens)
                .unwrap_or_else(|error| panic!("{sentence}: {error:?}"));
            assert!(!parsed.producer_name.value.is_empty());
            assert!(!parsed.producer_id.value.is_empty());
            assert!(!parsed.action.value.is_empty());
            assert!(!parsed.output_model.value.is_empty());
            assert!(!parsed.output_field.value.is_empty());
            let doc = parse_document(&format!("@모듈 검증(check)\n{sentence}\n"));
            let DeclarationAst::FieldProducer(hand) = doc.document.unwrap().declarations.remove(0)
            else {
                panic!()
            };
            assert_eq!(parsed.producer_name.value, hand.declaration.name);
            assert_eq!(parsed.producer_id.value, hand.declaration.id);
            assert_eq!(parsed.action.value, hand.action);
            assert_eq!(parsed.output_model.value, hand.output_model);
            assert_eq!(parsed.output_field.value, hand.output_field);
            match (kind, hand.source) {
                (0, FieldProducerSourceAst::ActionInput { input }) => {
                    assert_eq!(parsed.input.unwrap().value, input)
                }
                (1, FieldProducerSourceAst::InputField { input, field }) => {
                    assert_eq!(parsed.existing_input.unwrap().value, input);
                    assert_eq!(parsed.existing_field.unwrap().value, field)
                }
                (2, FieldProducerSourceAst::Constant { literal }) => assert_eq!(
                    parsed.constant.unwrap().value,
                    match literal {
                        crate::ast::LiteralAst::Integer(v) => v,
                        _ => panic!(),
                    }
                ),
                _ => panic!(),
            }
        }
    }

    #[test]
    fn executable_field_producer_grammar_rejects_foreign_sentence_shapes() {
        for sentence in [
            "조건 기록(binding)은 전달이 실행될 때 상태가 접수됨이면 제목을 알림의 제목으로 기록한다.",
            "템플릿 기록(binding)은 전달이 실행될 때 제목을 알림의 제목으로 채운다.",
        ] {
            let tokens = scan(sentence)
                .tokens
                .into_iter()
                .filter(|token| !matches!(token.kind, TokenKind::Newline))
                .collect::<Vec<_>>();
            assert!(
                parse_field_producer(&tokens).is_err(),
                "foreign shape must not match the executable grammar: {sentence}"
            );
        }
    }
}
