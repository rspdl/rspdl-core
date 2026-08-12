use rspdl_grammar_compiler::{
    Capture, Grammar, InputAdapter, ParseError, ParseMatch, TerminalMatch,
};

use crate::Span;
use crate::ast::{ConstraintExpressionAst, LiteralAst, OperandAst, RelationOperatorAst};
use crate::scanner::{Token, TokenKind};

use super::adapter::{match_literal, match_marked_ref};
use super::required_capture;

include!(concat!(env!("OUT_DIR"), "/constraint_grammar.rs"));

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GeneratedConstraint {
    pub model: Capture,
    pub left: Capture,
    pub right: GeneratedConstraintRight,
    pub operator: RelationOperatorAst,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum GeneratedConstraintRight {
    Field(Capture),
    Literal(LiteralAst),
}

impl GeneratedConstraint {
    pub(crate) fn expression(&self, span: Span) -> ConstraintExpressionAst {
        ConstraintExpressionAst {
            model: self.model.value.clone(),
            left: OperandAst::Field(self.left.value.clone()),
            operator: self.operator,
            right: match &self.right {
                GeneratedConstraintRight::Field(value) => OperandAst::Field(value.value.clone()),
                GeneratedConstraintRight::Literal(value) => OperandAst::Literal(value.clone()),
            },
            span,
        }
    }
}

pub(crate) fn parse_constraint(tokens: &[Token]) -> Result<GeneratedConstraint, ParseError> {
    let grammar: Grammar = generated_constraint_grammar();
    let parsed = grammar.parse("constraint_statement", tokens, &ConstraintTokenAdapter)?;
    let model = required_capture(&parsed, "model");

    if let Some(operator) = parsed.capture("field_operator") {
        return Ok(GeneratedConstraint {
            model,
            left: required_capture(&parsed, "field_left"),
            right: GeneratedConstraintRight::Field(required_capture(&parsed, "field_right")),
            operator: relation_operator(&operator.value),
        });
    }

    let left = required_capture(&parsed, "left");
    let (operator, literal) = literal_comparison(&parsed)
        .expect("validated constraint grammar always selects a literal comparison alternative");
    Ok(GeneratedConstraint {
        model,
        left,
        right: GeneratedConstraintRight::Literal(literal),
        operator,
    })
}

fn relation_operator(value: &str) -> RelationOperatorAst {
    match value {
        "같아야" => RelationOperatorAst::Equal,
        "달라야" => RelationOperatorAst::NotEqual,
        _ => unreachable!("grammar only captures supported field relation operators"),
    }
}

fn literal_comparison(parsed: &ParseMatch) -> Option<(RelationOperatorAst, LiteralAst)> {
    let integer = |name| {
        parsed
            .capture(name)
            .map(|capture| LiteralAst::Integer(capture.value.clone()))
    };
    for (name, operator) in [
        ("greater_attached", RelationOperatorAst::GreaterThan),
        ("greater", RelationOperatorAst::GreaterThan),
        ("less_attached", RelationOperatorAst::LessThan),
        ("less", RelationOperatorAst::LessThan),
        ("greater_or_equal", RelationOperatorAst::GreaterThanOrEqual),
        ("less_or_equal", RelationOperatorAst::LessThanOrEqual),
        ("integer_equal", RelationOperatorAst::Equal),
    ] {
        if let Some(literal) = integer(name) {
            return Some((operator, literal));
        }
    }
    for (name, operator, literal) in [
        (
            "string_equal",
            RelationOperatorAst::Equal,
            literal_string as fn(&str) -> LiteralAst,
        ),
        (
            "quoted_equal",
            RelationOperatorAst::Equal,
            literal_named as fn(&str) -> LiteralAst,
        ),
        (
            "word_equal",
            RelationOperatorAst::Equal,
            literal_word as fn(&str) -> LiteralAst,
        ),
        (
            "string_not_equal",
            RelationOperatorAst::NotEqual,
            literal_string as fn(&str) -> LiteralAst,
        ),
        (
            "quoted_not_equal",
            RelationOperatorAst::NotEqual,
            literal_named as fn(&str) -> LiteralAst,
        ),
        (
            "word_not_equal",
            RelationOperatorAst::NotEqual,
            literal_word as fn(&str) -> LiteralAst,
        ),
    ] {
        if let Some(capture) = parsed.capture(name) {
            return Some((operator, literal(&capture.value)));
        }
    }
    None
}

fn literal_string(value: &str) -> LiteralAst {
    LiteralAst::String(value.to_owned())
}

fn literal_named(value: &str) -> LiteralAst {
    LiteralAst::Named(value.to_owned())
}

fn literal_word(value: &str) -> LiteralAst {
    match value {
        "참" => LiteralAst::Boolean(true),
        "거짓" => LiteralAst::Boolean(false),
        _ if is_integer(value) => LiteralAst::Integer(value.to_owned()),
        _ => LiteralAst::Named(value.to_owned()),
    }
}

struct ConstraintTokenAdapter;

impl InputAdapter<Token> for ConstraintTokenAdapter {
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
            "integer" => integer(tokens, position),
            "integer_before" => integer_before(tokens, position, arguments),
            "string_equal" => string_literal(tokens, position, "이어야"),
            "quoted_equal" => quoted_identifier(tokens, position, "이어야"),
            "word_equal" => word_literal(tokens, position, &["이어야"], ""),
            "string_not_equal" => string_literal_not_equal(tokens, position),
            "quoted_not_equal" => quoted_identifier_not_equal(tokens, position),
            "word_not_equal" => word_literal(tokens, position, &["과", "와"], "달라야"),
            _ => Vec::new(),
        }
    }
}

