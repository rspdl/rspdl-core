//! Executable shadow parser for the two conditional-creation sentences.

use rspdl_grammar_compiler::{
    Capture, Grammar, InputAdapter, ParseError, ParseMatch, TerminalMatch,
};

use crate::scanner::Token;

use super::adapter::{match_literal, match_marked_ref};
use super::required_capture;

include!(concat!(env!("OUT_DIR"), "/creation_grammar.rs"));

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GeneratedCreationBranch {
    pub branch_name: Capture,
    pub branch_id: Capture,
    pub action: Capture,
    pub input: Capture,
    pub variant: Capture,
    pub output_model: Capture,
    pub decision: Capture,
}

pub(crate) fn parse_creation_branch(
    tokens: &[Token],
) -> Result<GeneratedCreationBranch, ParseError> {
    let parsed = parse("creation_branch_statement", tokens)?;
    Ok(GeneratedCreationBranch {
        branch_name: required_capture(&parsed, "branch_name"),
        branch_id: required_capture(&parsed, "branch_id"),
        action: required_capture(&parsed, "action"),
        input: required_capture(&parsed, "input"),
        variant: required_capture(&parsed, "variant"),
        output_model: required_capture(&parsed, "output_model"),
        decision: required_capture(&parsed, "decision"),
    })
}

fn parse(entry: &str, tokens: &[Token]) -> Result<ParseMatch, ParseError> {
    let grammar: Grammar = generated_creation_grammar();
    grammar.parse(entry, tokens, &CreationTokenAdapter)
}

struct CreationTokenAdapter;

impl InputAdapter<Token> for CreationTokenAdapter {
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
    use crate::ast::{CreationDecisionAst, DeclarationAst};
    use crate::scanner::TokenKind;
    use crate::{Diagnostic, parse as parse_document, scan};

    use super::*;

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

    fn oracle(sentence: &str) -> DeclarationAst {
        let parsed = parse_document(&format!("@모듈 검증(check)\n{sentence}\n"));
        assert!(
            !parsed.diagnostics.iter().any(Diagnostic::is_error),
            "{sentence}: {:?}",
            parsed.diagnostics
        );
        parsed
            .document
            .unwrap()
            .declarations
            .into_iter()
            .next()
            .unwrap()
    }

    #[test]
    fn generated_creation_branches_match_handwritten_create_and_skip_captures() {
        for (sentence, expected) in [
            (
                "접수 상태 알림 생성(received_notice_create)은 점검 요청 전달의 요청 상태가 접수됨이면 점검 요청 전달 알림을 하나 생성한다.",
                CreationDecisionAst::Create,
            ),
            (
                "보류 상태 알림 미생성(on_hold_notice_skip)은 점검 요청 전달의 요청 상태가 보류됨이면 점검 요청 전달 알림을 생성하지 않는다.",
                CreationDecisionAst::Skip,
            ),
            (
                "예 알림 생성(yes_notice_create)은 전달의 선택이 예라면 알림을 하나 생성한다.",
                CreationDecisionAst::Create,
            ),
            (
                "형식 오류 알림 생성(bad-id)은 전달의 상태가 접수됨이면 알림을 하나 생성한다.",
                CreationDecisionAst::Create,
            ),
        ] {
            let DeclarationAst::CreationBranch(handwritten) = oracle(sentence) else {
                panic!("oracle did not return a creation branch")
            };
            let generated = parse_creation_branch(&sentence_tokens(sentence)).unwrap();
            assert_eq!(generated.branch_name.value, handwritten.declaration.name);
            assert_eq!(generated.branch_id.value, handwritten.declaration.id);
            assert_eq!(generated.action.value, handwritten.action);
            assert_eq!(generated.input.value, handwritten.input);
            assert_eq!(generated.variant.value, handwritten.variant);
            assert_eq!(generated.output_model.value, handwritten.output_model);
            assert_eq!(
                generated.decision.value,
                match expected {
                    CreationDecisionAst::Create => "하나 생성한다",
                    CreationDecisionAst::Skip => "생성하지 않는다",
                }
            );
            assert_eq!(handwritten.decision, expected);
        }
    }

    #[test]
    fn generated_creation_grammar_rejects_foreign_shapes() {
        for sentence in [
            "알림 생성(notice_create)은 전달의 상태가 접수됨이면 알림을 생성한다.",
            "알림 생성(notice_create)은 전달의 상태가 접수됨이면 알림을 두 개 생성한다.",
            "알림 생성은 전달의 상태가 접수됨이면 알림을 하나 생성한다.",
            "알림 생성(notice_create)은 전달의 상태가 접수됨이면 알림을 생성하지 않는다 뒤에.",
        ] {
            assert!(
                parse_creation_branch(&sentence_tokens(sentence)).is_err(),
                "{sentence}"
            );
        }
    }
}
