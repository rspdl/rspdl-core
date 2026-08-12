use rspdl_grammar_compiler::{InputAdapter, TerminalMatch};

use crate::scanner::{Token, TokenKind};

pub(super) struct KoreanTokenAdapter;

impl InputAdapter<Token> for KoreanTokenAdapter {
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
            _ => Vec::new(),
        }
    }
}

pub(super) fn match_literal(
    tokens: &[Token],
    position: usize,
    literal: &str,
) -> Option<TerminalMatch> {
    let token = tokens.get(position)?;
    let matches = match &token.kind {
        TokenKind::Word(value) => value == literal,
        TokenKind::Period => literal == ".",
        TokenKind::Comma => literal == ",",
        TokenKind::Colon => literal == ":",
        _ => false,
    };
    matches.then(|| TerminalMatch::new(position + 1, literal, token.span.start, token.span.end))
}

pub(super) fn match_marked_ref(
    tokens: &[Token],
    position: usize,
    markers: &[String],
) -> Vec<TerminalMatch> {
    match tokens.get(position).map(|token| &token.kind) {
        Some(TokenKind::QuotedIdentifier(value)) => {
            let Some(marker_token) = tokens.get(position + 1) else {
                return Vec::new();
            };
            let TokenKind::Word(marker) = &marker_token.kind else {
                return Vec::new();
            };
            if markers.contains(marker) {
                let token = &tokens[position];
                vec![TerminalMatch::new(
                    position + 2,
                    value,
                    token.span.start,
                    token.span.end,
                )]
            } else {
                Vec::new()
            }
        }
        Some(TokenKind::Word(_)) => marked_bare_references(tokens, position, markers),
        _ => Vec::new(),
    }
}

fn marked_bare_references(
    tokens: &[Token],
    position: usize,
    markers: &[String],
) -> Vec<TerminalMatch> {
    let mut parts = Vec::new();
    let mut index = position;
    while let Some(Token {
        kind: TokenKind::Word(value),
        ..
    }) = tokens.get(index)
    {
        let matches = markers
            .iter()
            .filter_map(|marker| {
                value
                    .strip_suffix(marker)
                    .filter(|base| !base.is_empty())
                    .map(|base| (base, marker))
            })
            .collect::<Vec<_>>();
        if !matches.is_empty() {
            let first = &tokens[position];
            let last = &tokens[index];
            return matches
                .into_iter()
                .map(|(base, marker)| {
                    let mut value_parts = parts.clone();
                    value_parts.push(base.to_owned());
                    TerminalMatch::new(
                        index + 1,
                        value_parts.join(" "),
                        first.span.start,
                        last.span.end - marker.len(),
                    )
                })
                .collect();
        }
        parts.push(value.clone());
        index += 1;
    }
    Vec::new()
}