fn integer(tokens: &[Token], position: usize) -> Vec<TerminalMatch> {
    let Some(Token {
        kind: TokenKind::Word(value),
        span,
    }) = tokens.get(position)
    else {
        return Vec::new();
    };
    is_integer(value)
        .then(|| TerminalMatch::new(position + 1, value, span.start, span.end))
        .into_iter()
        .collect()
}

fn integer_before(tokens: &[Token], position: usize, arguments: &[String]) -> Vec<TerminalMatch> {
    let Some(marker) = arguments.first() else {
        return Vec::new();
    };
    let Some(Token {
        kind: TokenKind::Word(value),
        span,
    }) = tokens.get(position)
    else {
        return Vec::new();
    };
    let Some(number) = value
        .strip_suffix(marker)
        .filter(|number| is_integer(number))
    else {
        return Vec::new();
    };
    vec![TerminalMatch::new(
        position + 1,
        number,
        span.start,
        span.end - marker.len(),
    )]
}

fn string_literal(tokens: &[Token], position: usize, suffix: &str) -> Vec<TerminalMatch> {
    let Some(Token {
        kind: TokenKind::StringLiteral(value),
        span,
    }) = tokens.get(position)
    else {
        return Vec::new();
    };
    matches!(tokens.get(position + 1).map(|token| &token.kind), Some(TokenKind::Word(word)) if word == suffix)
        .then(|| TerminalMatch::new(position + 2, value, span.start, span.end))
        .into_iter()
        .collect()
}

fn quoted_identifier(tokens: &[Token], position: usize, suffix: &str) -> Vec<TerminalMatch> {
    let Some(Token {
        kind: TokenKind::QuotedIdentifier(value),
        span,
    }) = tokens.get(position)
    else {
        return Vec::new();
    };
    matches!(tokens.get(position + 1).map(|token| &token.kind), Some(TokenKind::Word(word)) if word == suffix)
        .then(|| TerminalMatch::new(position + 2, value, span.start, span.end))
        .into_iter()
        .collect()
}

