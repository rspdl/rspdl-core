use rspdl_grammar_compiler::{Grammar, InputAdapter, ParseError, ParseMatch, TerminalMatch};

use crate::Span;
use crate::ast::{NamedIdAst, RelationAst, RelationalConstraintAst, RelationalConstraintKindAst};
use crate::scanner::{Token, TokenKind};

use super::adapter::{match_literal, match_marked_ref};
use super::required_capture;

include!(concat!(env!("OUT_DIR"), "/relation_grammar.rs"));

fn parse_relation(tokens: &[Token]) -> Result<RelationAst, ParseError> {
    let grammar: Grammar = generated_relation_grammar();
    let parsed = grammar.parse("relation_declaration", tokens, &RelationAdapter)?;
    let source = required_capture(&parsed, "source_model");
    let target = parsed.capture("target_model");
    let name = required_capture(&parsed, "relation_name");
    let id = required_capture(&parsed, "relation_id");
    let mut parameter_models = vec![source.value];
    if let Some(target) = target {
        parameter_models.push(target.value.clone());
    }
    Ok(RelationAst {
        declaration: NamedIdAst {
            name: name.value,
            id: id.value,
            span: Span {
                start: name.start,
                end: id.end,
            },
        },
        parameter_models,
        span: sentence_span(tokens),
    })
}

fn parse_relational_constraint(tokens: &[Token]) -> Result<RelationalConstraintAst, ParseError> {
    let grammar: Grammar = generated_relation_grammar();
    let parsed = grammar.parse("relational_constraint", tokens, &RelationAdapter)?;
    let kind = required_capture(&parsed, "kind");
    let constraint = match kind.value.as_str() {
        "하나 이상 존재해야 한다" => RelationalConstraintKindAst::NonEmpty {
            model: required_capture(&parsed, "model").value,
        },
        "모든" => RelationalConstraintKindAst::Required {
            model: required_capture(&parsed, "model").value,
            relation: required_capture(&parsed, "relation").value,
        },
        "각" => RelationalConstraintKindAst::Unique {
            model: required_capture(&parsed, "model").value,
            relation: required_capture(&parsed, "relation").value,
        },
        "중 둘 이상은 동시에 성립할 수 없다" => {
            RelationalConstraintKindAst::Exclusive {
                relations: capture_values(&parsed, "relations"),
            }
        }
        "중 하나 이상은 항상 성립해야 한다" => {
            RelationalConstraintKindAst::Exhaustive {
                relations: capture_values(&parsed, "relations"),
            }
        }
        "동시에 성립할 수 있다" => RelationalConstraintKindAst::Coexistent {
            relations: capture_values(&parsed, "relations"),
        },
        value => panic!("validated relation grammar returned unknown kind {value:?}"),
    };
    Ok(RelationalConstraintAst {
        constraint,
        span: sentence_span(tokens),
    })
}

fn capture_values(parsed: &ParseMatch, name: &str) -> Vec<String> {
    parsed
        .captures
        .get(name)
        .expect("validated relation group captures references")
        .iter()
        .map(|capture| capture.value.clone())
        .collect()
}

fn sentence_span(tokens: &[Token]) -> Span {
    Span {
        start: tokens.first().map_or(0, |token| token.span.start),
        end: tokens.last().map_or(0, |token| token.span.end),
    }
}

struct RelationAdapter;

impl InputAdapter<Token> for RelationAdapter {
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
            "surface_name" => match_surface_names(tokens, position),
            "canonical_id" => match_canonical_id(tokens, position),
            "comma_ref" => match_comma_reference(tokens, position),
            _ => Vec::new(),
        }
    }
}

fn match_surface_names(tokens: &[Token], position: usize) -> Vec<TerminalMatch> {
    let mut values = Vec::new();
    let mut parts = Vec::new();
    for (index, token) in tokens.iter().enumerate().skip(position) {
        let value = match &token.kind {
            TokenKind::Word(value) | TokenKind::QuotedIdentifier(value) => value,
            _ => break,
        };
        parts.push(value.clone());
        values.push(TerminalMatch::new(
            index + 1,
            parts.join(" "),
            tokens[position].span.start,
            token.span.end,
        ));
    }
    values
}

fn match_canonical_id(tokens: &[Token], position: usize) -> Vec<TerminalMatch> {
    match tokens.get(position) {
        Some(Token {
            kind: TokenKind::CanonicalId(value),
            span,
        }) => vec![TerminalMatch::new(
            position + 1,
            value,
            span.start,
            span.end,
        )],
        _ => Vec::new(),
    }
}

