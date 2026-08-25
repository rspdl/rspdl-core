//! Shadow parser for module and declaration headers plus indented CFG items.
//!
//! The public parser still owns document ordering and indentation.  Keeping
//! that small shell outside the flat-token grammar makes the generated rules
//! directly comparable with the existing parser without changing recovery.

use rspdl_grammar_compiler::{
    Capture, Grammar, InputAdapter, ParseError, ParseMatch, TerminalMatch,
};

use crate::Span;
use crate::ast::{FieldAst, TypeReferenceAst};
use crate::scanner::{Token, TokenKind};

use super::required_capture;

include!(concat!(env!("OUT_DIR"), "/declarations_grammar.rs"));

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GeneratedNamedId {
    pub name: Capture,
    pub id: Capture,
}

impl GeneratedNamedId {
    pub(crate) fn span(&self) -> Span {
        Span {
            start: self.name.start,
            end: self.id.end,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GeneratedField {
    pub declaration: GeneratedNamedId,
    pub required: bool,
    pub value_type: TypeReferenceAst,
    pub span: Span,
}

pub(crate) fn parse_module_header(tokens: &[Token]) -> Result<GeneratedNamedId, ParseError> {
    parse_named_id("module_header", tokens)
}

pub(crate) fn parse_enum_header(tokens: &[Token]) -> Result<GeneratedNamedId, ParseError> {
    parse_named_id("enum_header", tokens)
}

pub(crate) fn parse_enum_value(tokens: &[Token]) -> Result<GeneratedNamedId, ParseError> {
    parse_named_id("enum_value", tokens)
}

pub(crate) fn parse_data_model_header(tokens: &[Token]) -> Result<GeneratedNamedId, ParseError> {
    parse_named_id("data_model_header", tokens)
}

pub(crate) fn parse_role_header(tokens: &[Token]) -> Result<GeneratedNamedId, ParseError> {
    parse_named_id("role_header", tokens)
}

pub(crate) fn parse_action_header(tokens: &[Token]) -> Result<GeneratedNamedId, ParseError> {
    parse_named_id("action_header", tokens)
}

pub(crate) fn parse_event_header(tokens: &[Token]) -> Result<GeneratedNamedId, ParseError> {
    parse_named_id("event_header", tokens)
}

pub(crate) fn parse_field_item(tokens: &[Token]) -> Result<GeneratedField, ParseError> {
    let parsed = parse_rule("field_item", tokens)?;
    let required = required_capture(&parsed, "required").value == "필수";
    let value_type = type_reference(&required_capture(&parsed, "value_type").value);
    Ok(GeneratedField {
        declaration: named_id(&parsed),
        required,
        value_type,
        span: tokens
            .first()
            .map(|token| token.span)
            .unwrap_or_default()
            .join(tokens.last().map(|token| token.span).unwrap_or_default()),
    })
}

pub(crate) fn field_ast(field: GeneratedField) -> FieldAst {
    let span = field.declaration.span();
    FieldAst {
        declaration: crate::NamedIdAst {
            name: field.declaration.name.value,
            id: field.declaration.id.value,
            span,
        },
        required: field.required,
        value_type: field.value_type,
        span: field.span,
    }
}

fn parse_named_id(rule: &str, tokens: &[Token]) -> Result<GeneratedNamedId, ParseError> {
    Ok(named_id(&parse_rule(rule, tokens)?))
}

fn parse_rule(rule: &str, tokens: &[Token]) -> Result<ParseMatch, ParseError> {
    let grammar: Grammar = generated_declarations_grammar();
    grammar.parse(rule, tokens, &DeclarationTokenAdapter)
}

fn named_id(parsed: &ParseMatch) -> GeneratedNamedId {
    GeneratedNamedId {
        name: required_capture(parsed, "name"),
        id: required_capture(parsed, "id"),
    }
}

fn type_reference(name: &str) -> TypeReferenceAst {
    match name {
        "문자열" => TypeReferenceAst::String,
        "정수" => TypeReferenceAst::Integer,
        "불리언" => TypeReferenceAst::Boolean,
        _ => TypeReferenceAst::Named(name.to_owned()),
    }
}

/// A declaration-only adapter. `surface_name` returns every non-empty
/// Word/quoted-identifier prefix; a following ID or end-of-input selects the
/// unique complete declaration shape without encoding AST data in a matcher.
struct DeclarationTokenAdapter;

impl InputAdapter<Token> for DeclarationTokenAdapter {
    fn match_literal(
        &self,
        tokens: &[Token],
        position: usize,
        literal: &str,
    ) -> Option<TerminalMatch> {
        let token = tokens.get(position)?;
        let matches = match &token.kind {
            TokenKind::Word(value) => value == literal,
            TokenKind::Period => literal == ".",
            TokenKind::Colon => literal == ":",
            _ => false,
        };
        matches.then(|| TerminalMatch::new(position + 1, literal, token.span.start, token.span.end))
    }

    fn match_contextual(
        &self,
        tokens: &[Token],
        position: usize,
        matcher: &str,
        _arguments: &[String],
    ) -> Vec<TerminalMatch> {
        match matcher {
            "canonical_id" => match tokens.get(position) {
                Some(Token {
                    kind: TokenKind::CanonicalId(value),
                    span,
                }) if !value.is_empty() => {
                    vec![TerminalMatch::new(
                        position + 1,
                        value,
                        span.start,
                        span.end,
                    )]
                }
                _ => Vec::new(),
            },
            "surface_name" => surface_name_prefixes(tokens, position),
            _ => Vec::new(),
        }
    }
}

fn surface_name_prefixes(tokens: &[Token], position: usize) -> Vec<TerminalMatch> {
    let mut parts = Vec::new();
    let mut matches = Vec::new();
    let Some(first) = tokens.get(position) else {
        return matches;
    };
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

#[cfg(test)]
mod tests {
    use crate::ast::{DataModelAst, DeclarationAst, EnumAst};
    use crate::scanner::TokenKind;
    use crate::{Diagnostic, NamedIdAst, parse, scan};

    use super::*;

    type NamedParser = fn(&[Token]) -> Result<GeneratedNamedId, ParseError>;

    fn tokens(source: &str) -> Vec<Token> {
        let scanned = scan(source);
        assert!(
            scanned.diagnostics.is_empty(),
            "{source}: {:?}",
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

    fn tokens_at(source: &str, line: &str) -> Vec<Token> {
        let start = source.find(line).expect("line is present in source");
        let end = start + line.len();
        scan(source)
            .tokens
            .into_iter()
            .filter(|token| {
                token.span.start >= start
                    && token.span.end <= end
                    && !matches!(
                        token.kind,
                        TokenKind::Newline | TokenKind::Indent | TokenKind::Dedent
                    )
            })
            .collect()
    }

    fn named(generated: GeneratedNamedId) -> NamedIdAst {
        let span = generated.span();
        NamedIdAst {
            name: generated.name.value,
            id: generated.id.value,
            span,
        }
    }

    #[test]
    fn generated_declarations_match_handwritten_ast_shapes_and_spans() {
        let source = "@모듈 비용 승인(expense)\n비용 상태(status)는 다음 값 중 하나다.\n    작성 중(draft)\n    `승인 완료`(approved)\n비용 신청(request)은 다음 필드들로 구성되어 있다.\n    식별자(id): 필수 문자열\n    승인 상태(status): 선택 비용 상태\n회계 관리자(accounting_manager)는 역할이다.\n상태 변경(change_state)은 행동이다.\n승인 요청 접수됨(request_received)은 사건이다.\n";
        let document = parse(source).document.expect("valid source has document");
        assert!(parse(source).diagnostics.is_empty());

        assert_eq!(
            named(parse_module_header(&tokens_at(source, "@모듈 비용 승인(expense)")).unwrap()),
            document.module.declaration
        );
        let DeclarationAst::Enum(EnumAst {
            declaration,
            values,
            ..
        }) = &document.declarations[0]
        else {
            panic!("enum")
        };
        assert_eq!(
            named(
                parse_enum_header(&tokens_at(source, "비용 상태(status)는 다음 값 중 하나다."))
                    .unwrap()
            ),
            *declaration
        );
        assert_eq!(
            named(parse_enum_value(&tokens_at(source, "작성 중(draft)")).unwrap()),
            values[0].declaration
        );
        assert_eq!(
            named(parse_enum_value(&tokens_at(source, "`승인 완료`(approved)")).unwrap()),
            values[1].declaration
        );

        let DeclarationAst::DataModel(DataModelAst {
            declaration,
            fields,
            ..
        }) = &document.declarations[1]
        else {
            panic!("model")
        };
        assert_eq!(
            named(
                parse_data_model_header(&tokens_at(
                    source,
                    "비용 신청(request)은 다음 필드들로 구성되어 있다."
                ))
                .unwrap(),
            ),
            *declaration
        );
        assert_eq!(
            field_ast(parse_field_item(&tokens_at(source, "식별자(id): 필수 문자열")).unwrap()),
            fields[0]
        );
        assert_eq!(
            field_ast(
                parse_field_item(&tokens_at(source, "승인 상태(status): 선택 비용 상태")).unwrap()
            ),
            fields[1]
        );

        let DeclarationAst::Role(role) = &document.declarations[2] else {
            panic!("role")
        };
        assert_eq!(
            named(
                parse_role_header(&tokens_at(
                    source,
                    "회계 관리자(accounting_manager)는 역할이다."
                ))
                .unwrap()
            ),
            role.declaration
        );
        let DeclarationAst::Action(action) = &document.declarations[3] else {
            panic!("action")
        };
        assert_eq!(
            named(
                parse_action_header(&tokens_at(source, "상태 변경(change_state)은 행동이다."))
                    .unwrap()
            ),
            action.declaration
        );
        let DeclarationAst::Event(event) = &document.declarations[4] else {
            panic!("event")
        };
        assert_eq!(
            named(
                parse_event_header(&tokens_at(
                    source,
                    "승인 요청 접수됨(request_received)은 사건이다."
                ))
                .unwrap()
            ),
            event.declaration
        );
    }

    #[test]
    fn generated_declarations_reject_oracle_invalid_shapes() {
        let cases: &[(&str, NamedParser)] = &[
            ("@모듈 비용 승인(expense).", parse_module_header),
            ("비용 상태(status)는 다음 값 중 하나다", parse_enum_header),
            (
                "비용 신청(request) 다음 필드들로 구성되어 있다.",
                parse_data_model_header,
            ),
            (
                "회계 관리자(accounting_manager)는 역할이다",
                parse_role_header,
            ),
            ("변경(change)은 행동이다 뒤에.", parse_action_header),
            ("접수됨(received)은 사건이다 뒤에.", parse_event_header),
            ("@역할 관리자(manager)", parse_module_header),
        ];
        for (source, generated) in cases {
            assert!(generated(&tokens(source)).is_err(), "{source}");
        }
        for source in [
            "상태(): 필수 문자열",
            "상태(status) 필수 문자열",
            "상태(status): 필수 문자열.",
        ] {
            assert!(parse_field_item(&tokens(source)).is_err(), "{source}");
        }
    }

    #[test]
    fn generated_line_rules_do_not_consume_neighboring_block_or_top_level_lines() {
        let source = "@모듈 테스트(test)\n상태(state)는 다음 값 중 하나다.\n  시작(start)\n관리자(manager)는 역할이다.\n";
        let parsed = parse(source);
        assert!(
            !parsed.diagnostics.iter().any(Diagnostic::is_error),
            "{:?}",
            parsed.diagnostics
        );
        assert!(parse_enum_header(&tokens("상태(state)는 다음 값 중 하나다.")).is_ok());
        assert!(parse_enum_value(&tokens("시작(start)")).is_ok());
        assert!(parse_role_header(&tokens("관리자(manager)는 역할이다.")).is_ok());
        assert!(parse_event_header(&tokens("접수됨(received)은 사건이다.")).is_ok());
    }

    #[test]
    fn quoted_names_and_annotation_false_positives_remain_bounded() {
        assert!(parse_module_header(&tokens("@모듈 `비용: 승인`(expense)")).is_ok());
        assert!(parse_role_header(&tokens("역할이다 관리자(manager)는 역할이다.")).is_ok());
        assert!(parse_enum_value(&tokens("상태(status):")).is_err());
        assert!(parse_field_item(&tokens("`상태: 이름`(status): 필수 문자열.")).is_err());
    }
}
