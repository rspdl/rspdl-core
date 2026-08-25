use serde::Serialize;

use crate::ast::*;
use crate::scanner::{Token, TokenKind, scan};
use crate::{Diagnostic, Span};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ParseOutput {
    pub document: Option<DocumentAst>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug)]
struct Line {
    indent: usize,
    tokens: Vec<Token>,
    span: Span,
}

pub fn parse(source: &str) -> ParseOutput {
    let scanned = scan(source);
    let mut diagnostics = scanned.diagnostics;
    let lines = logical_lines(&scanned.tokens);
    let mut cursor = 0usize;

    let Some(module_line) = lines.first() else {
        diagnostics.push(Diagnostic::error(
            "RSPDL-KO-SYN-001",
            "ko.syntax.module_required",
            Span::default(),
        ));
        return ParseOutput {
            document: None,
            diagnostics,
        };
    };
    let module = match parse_module(module_line, &mut diagnostics) {
        Ok(module) => module,
        Err(diagnostic) => {
            diagnostics.push(diagnostic);
            return ParseOutput {
                document: None,
                diagnostics,
            };
        }
    };
    cursor += 1;

    let mut declarations = Vec::new();
    while cursor < lines.len() {
        let line = &lines[cursor];
        if line.indent != 0 {
            diagnostics.push(Diagnostic::error(
                "RSPDL-KO-SYN-002",
                "ko.syntax.unexpected_top_level_indent",
                line.span,
            ));
            cursor += 1;
            continue;
        }

        let block_end = (cursor + 1..lines.len())
            .find(|index| lines[*index].indent == 0)
            .unwrap_or(lines.len());
        let body = &lines[cursor + 1..block_end];
        let kind = declaration_kind(line);
        let result = match kind {
            Some(DeclarationKind::Enum) => {
                parse_enum(line, body, &mut diagnostics).map(DeclarationAst::Enum)
            }
            Some(DeclarationKind::DataModel) => {
                parse_model(line, body, &mut diagnostics).map(DeclarationAst::DataModel)
            }
            Some(DeclarationKind::Relation) => {
                parse_relation(line, body, &mut diagnostics).map(DeclarationAst::Relation)
            }
            Some(DeclarationKind::RelationalConstraint(kind)) => {
                parse_relational_constraint(line, body, kind)
                    .map(DeclarationAst::RelationalConstraint)
            }
            Some(DeclarationKind::Screen) => {
                parse_screen(line, body, &mut diagnostics).map(DeclarationAst::Screen)
            }
            Some(DeclarationKind::ActionDataMutation) => {
                parse_action_data_mutation(line, body).map(DeclarationAst::ActionDataMutation)
            }
            Some(DeclarationKind::SumDerivation) => {
                parse_sum_derivation(line, body).map(DeclarationAst::SumDerivation)
            }
            Some(DeclarationKind::Recalculation) => {
                parse_recalculation(line, body).map(DeclarationAst::Recalculation)
            }
            Some(DeclarationKind::FieldIntent) => {
                parse_field_intent(line, body).map(DeclarationAst::FieldIntent)
            }
            Some(DeclarationKind::Constraint) => {
                parse_constraint(line, body, &mut diagnostics).map(DeclarationAst::Constraint)
            }
            Some(DeclarationKind::Role) => {
                parse_role(line, body, &mut diagnostics).map(|declaration| {
                    DeclarationAst::Role(RoleAst {
                        declaration,
                        span: line.span,
                    })
                })
            }
            Some(DeclarationKind::Action) => {
                parse_action(line, body, &mut diagnostics).map(|declaration| {
                    DeclarationAst::Action(ActionAst {
                        declaration,
                        span: line.span,
                    })
                })
            }
            Some(DeclarationKind::Event) => {
                parse_event(line, body, &mut diagnostics).map(|declaration| {
                    DeclarationAst::Event(EventAst {
                        declaration,
                        span: line.span,
                    })
                })
            }
            Some(DeclarationKind::ActionInput) => {
                parse_action_input(line, body, &mut diagnostics).map(DeclarationAst::ActionInput)
            }
            Some(DeclarationKind::EventInput) => {
                parse_event_input(line, body, &mut diagnostics).map(DeclarationAst::EventInput)
            }
            Some(DeclarationKind::CreationBranch) => {
                parse_creation_branch(line, body, &mut diagnostics)
                    .map(DeclarationAst::CreationBranch)
            }
            Some(DeclarationKind::FieldProducer) => {
                parse_field_producer(line, body, &mut diagnostics)
                    .map(DeclarationAst::FieldProducer)
            }
            Some(DeclarationKind::RelationProducer) => {
                parse_relation_producer(line, body, &mut diagnostics)
                    .map(DeclarationAst::RelationProducer)
            }
            Some(DeclarationKind::Policy) => {
                parse_policy(line, body, &mut diagnostics).map(DeclarationAst::Policy)
            }
            _ if word_at(line, 0).is_some_and(|word| word.starts_with('@')) => {
                Err(Diagnostic::error(
                    "RSPDL-KO-SYN-003",
                    "ko.syntax.domain_annotation_forbidden",
                    line.span,
                )
                .with_argument("annotation", word_at(line, 0).unwrap_or("@")))
            }
            _ => Err(Diagnostic::error(
                "RSPDL-KO-SYN-003",
                "ko.syntax.unknown_top_level_declaration",
                line.span,
            )),
        };
        match result {
            Ok(declaration) => declarations.push(declaration),
            Err(diagnostic) => diagnostics.push(diagnostic),
        }

        let expects_block = matches!(
            kind,
            Some(DeclarationKind::Enum | DeclarationKind::DataModel)
        );
        if expects_block {
            cursor = block_end;
        } else {
            cursor += 1;
        }
    }

    ParseOutput {
        document: Some(DocumentAst {
            module,
            declarations,
        }),
        diagnostics,
    }
}

fn logical_lines(tokens: &[Token]) -> Vec<Line> {
    let mut lines = Vec::new();
    let mut indent = 0usize;
    let mut current: Vec<Token> = Vec::new();
    for token in tokens {
        match token.kind {
            TokenKind::Indent => indent += 1,
            TokenKind::Dedent => indent = indent.saturating_sub(1),
            TokenKind::Newline => {
                if let (Some(first), Some(last)) = (current.first(), current.last()) {
                    lines.push(Line {
                        indent,
                        span: first.span.join(last.span),
                        tokens: std::mem::take(&mut current),
                    });
                }
            }
            _ => current.push(token.clone()),
        }
    }
    lines
}

#[derive(Clone, Copy)]
enum DeclarationKind {
    Enum,
    DataModel,
    Relation,
    RelationalConstraint(RelationalConstraintDeclarationKind),
    Screen,
    ActionDataMutation,
    SumDerivation,
    Recalculation,
    FieldIntent,
    Constraint,
    Role,
    Action,
    Event,
    ActionInput,
    EventInput,
    CreationBranch,
    FieldProducer,
    RelationProducer,
    Policy,
}

#[derive(Clone, Copy)]
enum RelationalConstraintDeclarationKind {
    NonEmpty,
    Required,
    Unique,
    Exclusive,
    Exhaustive,
    Coexistent,
}

fn declaration_kind(line: &Line) -> Option<DeclarationKind> {
    match word_at(line, 0) {
        _ if is_enum_header(line) => Some(DeclarationKind::Enum),
        _ if is_data_model_header(line) => Some(DeclarationKind::DataModel),
        _ if is_role_sentence(line) => Some(DeclarationKind::Role),
        _ if is_action_sentence(line) => Some(DeclarationKind::Action),
        _ if is_event_sentence(line) => Some(DeclarationKind::Event),
        _ if is_action_input_sentence(line) => Some(DeclarationKind::ActionInput),
        _ if is_event_input_sentence(line) => Some(DeclarationKind::EventInput),
        _ if is_creation_branch_sentence(line) => Some(DeclarationKind::CreationBranch),
        _ if is_template_producer_sentence(line) => Some(DeclarationKind::FieldProducer),
        _ if is_field_producer_sentence(line) => Some(DeclarationKind::FieldProducer),
        _ if is_relation_producer_sentence(line) => Some(DeclarationKind::RelationProducer),
        _ if is_relation_sentence(line) => Some(DeclarationKind::Relation),
        _ if is_nonempty_sentence(line) => Some(DeclarationKind::RelationalConstraint(
            RelationalConstraintDeclarationKind::NonEmpty,
        )),
        _ if is_required_relation_sentence(line) => Some(DeclarationKind::RelationalConstraint(
            RelationalConstraintDeclarationKind::Required,
        )),
        _ if is_unique_relation_sentence(line) => Some(DeclarationKind::RelationalConstraint(
            RelationalConstraintDeclarationKind::Unique,
        )),
        _ if is_exclusive_relation_sentence(line) => Some(DeclarationKind::RelationalConstraint(
            RelationalConstraintDeclarationKind::Exclusive,
        )),
        _ if is_exhaustive_relation_sentence(line) => Some(DeclarationKind::RelationalConstraint(
            RelationalConstraintDeclarationKind::Exhaustive,
        )),
        _ if is_coexistent_relation_sentence(line) => Some(DeclarationKind::RelationalConstraint(
            RelationalConstraintDeclarationKind::Coexistent,
        )),
        _ if is_screen_sentence(line) => Some(DeclarationKind::Screen),
        _ if is_action_data_mutation_sentence(line) => Some(DeclarationKind::ActionDataMutation),
        _ if is_recalculation_sentence(line) => Some(DeclarationKind::Recalculation),
        _ if is_sum_derivation_sentence(line) => Some(DeclarationKind::SumDerivation),
        _ if is_field_intent_sentence(line) => Some(DeclarationKind::FieldIntent),
        _ if is_rule_sentence(line) => {
            if is_policy_sentence(line) {
                Some(DeclarationKind::Policy)
            } else {
                Some(DeclarationKind::Constraint)
            }
        }
        _ => None,
    }
}