fn match_comma_reference(tokens: &[Token], position: usize) -> Vec<TerminalMatch> {
    let mut parts = Vec::new();
    let Some(first) = tokens.get(position) else {
        return Vec::new();
    };
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
    use crate::ast::DeclarationAst;
    use crate::{Diagnostic, parse, scan};

    use super::*;

    const PREFIX: &str = "@모듈 관계(relations)\n";

    fn sentence_tokens(sentence: &str) -> Vec<Token> {
        let source = format!("{PREFIX}{sentence}\n");
        let scanned = scan(&source);
        assert!(scanned.diagnostics.is_empty(), "{:?}", scanned.diagnostics);
        scanned
            .tokens
            .into_iter()
            .filter(|token| {
                token.span.start >= PREFIX.len()
                    && !matches!(
                        token.kind,
                        TokenKind::Newline | TokenKind::Indent | TokenKind::Dedent
                    )
            })
            .collect()
    }

    fn handwritten_relation(sentence: &str) -> Result<RelationAst, Vec<Diagnostic>> {
        let parsed = parse(&format!("{PREFIX}{sentence}\n"));
        if parsed.diagnostics.iter().any(Diagnostic::is_error) {
            return Err(parsed.diagnostics);
        }
        parsed
            .document
            .expect("valid source has a document")
            .declarations
            .into_iter()
            .find_map(|declaration| match declaration {
                DeclarationAst::Relation(value) => Some(value),
                _ => None,
            })
            .ok_or(parsed.diagnostics)
    }

    fn handwritten_constraint(sentence: &str) -> Result<RelationalConstraintAst, Vec<Diagnostic>> {
        let parsed = parse(&format!("{PREFIX}{sentence}\n"));
        if parsed.diagnostics.iter().any(Diagnostic::is_error) {
            return Err(parsed.diagnostics);
        }
        parsed
            .document
            .expect("valid source has a document")
            .declarations
            .into_iter()
            .find_map(|declaration| match declaration {
                DeclarationAst::RelationalConstraint(value) => Some(value),
                _ => None,
            })
            .ok_or(parsed.diagnostics)
    }

    #[test]
    fn generated_relations_match_handwritten_ast() {
        for sentence in [
            "프로젝트는 사용자를 소유자(owner)로 가질 수 있다.",
            "`비용 프로젝트` 는 `회계 사용자` 를 `주 검토자`(reviewer)로 가질 수 있다.",
            "사용자는 내부(internal)에 해당할 수 있다.",
            "`외부 사용자` 는 `외부 분류`(external)에 해당할 수 있다.",
        ] {
            let expected = handwritten_relation(sentence)
                .unwrap_or_else(|diagnostics| panic!("{sentence}: {diagnostics:?}"));
            let actual = parse_relation(&sentence_tokens(sentence))
                .unwrap_or_else(|error| panic!("{sentence}: {error:?}"));
            assert_eq!(actual, expected, "{sentence}");
        }
    }

    #[test]
    fn generated_relation_constraints_match_handwritten_ast() {
        for sentence in [
            "프로젝트는 하나 이상 존재해야 한다.",
            "모든 프로젝트는 소유자를 하나 이상 가져야 한다.",
            "각 프로젝트는 소유자를 최대 하나만 가질 수 있다.",
            "내부, 외부 중 둘 이상은 동시에 성립할 수 없다.",
            "승인 중, 검토 완료 중 하나 이상은 항상 성립해야 한다.",
            "소유자, 소유자 후보는 동시에 성립할 수 있다.",
            "`주 소유자`, `보조 소유자` 는 동시에 성립할 수 있다.",
        ] {
            let expected = handwritten_constraint(sentence)
                .unwrap_or_else(|diagnostics| panic!("{sentence}: {diagnostics:?}"));
            let actual = parse_relational_constraint(&sentence_tokens(sentence))
                .unwrap_or_else(|error| panic!("{sentence}: {error:?}"));
            assert_eq!(actual, expected, "{sentence}");
        }
    }

    #[test]
    fn generated_relation_grammar_rejects_invalid_and_foreign_shapes() {
        for sentence in [
            "프로젝트는 사용자를 소유자(owner) 가질 수 있다.",
            "프로젝트는 사용자를 소유자(owner)로 가질 수 있다",
            "프로젝트는 사용자를 소유자(owner)로 가질 수 있다 뒤에.",
            "관리자는 신청의 상태를 변경할 수 있다.",
            "프로젝트의 금액은 0보다 커야 한다.",
        ] {
            assert!(handwritten_relation(sentence).is_err(), "{sentence}");
            assert!(
                parse_relation(&sentence_tokens(sentence)).is_err(),
                "{sentence}"
            );
        }

        for sentence in [
            "프로젝트는 하나 존재해야 한다.",
            "모든 프로젝트는 소유자를 가져야 한다.",
            "내부 중 둘 이상은 동시에 성립할 수 없다.",
            "내부, 외부 동시에 성립할 수 있다.",
            "관리자는 신청의 상태를 변경할 수 있다.",
        ] {
            assert!(handwritten_constraint(sentence).is_err(), "{sentence}");
            assert!(
                parse_relational_constraint(&sentence_tokens(sentence)).is_err(),
                "{sentence}"
            );
        }
    }
}