fn string_literal_not_equal(tokens: &[Token], position: usize) -> Vec<TerminalMatch> {
    let Some(Token {
        kind: TokenKind::StringLiteral(value),
        span,
    }) = tokens.get(position)
    else {
        return Vec::new();
    };
    (matches!(tokens.get(position + 1).map(|token| &token.kind), Some(TokenKind::Word(word)) if matches!(word.as_str(), "과" | "와"))
        && matches!(tokens.get(position + 2).map(|token| &token.kind), Some(TokenKind::Word(word)) if word == "달라야"))
        .then(|| TerminalMatch::new(position + 3, value, span.start, span.end))
        .into_iter()
        .collect()
}

fn quoted_identifier_not_equal(tokens: &[Token], position: usize) -> Vec<TerminalMatch> {
    let Some(Token {
        kind: TokenKind::QuotedIdentifier(value),
        span,
    }) = tokens.get(position)
    else {
        return Vec::new();
    };
    (matches!(tokens.get(position + 1).map(|token| &token.kind), Some(TokenKind::Word(word)) if matches!(word.as_str(), "과" | "와"))
        && matches!(tokens.get(position + 2).map(|token| &token.kind), Some(TokenKind::Word(word)) if word == "달라야"))
        .then(|| TerminalMatch::new(position + 3, value, span.start, span.end))
        .into_iter()
        .collect()
}

fn word_literal(
    tokens: &[Token],
    position: usize,
    markers: &[&str],
    following_word: &str,
) -> Vec<TerminalMatch> {
    if matches!(tokens.get(position).map(|token| &token.kind), Some(TokenKind::Word(value)) if is_integer(value))
    {
        return Vec::new();
    }
    let mut parts = Vec::new();
    let mut index = position;
    while let Some(Token {
        kind: TokenKind::Word(value),
        ..
    }) = tokens.get(index)
    {
        for marker in markers {
            if let Some(last) = value.strip_suffix(marker).filter(|last| !last.is_empty()) {
                if !following_word.is_empty() && !parts.is_empty() {
                    return Vec::new();
                }
                if !following_word.is_empty()
                    && !matches!(tokens.get(index + 1).map(|token| &token.kind), Some(TokenKind::Word(word)) if word == following_word)
                {
                    return Vec::new();
                }
                parts.push(last);
                let first = &tokens[position];
                let last_token = &tokens[index];
                return vec![TerminalMatch::new(
                    index + 1 + usize::from(!following_word.is_empty()),
                    parts.join(" "),
                    first.span.start,
                    last_token.span.end - marker.len(),
                )];
            }
        }
        parts.push(value);
        index += 1;
    }
    Vec::new()
}

fn is_integer(value: &str) -> bool {
    let digits = value.strip_prefix('-').unwrap_or(value);
    !digits.is_empty()
        && digits.chars().all(|character| character.is_ascii_digit())
        && (digits == "0" || !digits.starts_with('0'))
        && value != "-0"
}

#[cfg(test)]
mod tests {
    use crate::ast::{ConstraintExpressionAst, DeclarationAst};
    use crate::scanner::TokenKind;
    use crate::{Diagnostic, parse, scan};

    use super::*;