fn parse_relation(
    line: &Line,
    body: &[Line],
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<RelationAst, Diagnostic> {
    reject_sentence_body(body, line.span, "relation")?;
    let tokens = sentence_tokens(line)?;
    let mut cursor = BodyCursor::new(tokens, line.span);
    let (source_model, source_marker) = cursor.marked_ref(&["은", "는"])?;
    let unary = is_unary_relation_sentence(line);
    let (target_model, target_marker) = if unary {
        (None, None)
    } else {
        let (model, marker) = cursor.marked_ref(&["을", "를"])?;
        (Some(model), Some(marker))
    };
    let id_index = tokens
        .iter()
        .enumerate()
        .skip(cursor.index)
        .find_map(|(index, token)| matches!(token.kind, TokenKind::CanonicalId(_)).then_some(index))
        .ok_or_else(|| {
            Diagnostic::error(
                "RSPDL-KO-SYN-070",
                "ko.syntax.relation_stable_id_required",
                line.span,
            )
        })?;
    let declaration = parse_name_with_id_tokens(tokens, cursor.index, id_index, line.span)?;
    cursor.index = id_index + 1;
    let mut parameter_models = vec![source_model.clone()];
    if unary {
        cursor.expect_word("에")?;
        cursor.expect_word("해당할")?;
    } else {
        match cursor.next_word() {
            Some("로" | "으로") => {}
            _ => return Err(cursor.error("ko.syntax.relation_direction_marker_required")),
        }
        cursor.expect_word("가질")?;
        parameter_models.push(
            target_model
                .clone()
                .expect("binary relation has a target model"),
        );
    }
    cursor.expect_word("수")?;
    cursor.expect_word("있다")?;
    cursor.expect_end()?;
    lint_marker(
        &source_model,
        &source_marker,
        "은",
        "는",
        line.span,
        diagnostics,
    );
    if let (Some(model), Some(marker)) = (&target_model, &target_marker) {
        lint_marker(model, marker, "을", "를", line.span, diagnostics);
    }
    Ok(RelationAst {
        declaration,
        parameter_models,
        span: line.span,
    })
}

fn parse_relational_constraint(
    line: &Line,
    body: &[Line],
    kind: RelationalConstraintDeclarationKind,
) -> Result<RelationalConstraintAst, Diagnostic> {
    reject_sentence_body(body, line.span, "relational_constraint")?;
    let tokens = sentence_tokens(line)?;
    let mut cursor = BodyCursor::new(tokens, line.span);
    let constraint = match kind {
        RelationalConstraintDeclarationKind::NonEmpty => {
            let (model, _) = cursor.marked_ref(&["은", "는"])?;
            cursor.expect_word("하나")?;
            cursor.expect_word("이상")?;
            cursor.expect_word("존재해야")?;
            cursor.expect_word("한다")?;
            cursor.expect_end()?;
            RelationalConstraintKindAst::NonEmpty { model }
        }
        RelationalConstraintDeclarationKind::Required => {
            cursor.expect_word("모든")?;
            let (model, _) = cursor.marked_ref(&["은", "는"])?;
            let (relation, _) = cursor.marked_ref(&["을", "를"])?;
            cursor.expect_word("하나")?;
            cursor.expect_word("이상")?;
            cursor.expect_word("가져야")?;
            cursor.expect_word("한다")?;
            cursor.expect_end()?;
            RelationalConstraintKindAst::Required { model, relation }
        }
        RelationalConstraintDeclarationKind::Unique => {
            cursor.expect_word("각")?;
            let (model, _) = cursor.marked_ref(&["은", "는"])?;
            let (relation, _) = cursor.marked_ref(&["을", "를"])?;
            cursor.expect_word("최대")?;
            cursor.expect_word("하나만")?;
            cursor.expect_word("가질")?;
            cursor.expect_word("수")?;
            cursor.expect_word("있다")?;
            cursor.expect_end()?;
            RelationalConstraintKindAst::Unique { model, relation }
        }
        RelationalConstraintDeclarationKind::Exclusive => {
            let separator = group_separator(tokens, 6, line.span)?;
            let relations = relation_group(&tokens[..separator], line.span)?;
            cursor.index = separator + 1;
            cursor.expect_word("둘")?;
            cursor.expect_word("이상은")?;
            cursor.expect_word("동시에")?;
            cursor.expect_word("성립할")?;
            cursor.expect_word("수")?;
            cursor.expect_word("없다")?;
            cursor.expect_end()?;
            RelationalConstraintKindAst::Exclusive { relations }
        }
        RelationalConstraintDeclarationKind::Exhaustive => {
            let separator = group_separator(tokens, 5, line.span)?;
            let relations = relation_group(&tokens[..separator], line.span)?;
            cursor.index = separator + 1;
            cursor.expect_word("하나")?;
            cursor.expect_word("이상은")?;
            cursor.expect_word("항상")?;
            cursor.expect_word("성립해야")?;
            cursor.expect_word("한다")?;
            cursor.expect_end()?;
            RelationalConstraintKindAst::Exhaustive { relations }
        }
        RelationalConstraintDeclarationKind::Coexistent => {
            let suffix_len = 4;
            let prefix_end = tokens.len().checked_sub(suffix_len).ok_or_else(|| {
                Diagnostic::error(
                    "RSPDL-KO-SYN-071",
                    "ko.syntax.relational_constraint_group_references",
                    line.span,
                )
            })?;
            let (references, _) =
                strip_final_marker(&tokens[..prefix_end], &["은", "는"], line.span)?;
            let relations = relation_group(&references, line.span)?;
            cursor.index = prefix_end;
            cursor.expect_word("동시에")?;
            cursor.expect_word("성립할")?;
            cursor.expect_word("수")?;
            cursor.expect_word("있다")?;
            cursor.expect_end()?;
            RelationalConstraintKindAst::Coexistent { relations }
        }
    };
    Ok(RelationalConstraintAst {
        constraint,
        span: line.span,
    })
}

fn relation_group(tokens: &[Token], span: Span) -> Result<Vec<String>, Diagnostic> {
    let references = parse_reference_list(tokens, span)?;
    if references.len() >= 2 {
        Ok(references)
    } else {
        Err(Diagnostic::error(
            "RSPDL-KO-SYN-071",
            "ko.syntax.relational_constraint_group_references",
            span,
        ))
    }
}

fn group_separator(tokens: &[Token], tail_len: usize, span: Span) -> Result<usize, Diagnostic> {
    let separator = tokens
        .len()
        .checked_sub(tail_len + 1)
        .filter(|index| matches!(&tokens[*index].kind, TokenKind::Word(word) if word == "중"));
    separator.ok_or_else(|| {
        Diagnostic::error(
            "RSPDL-KO-SYN-071",
            "ko.syntax.relational_constraint_group_references",
            span,
        )
    })
}

fn strip_final_marker(
    tokens: &[Token],
    markers: &[&str],
    span: Span,
) -> Result<(Vec<Token>, String), Diagnostic> {
    let mut values = tokens.to_vec();
    if values.is_empty() {
        return Err(Diagnostic::error(
            "RSPDL-KO-SYN-071",
            "ko.syntax.reference_list_required",
            span,
        ));
    }
    let separate_marker = match values.as_slice() {
        [
            ..,
            Token {
                kind: TokenKind::QuotedIdentifier(_),
                ..
            },
            Token {
                kind: TokenKind::Word(word),
                ..
            },
        ] if markers.contains(&word.as_str()) => Some(word.clone()),
        _ => None,
    };
    if let Some(marker) = separate_marker {
        values.pop();
        return Ok((values, marker));
    }
    let last = values.last_mut().expect("values is not empty");
    if let TokenKind::Word(word) = &mut last.kind
        && let Some((base, marker)) = markers.iter().find_map(|marker| {
            word.strip_suffix(marker)
                .filter(|base| !base.is_empty())
                .map(|base| (base.to_owned(), (*marker).to_owned()))
        })
    {
        *word = base;
        return Ok((values, marker));
    }
    Err(Diagnostic::error(
        "RSPDL-KO-SYN-071",
        "ko.syntax.reference_list_final_marker_required",
        span,
    ))
}

fn parse_reference_list(tokens: &[Token], span: Span) -> Result<Vec<String>, Diagnostic> {
    if tokens.is_empty() {
        return Err(Diagnostic::error(
            "RSPDL-KO-SYN-071",
            "ko.syntax.reference_list_required",
            span,
        ));
    }
    let mut values = Vec::new();
    let mut start = 0;
    for end in 0..=tokens.len() {
        if end != tokens.len() && !matches!(tokens[end].kind, TokenKind::Comma) {
            continue;
        }
        if start == end {
            return Err(Diagnostic::error(
                "RSPDL-KO-SYN-071",
                "ko.syntax.reference_list_empty_name",
                span,
            ));
        }
        let mut parts = Vec::new();
        for token in &tokens[start..end] {
            match &token.kind {
                TokenKind::Word(value) | TokenKind::QuotedIdentifier(value) => {
                    parts.push(value.clone());
                }
                _ => {
                    return Err(Diagnostic::error(
                        "RSPDL-KO-SYN-071",
                        "ko.syntax.reference_list_invalid",
                        token.span,
                    ));
                }
            }
        }
        values.push(parts.join(" "));
        start = end + 1;
    }
    Ok(values)
}

fn is_screen_sentence(line: &Line) -> bool {
    line.tokens
        .iter()
        .position(|token| matches!(token.kind, TokenKind::CanonicalId(_)))
        .and_then(|index| word_at(line, index + 1))
        == Some("에서는")
        && last_sentence_word(line, 0) == Some("있다")
        && last_sentence_word(line, 1) == Some("수")
}

fn is_action_data_mutation_sentence(line: &Line) -> bool {
    let operation = last_sentence_word(line, 0);
    matches!(operation, Some("생성한다" | "수정한다" | "삭제한다"))
        && line
            .tokens
            .iter()
            .any(|token| matches!(&token.kind, TokenKind::Word(word) if word == "실행되면"))
}

fn is_enum_header(line: &Line) -> bool {
    sentence_words_end_with(line, &["다음", "값", "중", "하나다"])
}

fn is_role_sentence(line: &Line) -> bool {
    sentence_words_end_with(line, &["역할이다"])
}

fn is_action_sentence(line: &Line) -> bool {
    sentence_words_end_with(line, &["행동이다"])
}

fn is_event_sentence(line: &Line) -> bool {
    sentence_words_end_with(line, &["사건이다"])
}

fn is_action_input_sentence(line: &Line) -> bool {
    sentence_words_end_with(line, &["입력받는다"])
}

fn is_event_input_sentence(line: &Line) -> bool {
    sentence_words_end_with(line, &["담는다"])
}

fn is_creation_branch_sentence(line: &Line) -> bool {
    let ends_with_creation = last_sentence_word(line, 0) == Some("생성한다")
        || (last_sentence_word(line, 0) == Some("않는다")
            && last_sentence_word(line, 1) == Some("생성하지"));
    ends_with_creation
        && line.tokens.iter().any(|token| {
            matches!(&token.kind, TokenKind::Word(word) if matches!(word.as_str(), "이면" | "라면") || word.ends_with("이면") || word.ends_with("라면"))
        })
}

fn is_field_producer_sentence(line: &Line) -> bool {
    last_sentence_word(line, 0) == Some("기록한다")
        && line.tokens.iter().any(|token| {
            matches!(&token.kind, TokenKind::Word(word)
                if matches!(word.as_str(), "실행될" | "발생할" | "이면" | "라면")
                    || word.ends_with("이면") || word.ends_with("라면"))
        })
}

fn is_template_producer_sentence(line: &Line) -> bool {
    last_sentence_word(line, 0) == Some("조합한다")
        && line
            .tokens
            .iter()
            .any(|token| matches!(token.kind, TokenKind::StringLiteral(_)))
        && line
            .tokens
            .iter()
            .any(|token| matches!(&token.kind, TokenKind::Word(word) if matches!(word.as_str(), "실행될" | "발생할")))
}

fn is_relation_producer_sentence(line: &Line) -> bool {
    last_sentence_word(line, 0) == Some("연결한다")
        && line
            .tokens
            .iter()
            .any(|token| matches!(&token.kind, TokenKind::Word(word) if matches!(word.as_str(), "실행될" | "발생할")))
}

fn is_relation_sentence(line: &Line) -> bool {
    is_binary_relation_sentence(line) || is_unary_relation_sentence(line)
}

fn is_binary_relation_sentence(line: &Line) -> bool {
    let Some(id_index) = line
        .tokens
        .iter()
        .position(|token| matches!(token.kind, TokenKind::CanonicalId(_)))
    else {
        return false;
    };
    matches!(word_at(line, id_index + 1), Some("로" | "으로"))
        && sentence_words_end_with(line, &["가질", "수", "있다"])
}

fn is_unary_relation_sentence(line: &Line) -> bool {
    let Some(id_index) = line
        .tokens
        .iter()
        .position(|token| matches!(token.kind, TokenKind::CanonicalId(_)))
    else {
        return false;
    };
    word_at(line, id_index + 1) == Some("에")
        && sentence_words_end_with(line, &["해당할", "수", "있다"])
}

fn is_nonempty_sentence(line: &Line) -> bool {
    sentence_words_end_with(line, &["하나", "이상", "존재해야", "한다"])
}

fn is_required_relation_sentence(line: &Line) -> bool {
    word_at(line, 0) == Some("모든")
        && sentence_words_end_with(line, &["하나", "이상", "가져야", "한다"])
}

fn is_unique_relation_sentence(line: &Line) -> bool {
    word_at(line, 0) == Some("각")
        && sentence_words_end_with(line, &["최대", "하나만", "가질", "수", "있다"])
}

fn is_exclusive_relation_sentence(line: &Line) -> bool {
    sentence_words_end_with(
        line,
        &["중", "둘", "이상은", "동시에", "성립할", "수", "없다"],
    )
}

fn is_exhaustive_relation_sentence(line: &Line) -> bool {
    sentence_words_end_with(line, &["중", "하나", "이상은", "항상", "성립해야", "한다"])
}

fn is_coexistent_relation_sentence(line: &Line) -> bool {
    sentence_words_end_with(line, &["동시에", "성립할", "수", "있다"])
        && !is_binary_relation_sentence(line)
}

fn sentence_words_end_with(line: &Line, suffix: &[&str]) -> bool {
    let words = line
        .tokens
        .iter()
        .filter_map(|token| match &token.kind {
            TokenKind::Word(word) => Some(word.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    words.ends_with(suffix)
}

fn is_sum_derivation_sentence(line: &Line) -> bool {
    last_sentence_word(line, 0) == Some("계산한다")
        && line
            .tokens
            .iter()
            .any(|token| matches!(&token.kind, TokenKind::Word(word) if word == "합계로"))
}

fn is_recalculation_sentence(line: &Line) -> bool {
    last_sentence_word(line, 0) == Some("계산한다") && last_sentence_word(line, 1) == Some("다시")
}

fn is_field_intent_sentence(line: &Line) -> bool {
    let words = line
        .tokens
        .iter()
        .filter_map(|token| match &token.kind {
            TokenKind::Word(word) => Some(word.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    words.ends_with(&["내부", "관리에만", "사용한다"])
        || words.ends_with(&["사용자", "화면에서", "조회하지", "않는다"])
}

fn is_data_model_header(line: &Line) -> bool {
    if word_at(line, 0).is_some_and(|word| word.starts_with('@')) {
        return false;
    }
    let words = line
        .tokens
        .iter()
        .filter_map(|token| match &token.kind {
            TokenKind::Word(word) => Some(word.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    words.ends_with(&["다음", "필드들로", "구성되어", "있다"])
}

fn is_rule_sentence(line: &Line) -> bool {
    matches!(last_sentence_word(line, 0), Some("한다" | "있다" | "없다"))
}

fn is_policy_sentence(line: &Line) -> bool {
    matches!(last_sentence_word(line, 0), Some("있다" | "없다"))
        && last_sentence_word(line, 1) == Some("수")
}

fn parse_module(line: &Line, _diagnostics: &mut Vec<Diagnostic>) -> Result<ModuleAst, Diagnostic> {
    if line.indent != 0 {
        return Err(Diagnostic::error(
            "RSPDL-KO-SYN-001",
            "ko.syntax.module_header_required",
            line.span,
        ));
    }
    let declaration = parse_annotated_name(line, "@모듈")?;
    Ok(ModuleAst {
        declaration,
        span: line.span,
    })
}

fn parse_enum(
    line: &Line,
    body: &[Line],
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<EnumAst, Diagnostic> {
    let declaration = parse_natural_header(line, &["다음", "값", "중", "하나다"], diagnostics)?;
    ensure_body(body, line.span)?;
    let mut values = Vec::new();
    for item in body {
        if item.indent != 1 {
            return Err(bad_block_indent(item));
        }
        if item
            .tokens
            .iter()
            .any(|token| matches!(token.kind, TokenKind::Period))
        {
            return Err(Diagnostic::error(
                "RSPDL-KO-SYN-012",
                "ko.syntax.item_period_forbidden",
                item.span,
            ));
        }
        let id_index = item.tokens.len().checked_sub(1).ok_or_else(|| {
            Diagnostic::error(
                "RSPDL-KO-SYN-005",
                "ko.syntax.enum_value_required",
                item.span,
            )
        })?;
        let declaration = parse_cfg_item_name(item, id_index)?;
        values.push(EnumValueAst {
            declaration,
            span: item.span,
        });
    }
    let span = line
        .span
        .join(body.last().expect("validated enum body").span);
    Ok(EnumAst {
        declaration,
        values,
        span,
    })
}

fn parse_model(
    line: &Line,
    body: &[Line],
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<DataModelAst, Diagnostic> {
    let declaration =
        parse_natural_header(line, &["다음", "필드들로", "구성되어", "있다"], diagnostics)?;
    ensure_body(body, line.span)?;
    let mut fields = Vec::new();
    for item in body {
        if item.indent != 1 {
            return Err(bad_block_indent(item));
        }
        let colon_index = item
            .tokens
            .iter()
            .position(|token| matches!(token.kind, TokenKind::Colon))
            .ok_or_else(|| {
                Diagnostic::error(
                    "RSPDL-KO-SYN-010",
                    "ko.syntax.field_colon_required",
                    item.span,
                )
            })?;
        let id_index = colon_index.checked_sub(1).ok_or_else(|| {
            Diagnostic::error(
                "RSPDL-KO-SYN-010",
                "ko.syntax.field_shape_required",
                item.span,
            )
        })?;
        let field = parse_cfg_item_name(item, id_index)?;
        if item
            .tokens
            .iter()
            .any(|token| matches!(token.kind, TokenKind::Period))
        {
            return Err(Diagnostic::error(
                "RSPDL-KO-SYN-012",
                "ko.syntax.item_period_forbidden",
                item.span,
            ));
        }
        let required = match word_at(item, colon_index + 1) {
            Some("필수") => true,
            Some("선택") => false,
            _ => {
                return Err(Diagnostic::error(
                    "RSPDL-KO-SYN-010",
                    "ko.syntax.field_requiredness_required",
                    item.span,
                ));
            }
        };
        let value_type = parse_type_reference(item, colon_index + 2)?;
        fields.push(FieldAst {
            declaration: field,
            required,
            value_type,
            span: item.span,
        });
    }
    let span = line
        .span
        .join(body.last().expect("validated data model body").span);
    Ok(DataModelAst {
        declaration,
        fields,
        span,
    })
}

fn parse_screen(
    line: &Line,
    body: &[Line],
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<ScreenAst, Diagnostic> {
    if !body.is_empty() {
        return Err(Diagnostic::error(
            "RSPDL-KO-SYN-060",
            "ko.syntax.screen_must_be_sentence",
            line.span,
        ));
    }
    let tokens = sentence_tokens(line)?;
    let id_index = tokens
        .iter()
        .position(|token| matches!(token.kind, TokenKind::CanonicalId(_)))
        .ok_or_else(|| {
            Diagnostic::error(
                "RSPDL-KO-SYN-061",
                "ko.syntax.screen_stable_id_required",
                line.span,
            )
        })?;
    let declaration = parse_name_with_id(line, 0, id_index)?;
    let mut cursor = BodyCursor::new(&tokens[id_index + 1..], line.span);
    cursor.expect_word("에서는")?;
    let (model, marker) = cursor.marked_ref(&["의", "을", "를"])?;

    let (fields, operation) = if marker == "의" {
        if cursor.tokens.len().saturating_sub(cursor.index) < 3 {
            return Err(cursor.error("ko.syntax.screen_field_operation_required"));
        }
        let operation_index = cursor.tokens.len() - 3;
        let fields = parse_field_list(&cursor.tokens[cursor.index..operation_index], line.span)?;
        cursor.index = operation_index;
        let operation = match cursor.next_word() {
            Some("조회할") => ScreenOperationKindAst::Read,
            Some("입력할") => ScreenOperationKindAst::Input,
            Some("수정할") => ScreenOperationKindAst::Update,
            _ => {
                return Err(cursor.error("ko.syntax.screen_field_operation_invalid"));
            }
        };
        (fields, operation)
    } else {
        let operation = match cursor.next_word() {
            Some("생성할") => ScreenOperationKindAst::Create,
            Some("조회할") => ScreenOperationKindAst::Read,
            Some("수정할") => ScreenOperationKindAst::Update,
            Some("삭제할") => ScreenOperationKindAst::Delete,
            _ => {
                return Err(cursor.error("ko.syntax.screen_model_operation_invalid"));
            }
        };
        (Vec::new(), operation)
    };
    cursor.expect_word("수")?;
    cursor.expect_word("있다")?;
    cursor.expect_end()?;
    if marker != "의" {
        lint_marker(&model, &marker, "을", "를", line.span, diagnostics);
    }
    Ok(ScreenAst {
        declaration,
        model,
        fields,
        operation,
        span: line.span,
    })
}

fn parse_action_data_mutation(
    line: &Line,
    body: &[Line],
) -> Result<ActionDataMutationAst, Diagnostic> {
    reject_sentence_body(body, line.span, "action_data_mutation")?;
    let mut cursor = BodyCursor::new(sentence_tokens(line)?, line.span);
    let (action, _) = cursor.marked_ref(&["이", "가"])?;
    cursor.expect_word("실행되면")?;
    let (model, _) = cursor.marked_ref(&["을", "를"])?;
    let mutation = match cursor.next_word() {
        Some("생성한다") => DataMutationKindAst::Create,
        Some("수정한다") => DataMutationKindAst::Update,
        Some("삭제한다") => DataMutationKindAst::Delete,
        _ => return Err(cursor.error("ko.syntax.action_data_mutation_invalid")),
    };
    cursor.expect_end()?;
    Ok(ActionDataMutationAst {
        action,
        model,
        mutation,
        span: line.span,
    })
}

fn parse_action_input(
    line: &Line,
    body: &[Line],
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<ActionInputAst, Diagnostic> {
    parse_action_input_with_verb(line, body, diagnostics, "입력받는다", "action_input")
}

fn parse_action_input_with_verb(
    line: &Line,
    body: &[Line],
    diagnostics: &mut Vec<Diagnostic>,
    verb: &str,
    syntax_kind: &str,
) -> Result<ActionInputAst, Diagnostic> {
    reject_sentence_body(body, line.span, syntax_kind)?;
    let tokens = sentence_tokens(line)?;
    let mut cursor = BodyCursor::new(tokens, line.span);
    let (action, action_marker) = cursor.marked_ref(&["은", "는"])?;
    let is_existing_model = cursor.consume_word("기존");
    let (input_type_name, input_type_marker) = cursor.marked_ref(&["을", "를"])?;
    let kind = if is_existing_model {
        ActionInputKindAst::ExistingModel {
            model: input_type_name.clone(),
        }
    } else {
        ActionInputKindAst::Value {
            value_type: type_reference_from_name(&input_type_name, line.span)?,
        }
    };
    let id_index = tokens
        .iter()
        .enumerate()
        .skip(cursor.index)
        .find_map(|(index, token)| matches!(token.kind, TokenKind::CanonicalId(_)).then_some(index))
        .ok_or_else(|| {
            Diagnostic::error(
                "RSPDL-KO-SYN-064",
                "ko.syntax.action_input_stable_id_required",
                line.span,
            )
        })?;
    let declaration = parse_name_with_id_tokens(tokens, cursor.index, id_index, line.span)?;
    cursor.index = id_index + 1;
    match cursor.next_word() {
        Some("로" | "으로") => {}
        _ => return Err(cursor.error("ko.syntax.action_input_name_marker_required")),
    }
    cursor.expect_word(verb)?;
    cursor.expect_end()?;
    lint_marker(&action, &action_marker, "은", "는", line.span, diagnostics);
    lint_marker(
        &input_type_name,
        &input_type_marker,
        "을",
        "를",
        line.span,
        diagnostics,
    );
    Ok(ActionInputAst {
        action,
        declaration,
        kind,
        span: line.span,
    })
}

fn parse_event_input(
    line: &Line,
    body: &[Line],
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<EventInputAst, Diagnostic> {
    let input = parse_action_input_with_verb(line, body, diagnostics, "담는다", "event_input")?;
    Ok(EventInputAst {
        event: input.action,
        declaration: input.declaration,
        kind: input.kind,
        span: input.span,
    })
}

fn parse_creation_branch(
    line: &Line,
    body: &[Line],
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<CreationBranchAst, Diagnostic> {
    reject_sentence_body(body, line.span, "creation_branch")?;
    let tokens = sentence_tokens(line)?;
    let id_index = tokens
        .iter()
        .position(|token| matches!(token.kind, TokenKind::CanonicalId(_)))
        .ok_or_else(|| {
            Diagnostic::error(
                "RSPDL-KO-SYN-072",
                "ko.syntax.creation_branch_stable_id_required",
                line.span,
            )
        })?;
    let declaration = parse_name_with_id_tokens(tokens, 0, id_index, line.span)?;
    let mut cursor = BodyCursor::new(&tokens[id_index + 1..], line.span);
    let topic_marker = cursor
        .next_word()
        .filter(|marker| matches!(*marker, "은" | "는"));
    let Some(topic_marker) = topic_marker else {
        return Err(creation_branch_error(
            &cursor,
            "RSPDL-KO-SYN-073",
            "ko.syntax.creation_branch_topic_marker_required",
        ));
    };
    let (action, _) = creation_branch_marked_ref(
        &mut cursor,
        &["의"],
        "ko.syntax.creation_branch_condition_marker_invalid",
    )?;
    let (input, input_marker) = creation_branch_marked_ref(
        &mut cursor,
        &["이", "가"],
        "ko.syntax.creation_branch_condition_marker_invalid",
    )?;
    let (variant, variant_marker) = creation_branch_marked_ref(
        &mut cursor,
        &["이면", "라면"],
        "ko.syntax.creation_branch_condition_marker_invalid",
    )?;
    let (output_model, output_marker) = creation_branch_marked_ref(
        &mut cursor,
        &["을", "를"],
        "ko.syntax.creation_branch_result_marker_invalid",
    )?;
    let decision = match (cursor.next_word(), cursor.next_word()) {
        (Some("하나"), Some("생성한다")) => CreationDecisionAst::Create,
        (Some("생성하지"), Some("않는다")) => CreationDecisionAst::Skip,
        _ => {
            return Err(creation_branch_error(
                &cursor,
                "RSPDL-KO-SYN-076",
                "ko.syntax.creation_branch_result_invalid",
            ));
        }
    };
    cursor.expect_end()?;
    lint_marker(
        &declaration.name,
        topic_marker,
        "은",
        "는",
        line.span,
        diagnostics,
    );
    lint_marker(&input, &input_marker, "이", "가", line.span, diagnostics);
    lint_marker(
        &variant,
        &variant_marker,
        "이면",
        "라면",
        line.span,
        diagnostics,
    );
    lint_marker(
        &output_model,
        &output_marker,
        "을",
        "를",
        line.span,
        diagnostics,
    );
    Ok(CreationBranchAst {
        declaration,
        action,
        input,
        variant,
        output_model,
        decision,
        span: line.span,
    })
}

fn parse_field_producer(
    line: &Line,
    body: &[Line],
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<FieldProducerAst, Diagnostic> {
    reject_sentence_body(body, line.span, "field_producer")?;
    let tokens = sentence_tokens(line)?;
    let id_index = tokens
        .iter()
        .position(|token| matches!(token.kind, TokenKind::CanonicalId(_)))
        .ok_or_else(|| {
            Diagnostic::error(
                "RSPDL-KO-SYN-077",
                "ko.syntax.field_producer_stable_id_required",
                line.span,
            )
        })?;
    let declaration = parse_name_with_id_tokens(tokens, 0, id_index, line.span)?;
    let mut cursor = BodyCursor::new(&tokens[id_index + 1..], line.span);
    let topic = cursor
        .next_word()
        .filter(|marker| matches!(*marker, "은" | "는"))
        .ok_or_else(|| cursor.error("ko.syntax.field_producer_topic_marker_required"))?;
    let (trigger_name, trigger_marker) = cursor.marked_ref(&["의", "이", "가"])?;
    let (trigger_kind, condition) = if trigger_marker == "의" {
        let (input, _input_marker) = cursor.marked_ref(&["이", "가"])?;
        let (variant, _variant_marker) = cursor.marked_ref(&["이면", "라면"])?;
        (
            ProducerTriggerKindAst::Action,
            Some(FieldProducerConditionAst { input, variant }),
        )
    } else {
        let trigger_kind = if cursor.consume_word("실행될") {
            ProducerTriggerKindAst::Action
        } else {
            cursor.expect_word("발생할")?;
            ProducerTriggerKindAst::Event
        };
        cursor.expect_word("때")?;
        (trigger_kind, None)
    };
    let source = if is_template_producer_sentence(line) {
        let token = cursor
            .next_token()
            .ok_or_else(|| cursor.error("ko.syntax.template_string_required"))?;
        let TokenKind::StringLiteral(value) = &token.kind else {
            return Err(cursor.error("ko.syntax.template_string_required"));
        };
        cursor.expect_word("를")?;
        validate_template_syntax(value, token.span)?;
        FieldProducerSourceAst::Template {
            value: value.clone(),
        }
    } else if cursor.consume_word("상수") {
        let (literal, _marker) = cursor.field_producer_literal_marked()?;
        FieldProducerSourceAst::Constant { literal }
    } else {
        let (first, marker) = cursor.marked_ref(&["의", "을", "를"])?;
        if marker == "의" {
            let (field, field_marker) = cursor.marked_ref(&["을", "를"])?;
            lint_marker(&field, &field_marker, "을", "를", line.span, diagnostics);
            FieldProducerSourceAst::InputField {
                input: first,
                field,
            }
        } else {
            lint_marker(&first, &marker, "을", "를", line.span, diagnostics);
            FieldProducerSourceAst::ActionInput { input: first }
        }
    };
    let (output_model, _) = cursor.marked_ref(&["의"])?;
    let (output_field, output_marker) = cursor.marked_ref(&["으로", "로"])?;
    if matches!(source, FieldProducerSourceAst::Template { .. }) {
        cursor.expect_word("조합한다")?;
    } else {
        cursor.expect_word("기록한다")?;
    }
    cursor.expect_end()?;
    lint_marker(&declaration.name, topic, "은", "는", line.span, diagnostics);
    if trigger_marker != "의" {
        lint_marker(
            &trigger_name,
            &trigger_marker,
            "이",
            "가",
            line.span,
            diagnostics,
        );
    }
    lint_marker(
        &output_field,
        &output_marker,
        "으로",
        "로",
        line.span,
        diagnostics,
    );
    Ok(FieldProducerAst {
        declaration,
        trigger: ProducerTriggerAst {
            name: trigger_name,
            kind: trigger_kind,
        },
        output_model,
        output_field,
        source,
        condition,
        span: line.span,
    })
}

fn validate_template_syntax(value: &str, span: Span) -> Result<(), Diagnostic> {
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '{' if chars.peek() == Some(&'{') => {
                chars.next();
            }
            '}' if chars.peek() == Some(&'}') => {
                chars.next();
            }
            '{' => {
                let mut placeholder = String::new();
                let mut closed = false;
                for next in chars.by_ref() {
                    if next == '{' {
                        return Err(Diagnostic::error(
                            "RSPDL-KO-SYN-079",
                            "ko.syntax.template_nested_placeholder",
                            span,
                        ));
                    }
                    if next == '}' {
                        closed = true;
                        break;
                    }
                    placeholder.push(next);
                }
                if !closed {
                    return Err(Diagnostic::error(
                        "RSPDL-KO-SYN-079",
                        "ko.syntax.template_unmatched_brace",
                        span,
                    ));
                }
                if placeholder.trim().is_empty() {
                    return Err(Diagnostic::error(
                        "RSPDL-KO-SYN-079",
                        "ko.syntax.template_empty_placeholder",
                        span,
                    ));
                }
                if placeholder != placeholder.trim()
                    || placeholder.contains('.')
                    || placeholder.contains('/')
                    || placeholder.contains("::")
                {
                    return Err(Diagnostic::error(
                        "RSPDL-KO-SYN-079",
                        "ko.syntax.template_path_placeholder_forbidden",
                        span,
                    )
                    .with_argument("placeholder", placeholder));
                }
            }
            '}' => {
                return Err(Diagnostic::error(
                    "RSPDL-KO-SYN-079",
                    "ko.syntax.template_unmatched_brace",
                    span,
                ));
            }
            _ => {}
        }
    }
    Ok(())
}

fn parse_relation_producer(
    line: &Line,
    body: &[Line],
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<RelationProducerAst, Diagnostic> {
    reject_sentence_body(body, line.span, "relation_producer")?;
    let tokens = sentence_tokens(line)?;
    let id_index = tokens
        .iter()
        .position(|token| matches!(token.kind, TokenKind::CanonicalId(_)))
        .ok_or_else(|| {
            Diagnostic::error(
                "RSPDL-KO-SYN-078",
                "ko.syntax.relation_producer_stable_id_required",
                line.span,
            )
        })?;
    let declaration = parse_name_with_id_tokens(tokens, 0, id_index, line.span)?;
    let mut cursor = BodyCursor::new(&tokens[id_index + 1..], line.span);
    let topic = cursor
        .next_word()
        .filter(|marker| matches!(*marker, "은" | "는"))
        .ok_or_else(|| cursor.error("ko.syntax.relation_producer_topic_marker_required"))?;
    let (trigger_name, trigger_marker) = cursor.marked_ref(&["이", "가"])?;
    let trigger_kind = if cursor.consume_word("실행될") {
        ProducerTriggerKindAst::Action
    } else {
        cursor.expect_word("발생할")?;
        ProducerTriggerKindAst::Event
    };
    cursor.expect_word("때")?;
    let (input, input_marker) = cursor.marked_ref(&["을", "를"])?;
    let (output_model, _) = cursor.marked_ref(&["의"])?;
    let (relation, relation_marker) = cursor.marked_ref(&["으로", "로"])?;
    cursor.expect_word("연결한다")?;
    cursor.expect_end()?;
    lint_marker(&declaration.name, topic, "은", "는", line.span, diagnostics);
    lint_marker(
        &trigger_name,
        &trigger_marker,
        "이",
        "가",
        line.span,
        diagnostics,
    );
    lint_marker(&input, &input_marker, "을", "를", line.span, diagnostics);
    lint_marker(
        &relation,
        &relation_marker,
        "으로",
        "로",
        line.span,
        diagnostics,
    );
    Ok(RelationProducerAst {
        declaration,
        trigger: ProducerTriggerAst {
            name: trigger_name,
            kind: trigger_kind,
        },
        input,
        output_model,
        relation,
        span: line.span,
    })
}

fn creation_branch_marked_ref(
    cursor: &mut BodyCursor<'_>,
    markers: &[&str],
    message_key: &'static str,
) -> Result<(String, String), Diagnostic> {
    cursor
        .marked_ref(markers)
        .map_err(|_| creation_branch_error(cursor, "RSPDL-KO-SYN-074", message_key))
}

fn creation_branch_error(
    cursor: &BodyCursor<'_>,
    rule_id: &'static str,
    message_key: &'static str,
) -> Diagnostic {
    let span = cursor
        .tokens
        .get(cursor.index)
        .map(|token| token.span)
        .unwrap_or(cursor.span);
    Diagnostic::error(rule_id, message_key, span)
}

fn parse_field_list(tokens: &[Token], span: Span) -> Result<Vec<String>, Diagnostic> {
    if tokens.is_empty() {
        return Err(Diagnostic::error(
            "RSPDL-KO-SYN-062",
            "ko.syntax.field_list_required",
            span,
        ));
    }
    let mut fields = Vec::new();
    let mut start = 0usize;
    for end in (0..=tokens.len())
        .filter(|index| *index == tokens.len() || matches!(tokens[*index].kind, TokenKind::Comma))
    {
        let segment = &tokens[start..end];
        if segment.is_empty() {
            return Err(Diagnostic::error(
                "RSPDL-KO-SYN-062",
                "ko.syntax.field_list_empty_name",
                span,
            ));
        }
        let is_last = end == tokens.len();
        let mut parts = segment
            .iter()
            .map(|token| match &token.kind {
                TokenKind::Word(word) | TokenKind::QuotedIdentifier(word) => Ok(word.clone()),
                _ => Err(Diagnostic::error(
                    "RSPDL-KO-SYN-062",
                    "ko.syntax.field_list_invalid",
                    token.span,
                )),
            })
            .collect::<Result<Vec<_>, _>>()?;
        if is_last {
            let marker_follows_quoted_identifier = parts.len() > 1
                && matches!(parts.last().map(String::as_str), Some("을" | "를"))
                && matches!(
                    segment.get(segment.len() - 2).map(|token| &token.kind),
                    Some(TokenKind::QuotedIdentifier(_))
                );
            if marker_follows_quoted_identifier {
                parts.pop();
            } else {
                let last = parts.last_mut().expect("segment is not empty");
                let stripped = ["을", "를"]
                    .iter()
                    .find_map(|marker| last.strip_suffix(marker).filter(|value| !value.is_empty()));
                let Some(stripped) = stripped else {
                    return Err(Diagnostic::error(
                        "RSPDL-KO-SYN-062",
                        "ko.syntax.field_list_final_marker_required",
                        span,
                    ));
                };
                *last = stripped.to_owned();
            }
        }
        fields.push(parts.join(" "));
        start = end + 1;
    }
    Ok(fields)
}

fn parse_sum_derivation(line: &Line, body: &[Line]) -> Result<SumDerivationAst, Diagnostic> {
    reject_sentence_body(body, line.span, "sum_derivation")?;
    let mut cursor = BodyCursor::new(sentence_tokens(line)?, line.span);
    let (target_model, _) = cursor.marked_ref(&["의"])?;
    let (target_field, _) = cursor.marked_ref(&["은", "는"])?;
    let (source_model, _) = cursor.marked_ref(&["의"])?;
    let (source_field, _) = cursor.marked_ref(&["의"])?;
    cursor.expect_word("합계로")?;
    cursor.expect_word("계산한다")?;
    cursor.expect_end()?;
    Ok(SumDerivationAst {
        target_model,
        target_field,
        source_model,
        source_field,
        span: line.span,
    })
}

fn parse_recalculation(line: &Line, body: &[Line]) -> Result<RecalculationAst, Diagnostic> {
    reject_sentence_body(body, line.span, "recalculation")?;
    let mut cursor = BodyCursor::new(sentence_tokens(line)?, line.span);
    let (source_model, _) = cursor.marked_ref(&["의"])?;
    let (source_field, _) = cursor.marked_ref(&["이", "가"])?;
    cursor.expect_word("바뀔")?;
    cursor.expect_word("때")?;
    let (target_model, _) = cursor.marked_ref(&["의"])?;
    let (target_field, _) = cursor.marked_ref(&["을", "를"])?;
    cursor.expect_word("다시")?;
    cursor.expect_word("계산한다")?;
    cursor.expect_end()?;
    Ok(RecalculationAst {
        source_model,
        source_field,
        target_model,
        target_field,
        span: line.span,
    })
}

fn parse_field_intent(line: &Line, body: &[Line]) -> Result<FieldIntentAst, Diagnostic> {
    reject_sentence_body(body, line.span, "field_intent")?;
    let mut cursor = BodyCursor::new(sentence_tokens(line)?, line.span);
    let (model, _) = cursor.marked_ref(&["의"])?;
    let (field, _) = cursor.marked_ref(&["은", "는"])?;
    let intent = match cursor.next_word() {
        Some("내부") => {
            cursor.expect_word("관리에만")?;
            cursor.expect_word("사용한다")?;
            FieldIntentKindAst::Internal
        }
        Some("사용자") => {
            cursor.expect_word("화면에서")?;
            cursor.expect_word("조회하지")?;
            cursor.expect_word("않는다")?;
            FieldIntentKindAst::Hidden
        }
        _ => return Err(cursor.error("ko.syntax.field_intent_invalid")),
    };
    cursor.expect_end()?;
    Ok(FieldIntentAst {
        model,
        field,
        intent,
        span: line.span,
    })
}

fn reject_sentence_body(body: &[Line], span: Span, kind: &str) -> Result<(), Diagnostic> {
    if body.is_empty() {
        Ok(())
    } else {
        Err(Diagnostic::error(
            "RSPDL-KO-SYN-063",
            "ko.syntax.sentence_block_forbidden",
            span,
        )
        .with_argument("kind", kind))
    }
}

fn parse_constraint(
    line: &Line,
    body: &[Line],
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<ConstraintAst, Diagnostic> {
    if !body.is_empty() {
        return Err(Diagnostic::error(
            "RSPDL-KO-SYN-020",
            "ko.syntax.constraint_block_forbidden",
            line.span,
        ));
    }
    let expression = parse_constraint_sentence(line, diagnostics)?;
    let declaration = anonymous_declaration(line.span);
    Ok(ConstraintAst {
        declaration,
        expression,
    })
}

fn parse_policy(
    line: &Line,
    body: &[Line],
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<PolicyAst, Diagnostic> {
    if !body.is_empty() {
        return Err(Diagnostic::error(
            "RSPDL-KO-SYN-030",
            "ko.syntax.policy_block_forbidden",
            line.span,
        ));
    }
    let body_line = line;
    let tokens = sentence_tokens(body_line)?;
    let mut cursor = BodyCursor::new(tokens, body_line.span);
    let (role, role_marker) = cursor.marked_ref(&["은", "는"])?;
    let (model, _) = cursor.marked_ref(&["의"])?;
    let (field, field_marker) = cursor.marked_ref(&["을", "를"])?;
    let (action, _) = cursor.marked_ref(&["할"])?;
    cursor.expect_word("수")?;
    let effect = match cursor.next_word() {
        Some("있다") => PolicyEffectAst::Allow,
        Some("없다") => PolicyEffectAst::Deny,
        _ => return Err(cursor.error("ko.syntax.policy_effect_invalid")),
    };
    cursor.expect_end()?;
    lint_marker(&role, &role_marker, "은", "는", body_line.span, diagnostics);
    lint_marker(
        &field,
        &field_marker,
        "을",
        "를",
        body_line.span,
        diagnostics,
    );
    let declaration = anonymous_declaration(body_line.span);
    Ok(PolicyAst {
        declaration,
        role,
        model,
        field,
        action,
        effect,
        span: body_line.span,
    })
}

fn parse_constraint_sentence(
    line: &Line,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<ConstraintExpressionAst, Diagnostic> {
    let tokens = sentence_tokens(line)?;
    let mut cursor = BodyCursor::new(tokens, line.span);
    let (model, _) = cursor.marked_ref(&["의"])?;
    let (left, marker) = cursor.marked_ref(&["와", "과", "은", "는"])?;

    if marker == "와" || marker == "과" {
        let (right, right_marker) = cursor.marked_ref(&["은", "는"])?;
        let operator = match cursor.next_word() {
            Some("같아야") => RelationOperatorAst::Equal,
            Some("달라야") => RelationOperatorAst::NotEqual,
            _ => return Err(cursor.error("ko.syntax.field_comparison_invalid")),
        };
        cursor.expect_word("한다")?;
        cursor.expect_end()?;
        lint_marker(&left, &marker, "과", "와", line.span, diagnostics);
        lint_marker(&right, &right_marker, "은", "는", line.span, diagnostics);
        return Ok(ConstraintExpressionAst {
            model,
            left: OperandAst::Field(left),
            operator,
            right: OperandAst::Field(right),
            span: line.span,
        });
    }

    let (operator, literal) = cursor.comparison_literal()?;
    cursor.expect_word("한다")?;
    cursor.expect_end()?;
    lint_marker(&left, &marker, "은", "는", line.span, diagnostics);
    Ok(ConstraintExpressionAst {
        model,
        left: OperandAst::Field(left),
        operator,
        right: OperandAst::Literal(literal),
        span: line.span,
    })
}

fn anonymous_declaration(span: Span) -> NamedIdAst {
    NamedIdAst {
        name: String::new(),
        id: String::new(),
        span,
    }
}

fn sentence_tokens(line: &Line) -> Result<&[Token], Diagnostic> {
    if !matches!(
        line.tokens.last().map(|token| &token.kind),
        Some(TokenKind::Period)
    ) {
        return Err(Diagnostic::error(
            "RSPDL-KO-SYN-040",
            "ko.syntax.period_required",
            line.span,
        ));
    }
    Ok(&line.tokens[..line.tokens.len() - 1])
}

fn parse_role(
    line: &Line,
    body: &[Line],
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<NamedIdAst, Diagnostic> {
    reject_sentence_body(body, line.span, "role")?;
    parse_natural_header(line, &["역할이다"], diagnostics)
}

fn parse_action(
    line: &Line,
    body: &[Line],
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<NamedIdAst, Diagnostic> {
    reject_sentence_body(body, line.span, "action")?;
    parse_natural_header(line, &["행동이다"], diagnostics)
}

fn parse_event(
    line: &Line,
    body: &[Line],
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<NamedIdAst, Diagnostic> {
    reject_sentence_body(body, line.span, "event")?;
    parse_natural_header(line, &["사건이다"], diagnostics)
}

fn parse_annotated_name(line: &Line, keyword: &str) -> Result<NamedIdAst, Diagnostic> {
    if word_at(line, 0) != Some(keyword) {
        return Err(Diagnostic::error(
            "RSPDL-KO-SYN-004",
            "ko.syntax.annotated_declaration_required",
            line.span,
        )
        .with_argument("keyword", keyword)
        .with_argument("id_kind", "stable_id"));
    }
    if line
        .tokens
        .iter()
        .any(|token| matches!(token.kind, TokenKind::Period | TokenKind::Colon))
    {
        return Err(Diagnostic::error(
            "RSPDL-KO-SYN-004",
            "ko.syntax.declaration_punctuation_forbidden",
            line.span,
        ));
    }
    let id_index = line.tokens.len().checked_sub(1).ok_or_else(|| {
        Diagnostic::error(
            "RSPDL-KO-SYN-006",
            "ko.syntax.declaration_id_required",
            line.span,
        )
    })?;
    parse_name_with_id(line, 1, id_index)
}

fn parse_natural_header(
    line: &Line,
    predicate: &[&str],
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<NamedIdAst, Diagnostic> {
    if !matches!(
        line.tokens.last().map(|token| &token.kind),
        Some(TokenKind::Period)
    ) {
        return Err(Diagnostic::error(
            "RSPDL-KO-SYN-004",
            "ko.syntax.natural_header_period_required",
            line.span,
        ));
    }
    let id_index = line
        .tokens
        .iter()
        .position(|token| matches!(token.kind, TokenKind::CanonicalId(_)))
        .ok_or_else(|| {
            Diagnostic::error(
                "RSPDL-KO-SYN-006",
                "ko.syntax.declaration_id_required",
                line.span,
            )
        })?;
    let declaration = parse_name_with_id(line, 0, id_index)?;
    let sentence = &line.tokens[id_index + 1..line.tokens.len() - 1];
    let mut cursor = BodyCursor::new(sentence, line.span);
    let marker = cursor
        .next_word()
        .filter(|marker| matches!(*marker, "은" | "는"))
        .ok_or_else(|| cursor.error("ko.syntax.declaration_topic_marker_required"))?;
    for expected in predicate {
        cursor.expect_word(expected)?;
    }
    cursor.expect_end()?;
    lint_marker(
        &declaration.name,
        marker,
        "은",
        "는",
        line.span,
        diagnostics,
    );
    Ok(declaration)
}

fn parse_cfg_item_name(line: &Line, id_index: usize) -> Result<NamedIdAst, Diagnostic> {
    parse_name_with_id(line, 0, id_index)
}

fn parse_name_with_id(
    line: &Line,
    name_start: usize,
    id_index: usize,
) -> Result<NamedIdAst, Diagnostic> {
    let id = canonical_id_at(line, id_index)?;
    let (name, name_span) = surface_name(line, name_start, id_index)?;
    Ok(NamedIdAst {
        name,
        id,
        span: name_span.join(line.tokens[id_index].span),
    })
}

fn parse_name_with_id_tokens(
    tokens: &[Token],
    name_start: usize,
    id_index: usize,
    span: Span,
) -> Result<NamedIdAst, Diagnostic> {
    let line = Line {
        indent: 0,
        tokens: tokens.to_vec(),
        span,
    };
    parse_name_with_id(&line, name_start, id_index)
}

fn canonical_id_at(line: &Line, index: usize) -> Result<String, Diagnostic> {
    match line.tokens.get(index).map(|token| &token.kind) {
        Some(TokenKind::CanonicalId(value)) if !value.is_empty() => Ok(value.clone()),
        _ => Err(Diagnostic::error(
            "RSPDL-KO-SYN-006",
            "ko.syntax.stable_id_required",
            line.span,
        )),
    }
}

fn surface_name(line: &Line, start: usize, end: usize) -> Result<(String, Span), Diagnostic> {
    if start >= end {
        return Err(Diagnostic::error(
            "RSPDL-KO-SYN-005",
            "ko.syntax.display_name_required",
            line.span,
        ));
    }
    let mut parts = Vec::new();
    for token in &line.tokens[start..end] {
        match &token.kind {
            TokenKind::Word(value) | TokenKind::QuotedIdentifier(value) => {
                parts.push(value.clone())
            }
            _ => {
                return Err(Diagnostic::error(
                    "RSPDL-KO-SYN-005",
                    "ko.syntax.display_name_invalid",
                    token.span,
                ));
            }
        }
    }
    let first = line.tokens[start].span;
    let last = line.tokens[end - 1].span;
    Ok((parts.join(" "), first.join(last)))
}

fn ensure_body(body: &[Line], span: Span) -> Result<(), Diagnostic> {
    if body.is_empty() {
        Err(Diagnostic::error(
            "RSPDL-KO-SYN-008",
            "ko.syntax.block_item_required",
            span,
        ))
    } else {
        Ok(())
    }
}

fn bad_block_indent(line: &Line) -> Diagnostic {
    Diagnostic::error(
        "RSPDL-KO-SYN-009",
        "ko.syntax.block_indent_inconsistent",
        line.span,
    )
}

fn word_at(line: &Line, index: usize) -> Option<&str> {
    match line.tokens.get(index).map(|token| &token.kind) {
        Some(TokenKind::Word(value)) => Some(value),
        _ => None,
    }
}

fn last_sentence_word(line: &Line, offset: usize) -> Option<&str> {
    let period = usize::from(matches!(
        line.tokens.last().map(|token| &token.kind),
        Some(TokenKind::Period)
    ));
    let index = line.tokens.len().checked_sub(1 + period + offset)?;
    word_at(line, index)
}

fn parse_type_reference(line: &Line, start: usize) -> Result<TypeReferenceAst, Diagnostic> {
    if start >= line.tokens.len() {
        return Err(Diagnostic::error(
            "RSPDL-KO-SYN-011",
            "ko.syntax.field_type_required",
            line.span,
        ));
    }
    let mut parts = Vec::new();
    for token in &line.tokens[start..] {
        match &token.kind {
            TokenKind::Word(value) | TokenKind::QuotedIdentifier(value) => {
                parts.push(value.clone())
            }
            _ => {
                return Err(Diagnostic::error(
                    "RSPDL-KO-SYN-011",
                    "ko.syntax.field_type_invalid",
                    token.span,
                ));
            }
        }
    }
    type_reference_from_name(&parts.join(" "), line.span)
}

fn type_reference_from_name(name: &str, span: Span) -> Result<TypeReferenceAst, Diagnostic> {
    match name {
        "문자열" => Ok(TypeReferenceAst::String),
        "정수" => Ok(TypeReferenceAst::Integer),
        "불리언" => Ok(TypeReferenceAst::Boolean),
        "" => Err(Diagnostic::error(
            "RSPDL-KO-SYN-011",
            "ko.syntax.field_type_required",
            span,
        )),
        _ => Ok(TypeReferenceAst::Named(name.to_owned())),
    }
}

struct BodyCursor<'a> {
    tokens: &'a [Token],
    index: usize,
    span: Span,
}

impl<'a> BodyCursor<'a> {
    fn new(tokens: &'a [Token], span: Span) -> Self {
        Self {
            tokens,
            index: 0,
            span,
        }
    }

    fn next_token(&mut self) -> Option<&'a Token> {
        let token = self.tokens.get(self.index)?;
        self.index += 1;
        Some(token)
    }

    fn marked_ref(&mut self, markers: &[&str]) -> Result<(String, String), Diagnostic> {
        let token = self
            .tokens
            .get(self.index)
            .ok_or_else(|| self.error("ko.syntax.reference_and_marker_required"))?;
        match &token.kind {
            TokenKind::QuotedIdentifier(value) => {
                self.index += 1;
                let marker = self
                    .next_word()
                    .ok_or_else(|| self.error("ko.syntax.quoted_reference_marker_required"))?;
                if markers.contains(&marker) {
                    Ok((value.clone(), marker.to_owned()))
                } else {
                    Err(self
                        .error("ko.syntax.reference_marker_invalid")
                        .with_argument("reference", value)
                        .with_argument("expected", markers.join("/")))
                }
            }
            TokenKind::Word(_) => {
                let mut parts = Vec::new();
                while let Some(TokenKind::Word(value)) =
                    self.tokens.get(self.index).map(|token| &token.kind)
                {
                    for marker in markers {
                        if let Some(base) =
                            value.strip_suffix(marker).filter(|base| !base.is_empty())
                        {
                            parts.push(base.to_owned());
                            self.index += 1;
                            return Ok((parts.join(" "), (*marker).to_owned()));
                        }
                    }
                    parts.push(value.clone());
                    self.index += 1;
                }
                Err(self
                    .error("ko.syntax.reference_marker_missing")
                    .with_argument("reference", parts.join(" "))
                    .with_argument("expected", markers.join("/")))
            }
            _ => Err(self.error("ko.syntax.surface_name_required")),
        }
    }

    fn comparison_literal(&mut self) -> Result<(RelationOperatorAst, LiteralAst), Diagnostic> {
        let token = self
            .tokens
            .get(self.index)
            .ok_or_else(|| self.error("ko.syntax.comparison_value_required"))?;
        if let TokenKind::StringLiteral(value) = &token.kind {
            let literal = LiteralAst::String(value.clone());
            if matches!(
                self.tokens.get(self.index + 1).map(|token| &token.kind),
                Some(TokenKind::Word(marker)) if matches!(marker.as_str(), "과" | "와")
            ) && matches!(
                self.tokens.get(self.index + 2).map(|token| &token.kind),
                Some(TokenKind::Word(operator)) if operator == "달라야"
            ) {
                self.index += 3;
                return Ok((RelationOperatorAst::NotEqual, literal));
            }
        }
        if let TokenKind::QuotedIdentifier(value) = &token.kind {
            let literal = LiteralAst::Named(value.clone());
            if matches!(
                self.tokens.get(self.index + 1).map(|token| &token.kind),
                Some(TokenKind::Word(marker)) if matches!(marker.as_str(), "과" | "와")
            ) && matches!(
                self.tokens.get(self.index + 2).map(|token| &token.kind),
                Some(TokenKind::Word(operator)) if operator == "달라야"
            ) {
                self.index += 3;
                return Ok((RelationOperatorAst::NotEqual, literal));
            }
        }
        if let TokenKind::Word(value) = &token.kind {
            for marker in ["과", "와"] {
                if let Some(literal) = value
                    .strip_suffix(marker)
                    .filter(|literal| !literal.is_empty())
                {
                    self.index += 1;
                    if self.next_word() == Some("달라야") {
                        return Ok((RelationOperatorAst::NotEqual, parse_word_literal(literal)));
                    }
                    return Err(self.error("ko.syntax.not_equal_shape_required"));
                }
            }
            if let Some(number) = value.strip_suffix("보다")
                && is_integer(number)
            {
                self.index += 1;
                let operator = match self.next_word() {
                    Some("커야") => RelationOperatorAst::GreaterThan,
                    Some("작아야") => RelationOperatorAst::LessThan,
                    _ => {
                        return Err(self.error("ko.syntax.integer_order_shape_required"));
                    }
                };
                return Ok((operator, LiteralAst::Integer(number.to_owned())));
            }
            if is_integer(value) {
                let number = value.clone();
                self.index += 1;
                let operator = match self.next_word() {
                    Some("이상이어야") => RelationOperatorAst::GreaterThanOrEqual,
                    Some("이하여야") => RelationOperatorAst::LessThanOrEqual,
                    Some("보다") => match self.next_word() {
                        Some("커야") => RelationOperatorAst::GreaterThan,
                        Some("작아야") => RelationOperatorAst::LessThan,
                        _ => return Err(self.error("ko.syntax.order_suffix_required")),
                    },
                    Some("이어야") => RelationOperatorAst::Equal,
                    _ => return Err(self.error("ko.syntax.integer_comparison_unsupported")),
                };
                return Ok((operator, LiteralAst::Integer(number)));
            }
            let mut parts = Vec::new();
            let mut literal_index = self.index;
            while let Some(TokenKind::Word(part)) =
                self.tokens.get(literal_index).map(|token| &token.kind)
            {
                if let Some(last) = part.strip_suffix("이어야").filter(|last| !last.is_empty()) {
                    parts.push(last.to_owned());
                    self.index = literal_index + 1;
                    return Ok((
                        RelationOperatorAst::Equal,
                        parse_word_literal(&parts.join(" ")),
                    ));
                }
                parts.push(part.clone());
                literal_index += 1;
            }
        }

        let literal = match &token.kind {
            TokenKind::StringLiteral(value) => LiteralAst::String(value.clone()),
            TokenKind::QuotedIdentifier(value) => LiteralAst::Named(value.clone()),
            TokenKind::Word(value) => parse_word_literal(value),
            _ => return Err(self.error("ko.syntax.literal_unsupported")),
        };
        self.index += 1;
        self.expect_word("이어야")?;
        Ok((RelationOperatorAst::Equal, literal))
    }

    fn field_producer_literal_marked(&mut self) -> Result<(LiteralAst, String), Diagnostic> {
        let token = self
            .tokens
            .get(self.index)
            .ok_or_else(|| self.error("ko.syntax.field_producer_literal_required"))?;
        let literal = match &token.kind {
            TokenKind::StringLiteral(value) => LiteralAst::String(value.clone()),
            TokenKind::QuotedIdentifier(value) => LiteralAst::Named(value.clone()),
            TokenKind::Word(value) => {
                for marker in ["을", "를"] {
                    if let Some(value) =
                        value.strip_suffix(marker).filter(|value| !value.is_empty())
                    {
                        self.index += 1;
                        return Ok((parse_word_literal(value), marker.into()));
                    }
                }
                let literal = parse_word_literal(value);
                self.index += 1;
                let marker = self
                    .next_word()
                    .filter(|marker| matches!(*marker, "을" | "를"))
                    .ok_or_else(|| {
                        self.error("ko.syntax.field_producer_literal_marker_required")
                    })?;
                return Ok((literal, marker.into()));
            }
            _ => return Err(self.error("ko.syntax.field_producer_literal_required")),
        };
        self.index += 1;
        let marker = self
            .next_word()
            .filter(|marker| matches!(*marker, "을" | "를"))
            .ok_or_else(|| self.error("ko.syntax.field_producer_literal_marker_required"))?;
        Ok((literal, marker.into()))
    }

    fn expect_word(&mut self, expected: &str) -> Result<(), Diagnostic> {
        match self.next_word() {
            Some(actual) if actual == expected => Ok(()),
            _ => Err(self
                .error("ko.syntax.word_required")
                .with_argument("expected", expected)),
        }
    }

    fn next_word(&mut self) -> Option<&'a str> {
        let value = match self.tokens.get(self.index).map(|token| &token.kind) {
            Some(TokenKind::Word(value)) => Some(value.as_str()),
            _ => None,
        };
        if value.is_some() {
            self.index += 1;
        }
        value
    }

    fn consume_word(&mut self, expected: &str) -> bool {
        if matches!(self.tokens.get(self.index).map(|token| &token.kind), Some(TokenKind::Word(value)) if value == expected)
        {
            self.index += 1;
            true
        } else {
            false
        }
    }

    fn expect_end(&self) -> Result<(), Diagnostic> {
        if self.index == self.tokens.len() {
            Ok(())
        } else {
            Err(self.error("ko.syntax.trailing_expression"))
        }
    }

    fn error(&self, message_key: &'static str) -> Diagnostic {
        let span = self
            .tokens
            .get(self.index)
            .map(|token| token.span)
            .unwrap_or(self.span);
        Diagnostic::error("RSPDL-KO-SYN-041", message_key, span)
    }
}

fn parse_word_literal(value: &str) -> LiteralAst {
    match value {
        "참" => LiteralAst::Boolean(true),
        "거짓" => LiteralAst::Boolean(false),
        _ if is_integer(value) => LiteralAst::Integer(value.to_owned()),
        _ => LiteralAst::Named(value.to_owned()),
    }
}

fn is_integer(value: &str) -> bool {
    let digits = value.strip_prefix('-').unwrap_or(value);
    !digits.is_empty()
        && digits.chars().all(|character| character.is_ascii_digit())
        && (digits == "0" || !digits.starts_with('0'))
        && value != "-0"
}

fn lint_marker(
    name: &str,
    actual: &str,
    consonant: &str,
    vowel: &str,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(last) = name
        .chars()
        .last()
        .filter(|character| ('가'..='힣').contains(character))
    else {
        return;
    };
    let expected = if (last as u32 - '가' as u32).is_multiple_of(28) {
        vowel
    } else {
        consonant
    };
    if actual != expected {
        diagnostics.push(
            Diagnostic::warning("RSPDL-KO-W001", "ko.lint.marker_preference", span)
                .with_argument("name", name)
                .with_argument("actual", actual)
                .with_argument("expected", expected),
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::Severity;

    use super::*;

    const SOURCE: &str = r#"@모듈 비용 승인(expense)

비용 상태(status)는 다음 값 중 하나다.
    작성 중(draft)
    승인됨(approved)

비용 신청(request)은 다음 필드들로 구성되어 있다.
    식별자(id): 필수 문자열
    금액(amount): 필수 정수
    상태(status): 필수 비용 상태

비용 신청의 금액은 0보다 커야 한다.

회계 관리자(accounting_manager)는 역할이다.
변경(change)은 행동이다.

회계 관리자는 비용 신청의 상태를 변경할 수 있다.
"#;

    #[test]
    fn parses_the_vertical_slice() {
        let output = parse(SOURCE);
        assert_eq!(output.diagnostics, Vec::<Diagnostic>::new());
        let document = output.document.unwrap();
        assert_eq!(document.declarations.len(), 6);
        let DeclarationAst::Constraint(constraint) = &document.declarations[2] else {
            panic!("third declaration should be a constraint sentence");
        };
        assert!(constraint.declaration.name.is_empty());
        assert!(constraint.declaration.id.is_empty());
        let DeclarationAst::Policy(policy) = &document.declarations[5] else {
            panic!("last declaration should be a policy sentence");
        };
        assert!(policy.declaration.name.is_empty());
        assert!(policy.declaration.id.is_empty());
    }

    #[test]
    fn preserves_utf8_byte_spans_for_blocks_and_source_backed_lines() {
        let output = parse(SOURCE);
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
        let document = output.document.unwrap();

        assert_eq!(
            SOURCE.get(document.module.span.start..document.module.span.end),
            Some("@모듈 비용 승인(expense)")
        );
        let DeclarationAst::DataModel(model) = &document.declarations[1] else {
            panic!("second declaration should be a data model");
        };
        assert_eq!(
            SOURCE.get(model.span.start..model.span.end),
            Some(
                "비용 신청(request)은 다음 필드들로 구성되어 있다.\n    식별자(id): 필수 문자열\n    금액(amount): 필수 정수\n    상태(status): 필수 비용 상태"
            )
        );
        assert_eq!(
            SOURCE.get(model.fields[1].span.start..model.fields[1].span.end),
            Some("금액(amount): 필수 정수")
        );
        let DeclarationAst::Role(role) = &document.declarations[3] else {
            panic!("fourth declaration should be a role");
        };
        assert_eq!(
            SOURCE.get(role.span.start..role.span.end),
            Some("회계 관리자(accounting_manager)는 역할이다.")
        );
    }

    #[test]
    fn anonymous_semantic_rules_do_not_allocate_locale_ids() {
        let alternate_labels = SOURCE
            .replace("비용 승인", "지출 승인")
            .replace("비용 상태", "처리 상태")
            .replace("비용 신청", "지출 요청")
            .replace("회계 관리자", "재무 담당자")
            .replace("금액", "합계")
            .replace("상태", "단계")
            .replace("변경", "수정");

        for source in [SOURCE, &alternate_labels] {
            let document = parse(source).document.unwrap();
            let DeclarationAst::Constraint(constraint) = &document.declarations[2] else {
                panic!("third declaration should be a constraint sentence");
            };
            let DeclarationAst::Policy(policy) = &document.declarations[5] else {
                panic!("last declaration should be a policy sentence");
            };
            assert!(constraint.declaration.id.is_empty());
            assert!(policy.declaration.id.is_empty());
        }
    }

    #[test]
    fn requires_sentence_periods() {
        let output = parse(&SOURCE.replace("커야 한다.", "커야 한다"));
        assert!(
            output
                .diagnostics
                .iter()
                .any(|diagnostic| { diagnostic.rule_id == "RSPDL-KO-SYN-040" })
        );
    }

    #[test]
    fn data_models_do_not_use_an_annotation() {
        let output = parse(&SOURCE.replace("비용 신청(request)은", "@데이터 비용 신청(request)은"));
        assert!(output.diagnostics.iter().any(Diagnostic::is_error));
    }

    #[test]
    fn parses_sentence_shaped_screen_and_sum_derivation_rules() {
        let source = r#"@모듈 장바구니(shopping)
장바구니(cart)는 다음 필드들로 구성되어 있다.
    결제 예정 금액(total): 필수 정수
장바구니 항목(item)은 다음 필드들로 구성되어 있다.
    수량(quantity): 필수 정수
    금액(amount): 필수 정수
장바구니 작성 화면(create_cart)에서는 장바구니를 생성할 수 있다.
장바구니 항목 입력 화면(create_item)에서는 장바구니 항목을 생성할 수 있다.
장바구니 항목 입력 화면(create_item)에서는 장바구니 항목의 수량, 금액을 입력할 수 있다.
장바구니 상세 화면(cart_detail)에서는 장바구니의 결제 예정 금액을 조회할 수 있다.
장바구니 항목 화면(item_detail)에서는 장바구니 항목의 수량, 금액을 조회할 수 있다.
장바구니 항목 수정 화면(update_item)에서는 장바구니 항목의 금액을 수정할 수 있다.
장바구니 항목 삭제 화면(delete_item)에서는 장바구니 항목을 삭제할 수 있다.
장바구니의 결제 예정 금액은 장바구니 항목의 금액의 합계로 계산한다.
장바구니 항목의 금액이 바뀔 때 장바구니의 결제 예정 금액을 다시 계산한다.
"#;
        let output = parse(source);
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
        let document = output.document.unwrap();
        assert!(document.declarations.iter().any(|declaration| matches!(
            declaration,
            DeclarationAst::Screen(ScreenAst { fields, .. }) if fields.len() == 2
        )));
        assert!(
            document
                .declarations
                .iter()
                .any(|declaration| matches!(declaration, DeclarationAst::SumDerivation(_)))
        );
        assert!(
            document
                .declarations
                .iter()
                .any(|declaration| matches!(declaration, DeclarationAst::Recalculation(_)))
        );
        assert!(document.declarations.iter().any(|declaration| matches!(
            declaration,
            DeclarationAst::Screen(ScreenAst {
                operation: ScreenOperationKindAst::Delete,
                ..
            })
        )));

        let quoted = parse(
            "@모듈 배송(delivery)\n배송 입력 화면(input)에서는 배송의 `배송 주소`, `수령인 이름`을 입력할 수 있다.\n",
        );
        assert!(quoted.diagnostics.is_empty(), "{:?}", quoted.diagnostics);
        assert!(
            quoted
                .document
                .unwrap()
                .declarations
                .iter()
                .any(|declaration| {
                    matches!(
                        declaration,
                        DeclarationAst::Screen(ScreenAst { fields, .. })
                            if fields == &["배송 주소", "수령인 이름"]
                    )
                })
        );
    }

    #[test]
    fn parses_action_data_mutation_sentences() {
        let source = r#"@모듈 주문(ordering)
주문(order)은 다음 필드들로 구성되어 있다.
    상태(status): 필수 문자열
주문 취소(cancel_order)는 행동이다.
주문 취소가 실행되면 주문을 수정한다.
"#;
        let output = parse(source);
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
        assert!(
            output
                .document
                .unwrap()
                .declarations
                .iter()
                .any(|declaration| {
                    matches!(
                        declaration,
                        DeclarationAst::ActionDataMutation(ActionDataMutationAst {
                            action,
                            model,
                            mutation: DataMutationKindAst::Update,
                            ..
                        }) if action == "주문 취소" && model == "주문"
                    )
                })
        );
    }

    #[test]
    fn parses_exact_conditional_creation_sentences_and_rejects_invalid_shapes() {
        let source = r#"@모듈 알림(notifications)
상태(status)는 다음 값 중 하나다.
    접수됨(received)
    보류됨(on_hold)
점검 요청 전달 알림(notice)은 다음 필드들로 구성되어 있다.
    내용(content): 선택 문자열
점검 요청 전달(assign_request)은 행동이다.
점검 요청 전달은 상태를 요청 상태(request_status)로 입력받는다.
접수 상태 알림 생성(received_notice_create)은 점검 요청 전달의 요청 상태가 접수됨이면 점검 요청 전달 알림을 하나 생성한다.
보류 상태 알림 미생성(on_hold_notice_skip)은 점검 요청 전달의 요청 상태가 보류됨이면 점검 요청 전달 알림을 생성하지 않는다.
"#;
        let output = parse(source);
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
        let branches = output
            .document
            .unwrap()
            .declarations
            .into_iter()
            .filter_map(|declaration| match declaration {
                DeclarationAst::CreationBranch(value) => Some(value),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(branches.len(), 2);
        assert_eq!(branches[0].declaration.id, "received_notice_create");
        assert_eq!(branches[0].action, "점검 요청 전달");
        assert_eq!(branches[0].input, "요청 상태");
        assert_eq!(branches[0].variant, "접수됨");
        assert_eq!(branches[0].output_model, "점검 요청 전달 알림");
        assert_eq!(branches[0].decision, CreationDecisionAst::Create);
        assert_eq!(branches[1].decision, CreationDecisionAst::Skip);

        for (line, rule_id, message_key) in [
            (
                "접수 상태 알림 생성은 점검 요청 전달의 요청 상태가 접수됨이면 점검 요청 전달 알림을 하나 생성한다.",
                "RSPDL-KO-SYN-072",
                "ko.syntax.creation_branch_stable_id_required",
            ),
            (
                "접수 상태 알림 생성(received_notice_create)은 점검 요청 전달의 요청 상태가 접수됨이면 점검 요청 전달 알림을 두 개 생성한다.",
                "RSPDL-KO-SYN-076",
                "ko.syntax.creation_branch_result_invalid",
            ),
        ] {
            let output = parse(&format!("@모듈 알림(notifications)\n{line}\n"));
            assert!(
                output.diagnostics.iter().any(|diagnostic| {
                    diagnostic.rule_id == rule_id && diagnostic.message_key == message_key
                }),
                "{line}: {:?}",
                output.diagnostics
            );
        }

        let invalid_id = parse(
            "@모듈 알림(notifications)\n접수 상태 알림 생성(bad-id)은 점검 요청 전달의 요청 상태가 접수됨이면 점검 요청 전달 알림을 하나 생성한다.\n",
        );
        assert!(invalid_id.diagnostics.is_empty());
        let DeclarationAst::CreationBranch(branch) = &invalid_id.document.unwrap().declarations[0]
        else {
            panic!("invalid stable ID shape still belongs to the common linker")
        };
        assert_eq!(branch.declaration.id, "bad-id");

        let block = format!(
            "@모듈 알림(notifications)\n{}\n    금지된 블록\n",
            "접수 상태 알림 생성(received_notice_create)은 점검 요청 전달의 요청 상태가 접수됨이면 점검 요청 전달 알림을 하나 생성한다."
        );
        assert!(parse(&block).diagnostics.iter().any(|diagnostic| {
            diagnostic.rule_id == "RSPDL-KO-SYN-063"
                && diagnostic.message_key == "ko.syntax.sentence_block_forbidden"
                && diagnostic.argument("kind") == Some("creation_branch")
        }));
    }

    #[test]
    fn parses_the_three_unconditional_field_producer_sentences() {
        let source = "@모듈 검증(check)\n알림 제목 기록(title_binding)은 점검 요청 전달이 실행될 때 알림 제목을 점검 요청 전달 알림의 제목으로 기록한다.\n요청 제목 기록(request_title_binding)은 점검 요청 전달이 실행될 때 대상 요청의 제목을 점검 요청 전달 알림의 요청 제목으로 기록한다.\n재시도 횟수 기록(retry_binding)은 점검 요청 전달이 실행될 때 상수 0을 점검 요청 전달 알림의 재시도 횟수로 기록한다.\n";
        let parsed = parse(source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let producers = parsed
            .document
            .unwrap()
            .declarations
            .into_iter()
            .filter_map(|declaration| match declaration {
                DeclarationAst::FieldProducer(value) => Some(value),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(producers.len(), 3);
        assert!(matches!(
            producers[0].source,
            FieldProducerSourceAst::ActionInput { .. }
        ));
        assert!(matches!(
            producers[1].source,
            FieldProducerSourceAst::InputField { .. }
        ));
        assert!(
            matches!(producers[2].source, FieldProducerSourceAst::Constant { literal: LiteralAst::Integer(ref value) } if value == "0")
        );
        assert!(
            producers
                .iter()
                .all(|producer| producer.condition.is_none())
        );
    }

    #[test]
    fn parses_event_payload_producer_sentences() {
        let source = "@모듈 검증(check)\n제목 기록(title_binding)은 요청 접수됨이 발생할 때 알림 제목을 알림의 제목으로 기록한다.\n수신자 연결(recipient_binding)은 요청 접수됨이 발생할 때 수신 기술자를 알림의 수신자로 연결한다.\n";
        let parsed = parse(source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let declarations = parsed.document.unwrap().declarations;
        let DeclarationAst::FieldProducer(field) = &declarations[0] else {
            panic!()
        };
        assert_eq!(field.trigger.kind, ProducerTriggerKindAst::Event);
        assert_eq!(field.trigger.name, "요청 접수됨");
        let DeclarationAst::RelationProducer(relation) = &declarations[1] else {
            panic!()
        };
        assert_eq!(relation.trigger.kind, ProducerTriggerKindAst::Event);
        assert_eq!(relation.trigger.name, "요청 접수됨");
    }

    #[test]
    fn parses_a_conditional_field_producer_sentence() {
        let source = "@모듈 검증(check)\n접수 제목 기록(received_title)은 전달의 요청 상태가 접수됨이면 상수 \"요청이 접수되었습니다\"를 알림의 제목으로 기록한다.\n";
        let parsed = parse(source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let DeclarationAst::FieldProducer(producer) = &parsed.document.unwrap().declarations[0]
        else {
            panic!()
        };
        assert_eq!(producer.trigger.name, "전달");
        assert_eq!(
            producer
                .condition
                .as_ref()
                .map(|condition| (&condition.input, &condition.variant)),
            Some((&"요청 상태".into(), &"접수됨".into()))
        );
    }

    #[test]
    fn reports_exact_spans_for_invalid_data_usage_sentences() {
        fn assert_diagnostic_span(source: &str, line: &str, rule_id: &str) {
            let diagnostic = parse(source)
                .diagnostics
                .into_iter()
                .find(|diagnostic| diagnostic.rule_id == rule_id)
                .unwrap_or_else(|| panic!("missing diagnostic {rule_id}"));
            let start = source.find(line).expect("test line should be present");
            assert_eq!(
                diagnostic.span,
                Span {
                    start,
                    end: start + line.len(),
                }
            );
        }

        let screen_line = "항목 작성 화면(create_item)에서는 항목을 생성할 수 있다.";
        let screen_with_body = format!("@모듈 테스트(test)\n{screen_line}\n    잘못된 블록\n");
        assert_diagnostic_span(&screen_with_body, screen_line, "RSPDL-KO-SYN-060");

        let missing_marker_line = "항목 작성 화면(create_item)에서는 항목의 금액 조회할 수 있다.";
        let missing_marker = format!("@모듈 테스트(test)\n{missing_marker_line}\n");
        assert_diagnostic_span(&missing_marker, missing_marker_line, "RSPDL-KO-SYN-062");

        let derivation_line = "항목의 합계는 항목의 금액의 합계로 계산한다.";
        let derivation_with_body =
            format!("@모듈 테스트(test)\n{derivation_line}\n    잘못된 블록\n");
        assert_diagnostic_span(&derivation_with_body, derivation_line, "RSPDL-KO-SYN-063");
    }

    #[test]
    fn unnatural_particles_are_non_blocking_warnings() {
        let output = parse(&SOURCE.replace("회계 관리자는", "회계 관리자은"));
        assert!(output.document.is_some());
        assert!(
            output
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.rule_id == "RSPDL-KO-W001"
                    && diagnostic.severity == Severity::Warning)
        );
        assert!(!output.diagnostics.iter().any(Diagnostic::is_error));
    }

    #[test]
    fn arbitrary_utf8_never_panics() {
        for source in [
            "",
            "💥",
            "\0",
            "모듈",
            "`닫히지 않음",
            "모델(model)은 다음 필드들로 구성되어 있다.\n  \t필드",
        ] {
            let result = std::panic::catch_unwind(|| parse(source));
            assert!(result.is_ok(), "{source:?}");
        }
    }

    #[test]
    fn parses_relations_and_relational_meta_rules() {
        let source = r#"@모듈 관계(relations)
프로젝트(project)는 다음 필드들로 구성되어 있다.
    이름(name): 필수 문자열
사용자(user)는 다음 필드들로 구성되어 있다.
    이름(name): 필수 문자열
프로젝트는 사용자를 소유자(owner)로 가질 수 있다.
사용자는 내부(internal)에 해당할 수 있다.
사용자는 외부(external)에 해당할 수 있다.
프로젝트는 하나 이상 존재해야 한다.
모든 프로젝트는 소유자를 하나 이상 가져야 한다.
각 프로젝트는 소유자를 최대 하나만 가질 수 있다.
내부, 외부 중 둘 이상은 동시에 성립할 수 없다.
내부, 외부 중 하나 이상은 항상 성립해야 한다.
소유자, 소유자 후보는 동시에 성립할 수 있다.
"#;
        let output = parse(source);

        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
        let declarations = output.document.unwrap().declarations;
        assert_eq!(
            declarations
                .iter()
                .filter(|declaration| matches!(
                    declaration,
                    DeclarationAst::DataModel(DataModelAst { fields, .. }) if !fields.is_empty()
                ))
                .count(),
            2
        );
        assert_eq!(
            declarations
                .iter()
                .filter(|declaration| matches!(declaration, DeclarationAst::Relation(_)))
                .count(),
            3
        );
        assert_eq!(
            declarations
                .iter()
                .filter(|declaration| {
                    matches!(declaration, DeclarationAst::RelationalConstraint(_))
                })
                .count(),
            6
        );
    }

    #[test]
    fn relation_group_separator_does_not_split_a_name_containing_jung() {
        let source = r#"@모듈 상태(status)
신청(request)은 다음 필드들로 구성되어 있다.
    이름(name): 필수 문자열
신청은 승인 중(pending)에 해당할 수 있다.
신청은 검토 완료(reviewed)에 해당할 수 있다.
승인 중, 검토 완료 중 둘 이상은 동시에 성립할 수 없다.
승인 중, 검토 완료 중 하나 이상은 항상 성립해야 한다.
"#;

        let output = parse(source);
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
        let declarations = output.document.unwrap().declarations;
        let groups = declarations
            .iter()
            .filter_map(|declaration| match declaration {
                DeclarationAst::RelationalConstraint(RelationalConstraintAst {
                    constraint:
                        RelationalConstraintKindAst::Exclusive { relations }
                        | RelationalConstraintKindAst::Exhaustive { relations },
                    ..
                }) => Some(relations),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(groups.len(), 2);
        assert!(
            groups
                .iter()
                .all(|relations| relations.as_slice() == ["승인 중", "검토 완료"])
        );
    }

    #[test]
    fn domain_annotations_are_rejected() {
        let source = "@모듈 관계(relations)\n@열거형 상태(status)\n@개체 프로젝트(project)\n@관계 소유자(owner)\n@비어있지않음 프로젝트\n@필수 소유자\n@유일 소유자\n@배타 소유자, 검토자\n@전체 소유자, 검토자\n@공존 소유자, 검토자\n@역할 관리자(manager)\n@행동 변경(change)\n";
        let output = parse(source);

        assert_eq!(
            output
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.rule_id == "RSPDL-KO-SYN-003")
                .count(),
            11
        );
        assert!(output.diagnostics.iter().all(|diagnostic| {
            diagnostic.message_key == "ko.syntax.domain_annotation_forbidden"
        }));
    }
}
