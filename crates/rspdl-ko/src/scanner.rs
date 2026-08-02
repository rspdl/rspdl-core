use serde::Serialize;

use crate::{Diagnostic, Span};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum TokenKind {
    Word(String),
    QuotedIdentifier(String),
    CanonicalId(String),
    StringLiteral(String),
    Colon,
    Period,
    Indent,
    Dedent,
    Newline,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ScanOutput {
    pub tokens: Vec<Token>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn scan(source: &str) -> ScanOutput {
    let mut tokens = Vec::new();
    let mut diagnostics = Vec::new();
    let mut indents = vec![0usize];
    let mut offset = 0usize;

    for segment in source.split_inclusive('\n') {
        let line = segment.strip_suffix('\n').unwrap_or(segment);
        let line_without_cr = line.strip_suffix('\r').unwrap_or(line);
        let content_end = offset + line_without_cr.len();
        let mut width = 0usize;
        let mut prefix_end = 0usize;
        let mut bad_tab = None;
        for (index, character) in line_without_cr.char_indices() {
            match character {
                ' ' => {
                    width += 1;
                    prefix_end = index + 1;
                }
                '\t' => {
                    bad_tab = Some(index);
                    prefix_end = index + 1;
                }
                _ => break,
            }
        }
        if let Some(index) = bad_tab {
            diagnostics.push(Diagnostic::error(
                "RSPDL-KO-LEX-001",
                "들여쓰기에 tab을 사용할 수 없습니다.",
                Span {
                    start: offset + index,
                    end: offset + index + 1,
                },
            ));
        }

        let content = &line_without_cr[prefix_end..];
        let significant = !content.trim().is_empty() && !content.trim_start().starts_with('#');
        if significant {
            let current = *indents.last().expect("indent stack is never empty");
            if width > current {
                indents.push(width);
                tokens.push(Token {
                    kind: TokenKind::Indent,
                    span: Span {
                        start: offset,
                        end: offset + prefix_end,
                    },
                });
            } else if width < current {
                while indents.last().is_some_and(|value| *value > width) {
                    indents.pop();
                    tokens.push(Token {
                        kind: TokenKind::Dedent,
                        span: Span {
                            start: offset,
                            end: offset + prefix_end,
                        },
                    });
                }
                if indents.last().copied() != Some(width) {
                    diagnostics.push(Diagnostic::error(
                        "RSPDL-KO-LEX-002",
                        "이전 블록과 일치하지 않는 들여쓰기입니다.",
                        Span {
                            start: offset,
                            end: offset + prefix_end,
                        },
                    ));
                    indents.push(width);
                    tokens.push(Token {
                        kind: TokenKind::Indent,
                        span: Span {
                            start: offset,
                            end: offset + prefix_end,
                        },
                    });
                }
            }
            scan_content(content, offset + prefix_end, &mut tokens, &mut diagnostics);
        }
        tokens.push(Token {
            kind: TokenKind::Newline,
            span: Span {
                start: content_end,
                end: content_end + usize::from(segment.ends_with('\n')),
            },
        });
        offset += segment.len();
    }

    if source.is_empty() {
        tokens.push(Token {
            kind: TokenKind::Newline,
            span: Span::default(),
        });
    }
    while indents.len() > 1 {
        indents.pop();
        tokens.push(Token {
            kind: TokenKind::Dedent,
            span: Span {
                start: source.len(),
                end: source.len(),
            },
        });
    }

    ScanOutput {
        tokens,
        diagnostics,
    }
}

fn scan_content(
    content: &str,
    base: usize,
    tokens: &mut Vec<Token>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut cursor = 0usize;
    while cursor < content.len() {
        let character = content[cursor..]
            .chars()
            .next()
            .expect("cursor is on a character boundary");
        if character.is_whitespace() {
            cursor += character.len_utf8();
            continue;
        }
        if character == '#' {
            break;
        }
        let start = cursor;
        match character {
            ':' => {
                cursor += 1;
                push(tokens, TokenKind::Colon, base + start, base + cursor);
            }
            '.' => {
                cursor += 1;
                push(tokens, TokenKind::Period, base + start, base + cursor);
            }
            '`' => {
                cursor += 1;
                let value_start = cursor;
                while cursor < content.len() && !content[cursor..].starts_with('`') {
                    cursor += content[cursor..]
                        .chars()
                        .next()
                        .expect("quoted identifier character")
                        .len_utf8();
                }
                if cursor == content.len() {
                    diagnostics.push(Diagnostic::error(
                        "RSPDL-KO-LEX-003",
                        "닫히지 않은 backtick 식별자입니다.",
                        Span {
                            start: base + start,
                            end: base + cursor,
                        },
                    ));
                    break;
                }
                let value = content[value_start..cursor].to_owned();
                cursor += 1;
                push(
                    tokens,
                    TokenKind::QuotedIdentifier(value),
                    base + start,
                    base + cursor,
                );
            }
            '(' => {
                let closing = ')';
                cursor += 1;
                let value_start = cursor;
                while cursor < content.len() && !content[cursor..].starts_with(closing) {
                    cursor += content[cursor..]
                        .chars()
                        .next()
                        .expect("canonical identifier character")
                        .len_utf8();
                }
                if cursor == content.len() {
                    diagnostics.push(Diagnostic::error(
                        "RSPDL-KO-LEX-004",
                        format!("`{closing}`로 닫히지 않은 stable ID입니다."),
                        Span {
                            start: base + start,
                            end: base + cursor,
                        },
                    ));
                    break;
                }
                let value = content[value_start..cursor].to_owned();
                cursor += 1;
                push(
                    tokens,
                    TokenKind::CanonicalId(value),
                    base + start,
                    base + cursor,
                );
            }
            '"' => {
                let mut end = cursor + 1;
                let mut escaped = false;
                while end < content.len() {
                    let next = content[end..].chars().next().expect("string character");
                    end += next.len_utf8();
                    if next == '"' && !escaped {
                        break;
                    }
                    escaped = next == '\\' && !escaped;
                    if next != '\\' {
                        escaped = false;
                    }
                }
                let raw = &content[start..end.min(content.len())];
                match serde_json::from_str::<String>(raw) {
                    Ok(value) => push(
                        tokens,
                        TokenKind::StringLiteral(value),
                        base + start,
                        base + end,
                    ),
                    Err(_) => diagnostics.push(Diagnostic::error(
                        "RSPDL-KO-LEX-005",
                        "문자열 literal 형식이 올바르지 않습니다.",
                        Span {
                            start: base + start,
                            end: base + end.min(content.len()),
                        },
                    )),
                }
                cursor = end.min(content.len());
            }
            _ => {
                while cursor < content.len() {
                    let next = content[cursor..].chars().next().expect("word character");
                    if next.is_whitespace()
                        || matches!(next, ':' | '.' | '#' | '`' | '[' | '(' | '"')
                    {
                        break;
                    }
                    cursor += next.len_utf8();
                }
                if cursor == start {
                    cursor += character.len_utf8();
                }
                push(
                    tokens,
                    TokenKind::Word(content[start..cursor].to_owned()),
                    base + start,
                    base + cursor,
                );
            }
        }
    }
}

fn push(tokens: &mut Vec<Token>, kind: TokenKind, start: usize, end: usize) {
    tokens.push(Token {
        kind,
        span: Span { start, end },
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_indent_and_dedent_while_ignoring_blank_comments() {
        let output = scan(
            "신청(request)은 다음 필드들로 구성되어 있다.\n  # comment\n  금액(amount): 필수 정수\n@역할 관리자(admin)\n",
        );
        assert!(output.diagnostics.is_empty());
        assert_eq!(
            output
                .tokens
                .iter()
                .filter(|token| matches!(token.kind, TokenKind::Indent))
                .count(),
            1
        );
        assert_eq!(
            output
                .tokens
                .iter()
                .filter(|token| matches!(token.kind, TokenKind::Dedent))
                .count(),
            1
        );
    }

    #[test]
    fn tabs_and_inconsistent_dedents_are_diagnostics() {
        let output = scan(
            "신청(request)은 다음 필드들로 구성되어 있다.\n    금액(amount): 필수 정수\n  이름(name): 필수 문자열\n\t상태(state): 필수 문자열\n",
        );
        assert!(
            output
                .diagnostics
                .iter()
                .any(|d| d.rule_id == "RSPDL-KO-LEX-001")
        );
        assert!(
            output
                .diagnostics
                .iter()
                .any(|d| d.rule_id == "RSPDL-KO-LEX-002")
        );
    }
}