    fn constraint_tokens(sentence: &str) -> Vec<Token> {
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

    fn handwritten_constraint(sentence: &str) -> Result<ConstraintExpressionAst, Vec<Diagnostic>> {
        let source = format!(
            "@모듈 비교(comparison)\n항목(item)은 다음 필드들로 구성되어 있다.\n    값(value): 필수 정수\n    다른 값(other): 선택 정수\n{sentence}\n"
        );
        let parsed = parse(&source);
        if parsed.diagnostics.iter().any(Diagnostic::is_error) {
            return Err(parsed.diagnostics);
        }
        parsed
            .document
            .expect("valid source has a document")
            .declarations
            .into_iter()
            .find_map(|declaration| match declaration {
                DeclarationAst::Constraint(value) => Some(value.expression),
                _ => None,
            })
            .ok_or(parsed.diagnostics)
    }

    fn generated_expression(sentence: &str) -> Result<ConstraintExpressionAst, ParseError> {
        let tokens = constraint_tokens(sentence);
        let span = tokens
            .first()
            .unwrap()
            .span
            .join(tokens.last().unwrap().span);
        parse_constraint(&tokens).map(|value| value.expression(span))
    }

    #[test]
    fn generated_constraint_matches_handwritten_ast_for_every_supported_shape() {
        let cases = [
            "항목의 값은 0보다 커야 한다.",
            "항목의 값은 -1 보다 커야 한다.",
            "항목의 값은 0 이상이어야 한다.",
            "항목의 값은 100보다 작아야 한다.",
            "항목의 값은 100 이하여야 한다.",
            "항목의 값은 참이어야 한다.",
            "항목의 값은 거짓이어야 한다.",
            "항목의 값은 승인 완료이어야 한다.",
            "항목의 값은 \"취소됨\"이어야 한다.",
            "항목의 값은 `승인 완료` 이어야 한다.",
            "항목의 값은 0과 달라야 한다.",
            "항목의 값은 \"취소됨\"와 달라야 한다.",
            "항목의 값은 `승인 완료` 와 달라야 한다.",
            "항목의 값과 다른 값은 같아야 한다.",
            "항목의 값과 다른 값은 달라야 한다.",
            "`항목` 의 `값` 과 `다른 값` 은 같아야 한다.",
        ];
        for sentence in cases {
            let handwritten = handwritten_constraint(sentence)
                .unwrap_or_else(|diagnostics| panic!("{sentence}: {diagnostics:?}"));
            let generated = generated_expression(sentence)
                .unwrap_or_else(|error| panic!("{sentence}: {error:?}"));
            assert_eq!(generated.model, handwritten.model, "{sentence}");
            assert_eq!(generated.left, handwritten.left, "{sentence}");
            assert_eq!(generated.operator, handwritten.operator, "{sentence}");
            assert_eq!(generated.right, handwritten.right, "{sentence}");
        }
    }

    #[test]
    fn generated_constraint_rejects_every_shape_rejected_by_handwritten_oracle() {
        let cases = [
            "항목 값은 0보다 커야 한다.",
            "항목의 값이 0보다 커야 한다.",
            "항목의 값은 0보다 한다.",
            "항목의 값은 0보다 커야 한다",
            "항목의 값은 0보다 커야 한다 뒤에.",
            "항목의 값은 01보다 커야 한다.",
            "항목의 값은 -0보다 커야 한다.",
            "항목의 값은 0과 같아야 한다.",
            "항목의 값은 승인 완료와 달라야 한다.",
            "항목의 값과 다른 값이 같아야 한다.",
            "항목의 값과 다른 값은 같다 한다.",
        ];
        for sentence in cases {
            assert!(handwritten_constraint(sentence).is_err(), "{sentence}");
            assert!(generated_expression(sentence).is_err(), "{sentence}");
        }
    }

    #[test]
    fn generated_constraint_rejects_other_sentence_productions() {
        let cases = [
            "관리자는 항목의 값을 변경할 수 있다.",
            "항목은 하나 이상 존재해야 한다.",
            "소유자, 검토자 중 둘 이상은 동시에 성립할 수 없다.",
            "항목의 값은 항목의 다른 값의 합계로 계산한다.",
        ];
        for sentence in cases {
            assert!(generated_expression(sentence).is_err(), "{sentence}");
        }
    }

    #[test]
    fn generated_constraint_capture_spans_exclude_attached_markers() {
        for (sentence, expected) in [
            ("항목의 값은 0보다 커야 한다.", "0"),
            ("항목의 값은 100보다 작아야 한다.", "100"),
        ] {
            let generated = parse_constraint(&constraint_tokens(sentence)).unwrap();
            assert_eq!(
                &sentence[generated.model.start..generated.model.end],
                "항목"
            );
            assert_eq!(&sentence[generated.left.start..generated.left.end], "값");
            let GeneratedConstraintRight::Literal(LiteralAst::Integer(value)) = generated.right
            else {
                panic!("ordered comparison should capture an integer literal");
            };
            assert_eq!(value, expected);
        }
    }
}
