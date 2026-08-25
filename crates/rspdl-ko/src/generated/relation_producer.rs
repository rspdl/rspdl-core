//! Executable grammar shadow for direct output relation-slot producers.
use super::adapter::{match_literal, match_marked_ref};
use super::required_capture;
use crate::ast::ProducerTriggerKindAst;
use crate::scanner::Token;
use rspdl_grammar_compiler::{Capture, Grammar, InputAdapter, ParseError, TerminalMatch};
include!(concat!(env!("OUT_DIR"), "/relation_producer_grammar.rs"));

#[allow(dead_code)]
pub(crate) struct GeneratedRelationProducer {
    pub producer_name: Capture,
    pub producer_id: Capture,
    pub action: Capture,
    pub trigger_kind: ProducerTriggerKindAst,
    pub input: Capture,
    pub output_model: Capture,
    pub relation: Capture,
}

pub(crate) fn parse_relation_producer(
    tokens: &[Token],
) -> Result<GeneratedRelationProducer, ParseError> {
    let grammar: Grammar = generated_relation_producer_grammar();
    let parsed = grammar.parse("relation_producer_statement", tokens, &Adapter)?;
    Ok(GeneratedRelationProducer {
        producer_name: required_capture(&parsed, "producer_name"),
        producer_id: required_capture(&parsed, "producer_id"),
        action: required_capture(&parsed, "action"),
        trigger_kind: match required_capture(&parsed, "trigger_verb").value.as_str() {
            "실행될" => ProducerTriggerKindAst::Action,
            "발생할" => ProducerTriggerKindAst::Event,
            _ => unreachable!("relation producer grammar only captures supported trigger verbs"),
        },
        input: required_capture(&parsed, "input"),
        output_model: required_capture(&parsed, "output_model"),
        relation: required_capture(&parsed, "relation"),
    })
}
struct Adapter;
impl InputAdapter<Token> for Adapter {
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
        args: &[String],
    ) -> Vec<TerminalMatch> {
        match matcher {
            "marked_ref" => match_marked_ref(tokens, position, args),
            "surface_name" => surface_name_prefixes(tokens, position),
            "canonical_id" => canonical_id(tokens, position),
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
    for (index, token) in tokens.iter().enumerate().skip(position) {
        let value = match &token.kind {
            crate::TokenKind::Word(value) | crate::TokenKind::QuotedIdentifier(value) => value,
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
            kind: crate::TokenKind::CanonicalId(value),
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

#[cfg(test)]
mod tests {
    use super::parse_relation_producer;
    use crate::{DeclarationAst, TokenKind, parse, scan};
    #[test]
    fn generated_relation_producer_matches_action_and_event_handwritten_parser() {
        for sentence in [
            "수신자 연결(recipient_binding)은 점검 요청 전달이 실행될 때 수신 기술자를 점검 전달 알림의 수신자로 연결한다.",
            "수신자 연결(event_recipient_binding)은 요청 접수됨이 발생할 때 수신 기술자를 알림의 수신자로 연결한다.",
        ] {
            let tokens = scan(sentence)
                .tokens
                .into_iter()
                .filter(|token| !matches!(token.kind, TokenKind::Newline))
                .collect::<Vec<_>>();
            let generated = parse_relation_producer(&tokens).unwrap();
            let parsed = parse(&format!("@모듈 검증(check)\n{sentence}\n"));
            let DeclarationAst::RelationProducer(handwritten) =
                parsed.document.unwrap().declarations.remove(0)
            else {
                panic!()
            };
            assert_eq!(generated.producer_id.value, handwritten.declaration.id);
            assert_eq!(generated.action.value, handwritten.trigger.name);
            assert_eq!(generated.trigger_kind, handwritten.trigger.kind);
            assert_eq!(generated.input.value, handwritten.input);
            assert_eq!(generated.output_model.value, handwritten.output_model);
            assert_eq!(generated.relation.value, handwritten.relation);
        }
    }
}
