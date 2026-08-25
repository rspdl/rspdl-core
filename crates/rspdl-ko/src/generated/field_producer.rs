//! Executable grammar shadow for pre-mutation field-producer sentences.
use super::adapter::{match_literal, match_marked_ref};
use super::required_capture;
use crate::ast::ProducerTriggerKindAst;
use crate::scanner::Token;
use rspdl_grammar_compiler::{Capture, Grammar, InputAdapter, ParseError, TerminalMatch};
include!(concat!(env!("OUT_DIR"), "/field_producer_grammar.rs"));

#[derive(Debug)]
pub(crate) struct GeneratedFieldProducer {
    pub producer_name: Capture,
    pub producer_id: Capture,
    pub action: Capture,
    pub trigger_kind: ProducerTriggerKindAst,
    pub condition_input: Option<Capture>,
    pub condition_variant: Option<Capture>,
    pub output_model: Capture,
    pub output_field: Capture,
    pub input: Option<Capture>,
    pub existing_input: Option<Capture>,
    pub existing_field: Option<Capture>,
    pub constant: Option<Capture>,
    #[allow(dead_code)]
    pub template: Option<Capture>,
}
pub(crate) fn parse_field_producer(tokens: &[Token]) -> Result<GeneratedFieldProducer, ParseError> {
    let grammar: Grammar = generated_field_producer_grammar();
    let parsed = grammar.parse("field_producer_statement", tokens, &Adapter)?;
    Ok(GeneratedFieldProducer {
        producer_name: required_capture(&parsed, "producer_name"),
        producer_id: required_capture(&parsed, "producer_id"),
        action: parsed
            .capture("action")
            .or_else(|| parsed.capture("conditional_action"))
            .cloned()
            .expect("field producer grammar always captures an action"),
        trigger_kind: match parsed
            .capture("trigger_verb")
            .map(|capture| capture.value.as_str())
        {
            Some("발생할") => ProducerTriggerKindAst::Event,
            Some("실행될") | None => ProducerTriggerKindAst::Action,
            Some(_) => unreachable!("field producer grammar only captures supported trigger verbs"),
        },
        condition_input: parsed.capture("condition_input").cloned(),
        condition_variant: parsed.capture("condition_variant").cloned(),
        output_model: required_capture(&parsed, "output_model"),
        output_field: required_capture(&parsed, "output_field"),
        input: parsed.capture("input").cloned(),
        existing_input: parsed.capture("existing_input").cloned(),
        existing_field: parsed.capture("existing_field").cloned(),
        constant: parsed.capture("constant").cloned(),
        template: parsed.capture("template").cloned(),
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
            "template_string" => template_string(t, p, a),
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
fn template_string(tokens: &[Token], position: usize, markers: &[String]) -> Vec<TerminalMatch> {
    let Some(Token {
        kind: crate::TokenKind::StringLiteral(value),
        span,
    }) = tokens.get(position)
    else {
        return Vec::new();
    };
    matches!(tokens.get(position + 1).map(|token| &token.kind), Some(crate::TokenKind::Word(marker)) if markers.iter().any(|expected| expected == marker))
        .then(|| TerminalMatch::new(position + 2, value, span.start, span.end))
        .into_iter()
        .collect()
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
    fn executable_field_producer_grammar_accepts_action_and_event_source_shapes() {
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
            (
                "알림 제목 기록(event_title_binding)은 요청 접수됨이 발생할 때 알림 제목을 알림의 제목으로 기록한다.",
                0,
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
            assert_eq!(parsed.action.value, hand.trigger.name);
            assert_eq!(parsed.trigger_kind, hand.trigger.kind);
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
    fn executable_field_producer_grammar_matches_the_conditional_shape() {
        let sentence = "접수 제목 기록(received_title)은 전달의 요청 상태가 접수됨이면 상수 \"요청이 접수되었습니다\"를 알림의 제목으로 기록한다.";
        let tokens = scan(sentence)
            .tokens
            .into_iter()
            .filter(|token| !matches!(token.kind, TokenKind::Newline))
            .collect::<Vec<_>>();
        let generated = parse_field_producer(&tokens).unwrap();
        assert_eq!(generated.action.value, "전달");
        assert_eq!(generated.condition_input.unwrap().value, "요청 상태");
        assert_eq!(generated.condition_variant.unwrap().value, "접수됨");

        let document = parse_document(&format!("@모듈 검증(check)\n{sentence}\n"));
        let DeclarationAst::FieldProducer(handwritten) =
            document.document.unwrap().declarations.remove(0)
        else {
            panic!()
        };
        assert_eq!(handwritten.trigger.name, "전달");
        assert_eq!(
            handwritten
                .condition
                .map(|condition| (condition.input, condition.variant)),
            Some(("요청 상태".into(), "접수됨".into()))
        );
    }

    #[test]
    fn executable_field_producer_grammar_matches_output_only_template_shape() {
        let sentence = "알림 내용 조합(content_template)은 점검 요청 전달이 실행될 때 \"{제목} 점검이 전달되었습니다.\"를 점검 전달 알림의 내용으로 조합한다.";
        let tokens = scan(sentence)
            .tokens
            .into_iter()
            .filter(|token| !matches!(token.kind, TokenKind::Newline))
            .collect::<Vec<_>>();
        let generated = parse_field_producer(&tokens).unwrap();
        assert_eq!(
            generated.template.unwrap().value,
            "{제목} 점검이 전달되었습니다."
        );
        let document = parse_document(&format!("@모듈 검증(check)\n{sentence}\n"));
        let DeclarationAst::FieldProducer(handwritten) =
            document.document.unwrap().declarations.remove(0)
        else {
            panic!()
        };
        assert!(matches!(
            handwritten.source,
            FieldProducerSourceAst::Template { value } if value == "{제목} 점검이 전달되었습니다."
        ));
    }

    #[test]
    fn executable_field_producer_grammar_rejects_foreign_sentence_shapes() {
        for sentence in [
            "조건 기록(binding)은 전달이 실행될 때 상태가 접수됨이면 제목을 알림의 제목으로 기록한다.",
            "템플릿 기록(binding)은 전달이 실행될 때 제목을 알림의 제목으로 채운다.",
            "알림 내용 조합(content_template)은 전달의 상태가 접수됨이면 \"{제목}\"를 알림의 내용으로 조합한다.",
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
