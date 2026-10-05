//! Generated fixed clauses with indentation-owned recursive statement structure.
use crate::ast::*;
use crate::grammar_adapter as adapter;
use crate::parser::Line;
use crate::{Diagnostic, Span, Token, TokenKind};
use rspdl_grammar_compiler::{Capture, InputAdapter, ParseMatch, TerminalMatch};
include!(concat!(env!("OUT_DIR"), "/statement_grammar.rs"));
struct Adapter;
impl InputAdapter<Token> for Adapter {
    fn match_literal(
        &self,
        tokens: &[Token],
        position: usize,
        literal: &str,
    ) -> Option<TerminalMatch> {
        adapter::KoreanTokenAdapter.match_literal(tokens, position, literal)
    }
    fn match_contextual(
        &self,
        tokens: &[Token],
        position: usize,
        matcher: &str,
        args: &[String],
    ) -> Vec<TerminalMatch> {
        let Some(first) = tokens.get(position) else {
            return vec![];
        };
        match matcher {
            "marked_ref" => adapter::match_marked_ref(tokens, position, args),
            "canonical_id" => match &first.kind {
                TokenKind::CanonicalId(v) => vec![TerminalMatch::new(
                    position + 1,
                    v,
                    first.span.start,
                    first.span.end,
                )],
                _ => vec![],
            },
            "surface_name" => {
                let mut parts = vec![];
                let mut out = vec![];
                for (i, t) in tokens.iter().enumerate().skip(position) {
                    match &t.kind {
                        TokenKind::Word(v) | TokenKind::QuotedIdentifier(v) => {
                            parts.push(v.as_str())
                        }
                        _ => break,
                    }
                    out.push(TerminalMatch::new(
                        i + 1,
                        parts.join(" "),
                        first.span.start,
                        t.span.end,
                    ));
                }
                out
            }
            "statement_operand" => {
                let mut out = vec![];
                if let TokenKind::StringLiteral(v) = &first.kind
                    && let Some(t) = tokens.get(position + 1)
                    && matches!(&t.kind,TokenKind::Word(m) if args.contains(m))
                {
                    out.push(TerminalMatch::new(
                        position + 2,
                        format!("S{}", serde_json::to_string(v).unwrap()),
                        first.span.start,
                        t.span.end,
                    ));
                }
                for m in adapter::match_marked_ref(tokens, position, args) {
                    if !adapter::match_marked_ref(tokens, position, &["의".into()])
                        .iter()
                        .all(|b| b.end >= m.end)
                    {
                        continue;
                    }
                    out.push(TerminalMatch::new(
                        m.end,
                        format!(
                            "{}{}",
                            if matches!(first.kind, TokenKind::QuotedIdentifier(_)) {
                                "Q"
                            } else {
                                "I"
                            },
                            serde_json::to_string(&m.value).unwrap()
                        ),
                        m.start_offset,
                        m.end_offset,
                    ));
                }
                for b in adapter::match_marked_ref(tokens, position, &["의".into()]) {
                    for f in adapter::match_marked_ref(tokens, b.end, args) {
                        out.push(TerminalMatch::new(
                            f.end,
                            format!(
                                "F{}",
                                serde_json::to_string(&(b.value.clone(), f.value)).unwrap()
                            ),
                            b.start_offset,
                            f.end_offset,
                        ));
                    }
                }
                out
            }
            _ => vec![],
        }
    }
}
fn clause(line: &Line, rule: &str) -> Option<ParseMatch> {
    generated_statement_grammar()
        .parse(rule, &line.tokens, &Adapter)
        .ok()
}
fn cap(p: &ParseMatch, n: &str) -> String {
    p.capture(n).expect("grammar capture").value.clone()
}
fn named(p: &ParseMatch, span: Span) -> NamedIdAst {
    NamedIdAst {
        name: cap(p, "name"),
        id: cap(p, "id"),
        span,
    }
}
fn error(span: Span) -> Diagnostic {
    Diagnostic::error("RSPDL-KO-SYN-060", "ko.syntax.statement_invalid", span)
}
pub(crate) fn is_header(line: &Line) -> bool {
    // Recognize the unambiguous fixed ending even when the declaration ID is missing.
    line.tokens
        .iter()
        .any(|t| matches!(&t.kind,TokenKind::Word(v) if v == "정한다"))
        && line
            .tokens
            .iter()
            .any(|t| matches!(&t.kind,TokenKind::Word(v) if v == "다음과"))
}
fn operand(c: &Capture) -> StatementOperandAst {
    let v = &c.value[1..];
    match c.value.as_bytes()[0] {
        b'S' => StatementOperandAst::Literal(LiteralAst::String(serde_json::from_str(v).unwrap())),
        b'Q' => StatementOperandAst::Input(serde_json::from_str(v).unwrap()),
        b'F' => {
            let (binding, field) = serde_json::from_str(v).unwrap();
            StatementOperandAst::InputField { binding, field }
        }
        _ => {
            let s: String = serde_json::from_str(v).unwrap();
            if s == "참" || s == "거짓" {
                StatementOperandAst::Literal(LiteralAst::Boolean(s == "참"))
            } else if !s.trim_start_matches(['-', '+']).is_empty()
                && s.trim_start_matches(['-', '+'])
                    .bytes()
                    .all(|b| b.is_ascii_digit())
            {
                StatementOperandAst::Literal(LiteralAst::Integer(s))
            } else {
                StatementOperandAst::Input(s)
            }
        }
    }
}
fn children_end(lines: &[Line], start: usize) -> usize {
    (start + 1..lines.len())
        .find(|&i| lines[i].indent <= lines[start].indent)
        .unwrap_or(lines.len())
}
fn condition(lines: &[Line]) -> Result<StatementConditionAst, Diagnostic> {
    if lines.is_empty() {
        return Err(error(Span::default()));
    }
    let mut conditions = vec![];
    let mut i = 0;
    while i < lines.len() {
        let l = &lines[i];
        if l.indent != lines[0].indent {
            return Err(error(l.span));
        }
        let end = children_end(lines, i);
        let children = &lines[i + 1..end];
        let value = if clause(l, "all").is_some() {
            if children.is_empty() {
                return Err(error(l.span));
            }
            condition_group(children, false)?
        } else if clause(l, "any").is_some() {
            if children.is_empty() {
                return Err(error(l.span));
            }
            condition_group(children, true)?
        } else if clause(l, "not").is_some() {
            if children.is_empty()
                || children.iter().filter(|c| c.indent == l.indent + 1).count() != 1
            {
                return Err(error(l.span));
            }
            let inner = condition(children)?;
            StatementConditionAst::Not(Box::new(inner))
        } else if let Some(p) = clause(l, "comparison") {
            if !children.is_empty() {
                return Err(error(l.span));
            }
            let op = match cap(&p, "operator").as_str() {
                "같다" => RelationOperatorAst::Equal,
                "다르다" => RelationOperatorAst::NotEqual,
                "작다" => RelationOperatorAst::LessThan,
                "작거나 같다" => RelationOperatorAst::LessThanOrEqual,
                "크다" => RelationOperatorAst::GreaterThan,
                _ => RelationOperatorAst::GreaterThanOrEqual,
            };
            StatementConditionAst::Compare {
                left: operand(p.capture("left").unwrap()),
                operator: op,
                right: operand(p.capture("right").unwrap()),
                span: l.span,
            }
        } else {
            return Err(error(l.span));
        };
        conditions.push(value);
        i = end;
    }
    if conditions.len() == 1 {
        Ok(conditions.remove(0))
    } else {
        Ok(StatementConditionAst::And(conditions))
    }
}
fn condition_group(lines: &[Line], or: bool) -> Result<StatementConditionAst, Diagnostic> {
    if lines.is_empty() {
        return Err(error(Span::default()));
    }
    let value = condition(lines)?;
    let sibling_count = lines.iter().filter(|l| l.indent == lines[0].indent).count();
    let values = if sibling_count > 1 {
        match value {
            StatementConditionAst::And(xs) => xs,
            x => vec![x],
        }
    } else {
        vec![value]
    };
    Ok(if or {
        StatementConditionAst::Or(values)
    } else {
        StatementConditionAst::And(values)
    })
}
pub(crate) fn parse(header: &Line, lines: &[Line]) -> Result<StatementAst, Diagnostic> {
    let p = clause(header, "header").ok_or_else(|| error(header.span))?;
    if let Some(line) = lines.iter().find(|l| l.indent > 64) {
        return Err(error(line.span));
    }
    let mut trigger = None;
    let mut actor = None;
    let mut trigger_span = header.span;
    let mut actor_span = header.span;
    let mut policy = None;
    let mut bindings = vec![];
    let mut effects = vec![];
    let mut cond = None;
    let mut i = 0;
    while i < lines.len() {
        let l = &lines[i];
        if l.indent != 1 {
            return Err(error(l.span));
        }
        let end = children_end(lines, i);
        let children = &lines[i + 1..end];
        if let Some(p) = clause(l, "trigger") {
            if trigger.is_some() || !children.is_empty() {
                return Err(error(l.span));
            }
            trigger_span = l.span;
            trigger = Some(ProducerTriggerAst {
                name: cap(&p, "name"),
                kind: if cap(&p, "kind") == "실행될" {
                    ProducerTriggerKindAst::Action
                } else {
                    ProducerTriggerKindAst::Event
                },
            });
        } else if let Some(p) = clause(l, "actor") {
            if actor.is_some() || !children.is_empty() {
                return Err(error(l.span));
            }
            actor_span = l.span;
            actor = Some(cap(&p, "name"));
        } else if let Some(p) = clause(l, "binding") {
            if !children.is_empty() {
                return Err(error(l.span));
            }
            bindings.push(StatementBindingAst {
                declaration: named(&p, l.span),
                owner: cap(&p, "owner"),
                input: cap(&p, "input"),
                span: l.span,
            });
        } else if let Some(p) = clause(l, "policy") {
            if policy.is_some() || !children.is_empty() {
                return Err(error(l.span));
            }
            policy = Some(match cap(&p, "kind").as_str() {
                "할 수 있다" => rspdl_domain::StatementPolicy::Can,
                "할 수 없다" => rspdl_domain::StatementPolicy::Cannot,
                _ => rspdl_domain::StatementPolicy::DoAttempt,
            });
        } else if clause(l, "all").is_some()
            || clause(l, "any").is_some()
            || clause(l, "not").is_some()
        {
            if cond.is_some() {
                return Err(error(l.span));
            }
            cond = Some(condition(&lines[i..end])?);
        } else if let Some(p) = clause(l, "effect") {
            let binding = cap(&p, "binding");
            let span = l.span;
            effects.push(match cap(&p, "kind").as_str() {
                "조회한다" => {
                    if !children.is_empty() {
                        return Err(error(span));
                    }
                    StatementEffectAst::Read { binding, span }
                }
                "삭제한다" => {
                    if !children.is_empty() {
                        return Err(error(span));
                    }
                    StatementEffectAst::Delete { binding, span }
                }
                _ => {
                    if children.is_empty() {
                        return Err(error(span));
                    }
                    let mut assignments = vec![];
                    for c in children {
                        if c.indent != l.indent + 1 {
                            return Err(error(c.span));
                        }
                        let p = clause(c, "assignment").ok_or_else(|| error(c.span))?;
                        assignments.push(StatementAssignmentAst {
                            field: cap(&p, "field"),
                            value: operand(p.capture("value").unwrap()),
                            span: c.span,
                        });
                    }
                    StatementEffectAst::Update {
                        binding,
                        assignments,
                        span,
                    }
                }
            });
        } else {
            return Err(error(l.span));
        }
        i = end;
    }
    if effects.is_empty() {
        return Err(error(header.span));
    }
    Ok(StatementAst {
        declaration: named(&p, header.span),
        trigger: trigger.ok_or_else(|| error(header.span))?,
        actor: actor.ok_or_else(|| error(header.span))?,
        trigger_span,
        actor_span,
        bindings,
        condition: cond.unwrap_or(StatementConditionAst::True),
        policy: policy.ok_or_else(|| error(header.span))?,
        effects,
        span: header.span,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn source(condition: &str) -> String {
        format!(
            "@모듈 검사(check)\n처리(rule)는 다음과 같이 정한다.\n    변경이 실행될 때 적용한다.\n    시스템이 수행한다.\n    대상(target)은 변경의 대상 입력을 사용한다.\n{condition}    이 처리를 자동으로 시도한다.\n    대상을 수정한다.\n        횟수를 0으로 정한다.\n"
        )
    }
    fn statement(condition: &str) -> StatementAst {
        let p = crate::parse(&source(condition));
        assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
        match p.document.unwrap().declarations.remove(0) {
            DeclarationAst::Statement(s) => s,
            _ => panic!("statement"),
        }
    }
    #[test]
    fn preserves_nested_all_inside_any_and_not() {
        let inner = "        다음 조건을 모두 만족할 때 적용한다.\n            대상의 횟수가 0보다 크다.\n            대상의 횟수가 10보다 작다.\n";
        let s = statement(&format!(
            "    다음 조건 중 하나 이상을 만족할 때 적용한다.\n{inner}"
        ));
        assert!(
            matches!(s.condition,StatementConditionAst::Or(xs) if matches!(xs.as_slice(),[StatementConditionAst::And(ys)] if ys.len()==2))
        );
        let s = statement(&format!(
            "    다음 조건을 만족하지 않을 때 적용한다.\n{inner}"
        ));
        assert!(
            matches!(s.condition,StatementConditionAst::Not(x) if matches!(&*x,StatementConditionAst::And(ys) if ys.len()==2))
        );
        let s = statement(&format!(
            "    다음 조건 중 하나 이상을 만족할 때 적용한다.\n{inner}        대상의 횟수가 20과 같다.\n"
        ));
        assert!(
            matches!(s.condition,StatementConditionAst::Or(xs) if xs.len()==2 && matches!(&xs[0],StatementConditionAst::And(ys) if ys.len()==2))
        );
    }
    #[test]
    fn formatter_round_trip_quoted_names_and_groups() {
        let input = source(
            "    다음 조건을 모두 만족할 때 적용한다.\n        `대상` 의 `횟수` 가 0 보다 크다.\n",
        );
        let formatted = crate::format_source(&input);
        assert!(
            formatted.diagnostics.is_empty(),
            "{:?}",
            formatted.diagnostics
        );
        let text = formatted.text.unwrap();
        assert_eq!(crate::format_source(&text).text, Some(text));
    }
    #[test]
    fn formatter_keeps_quoted_aliases_with_particles_or_literal_names() {
        for name in ["대상 의 횟수", "값이", "참", "123"] {
            let input = source(&format!(
                "    다음 조건을 모두 만족할 때 적용한다.\n        `{name}` 이 0보다 크다.\n"
            ))
            .replace("대상(target)", &format!("`{name}`(target)"))
            .replace("대상을 수정한다", &format!("`{name}` 을 수정한다"));
            let first = crate::parse(&input);
            assert!(
                first.diagnostics.is_empty(),
                "{name}: {:?}",
                first.diagnostics
            );
            let formatted = crate::format_source(&input).text.unwrap();
            let second = crate::parse(&formatted);
            assert!(
                second.diagnostics.is_empty(),
                "{name}: {:?}",
                second.diagnostics
            );
            let DeclarationAst::Statement(s) = &second.document.unwrap().declarations[0] else {
                panic!("statement")
            };
            let StatementConditionAst::And(xs) = &s.condition else {
                panic!("and")
            };
            assert!(
                matches!(&xs[0], StatementConditionAst::Compare { left: StatementOperandAst::Input(v), .. } if v == name)
            );
            assert_eq!(crate::format_source(&formatted).text, Some(formatted));
        }
    }

    #[test]
    fn rejects_empty_group_multiple_not_children_and_missing_clauses() {
        for cond in [
            "    다음 조건을 모두 만족할 때 적용한다.\n",
            "    다음 조건을 만족하지 않을 때 적용한다.\n        대상의 횟수가 0보다 크다.\n        대상의 횟수가 1보다 크다.\n",
        ] {
            assert!(
                crate::parse(&source(cond))
                    .diagnostics
                    .iter()
                    .any(Diagnostic::is_error)
            );
        }
        for missing in [
            "    시스템이 수행한다.\n",
            "    이 처리를 자동으로 시도한다.\n",
            "    변경이 실행될 때 적용한다.\n",
        ] {
            assert!(
                crate::parse(&source("").replace(missing, ""))
                    .diagnostics
                    .iter()
                    .any(Diagnostic::is_error)
            );
        }
    }
    #[test]
    fn rejects_excessive_condition_nesting() {
        let mut conditions = String::new();
        for depth in 1..=65 {
            conditions.push_str(&format!(
                "{}다음 조건을 모두 만족할 때 적용한다.\n",
                "    ".repeat(depth)
            ));
        }
        conditions.push_str(&format!("{}대상의 횟수가 0보다 크다.\n", "    ".repeat(66)));
        assert!(
            crate::parse(&source(&conditions))
                .diagnostics
                .iter()
                .any(Diagnostic::is_error)
        );
    }
}
