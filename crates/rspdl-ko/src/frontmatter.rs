//! 문서 머리말(frontmatter) 블록 reader.
//!
//! 문서 맨 앞의 `---` 블록은 정보구조, 화면 레이아웃과 화면 흐름을 담는다. 문장 문법과 달리
//! YAML 부분집합이므로 문장 scanner 를 거치지 않는다 — 두 문법 모두 들여쓰기에 의미를 두기
//! 때문에 한 scanner 로 읽으면 서로의 블록 경계를 침범한다. 그래서 블록을 먼저 떼어 내고
//! 이 모듈이 따로 읽는다.
//!
//! 여기서는 **모양만** 읽는다. 참조된 화면·필드·모델·행동이 실제로 선언되었는지는 이후
//! 단계가 판정한다.

use std::borrow::Cow;

use crate::ast::{
    CategoryAst, FrontmatterAst, FrontmatterRefAst, LayoutElementAst, NamedIdAst, ScreenLayoutAst,
    ScreenLayoutKindAst, ScreenPathAst,
};
use crate::{Diagnostic, Span};

const UNTERMINATED_BLOCK: &str = "RSPDL-KO-FM-001";
const TAB_INDENTATION: &str = "RSPDL-KO-FM-002";
const INCONSISTENT_INDENT: &str = "RSPDL-KO-FM-003";
const UNSUPPORTED_YAML_FEATURE: &str = "RSPDL-KO-FM-004";
const UNKNOWN_KEY: &str = "RSPDL-KO-FM-005";
const DUPLICATE_KEY: &str = "RSPDL-KO-FM-006";
const INVALID_SCALAR_TYPE: &str = "RSPDL-KO-FM-007";
const INVALID_STRUCTURE: &str = "RSPDL-KO-FM-008";
const UNKNOWN_LAYOUT_ELEMENT: &str = "RSPDL-KO-FM-009";
const KEY_INDENTED_TOO_DEEP: &str = "RSPDL-KO-FM-011";

/// 블록의 경계를 이루는 줄. Markdown frontmatter 와 같은 표기다.
const DELIMITER: &str = "---";

/// 머리말 최상위 키.
const KEY_MODULE: &str = "모듈";
const KEY_INFORMATION_ARCHITECTURE: &str = "정보구조";
const KEY_SCREENS: &str = "화면";
const KEY_PATHS: &str = "흐름";

pub(crate) struct FrontmatterOutput<'a> {
    pub(crate) frontmatter: Option<FrontmatterAst>,
    /// 문장 scanner 가 볼 원문.
    ///
    /// 머리말 구간을 잘라 내지 않고 **공백으로 덮는다**. 길이가 그대로이므로 이후 모든 span 이
    /// 파일 기준 offset 으로 맞아떨어진다 — 잘라 내면 scanner 가 만든 모든 span 에 보정을
    /// 더해야 하고, 그 보정을 한 군데서만 빠뜨려도 진단이 엉뚱한 곳을 가리킨다.
    pub(crate) source: Cow<'a, str>,
    pub(crate) diagnostics: Vec<Diagnostic>,
}

/// 머리말을 읽고, 문장 parser 가 볼 원문을 함께 돌려준다.
pub(crate) fn read(source: &str) -> FrontmatterOutput<'_> {
    let Some(block) = locate(source) else {
        return FrontmatterOutput {
            frontmatter: None,
            source: Cow::Borrowed(source),
            diagnostics: Vec::new(),
        };
    };

    let mut diagnostics = Vec::new();
    let block_span = Span {
        start: block.start,
        end: block.end,
    };

    let Some(inner) = block.inner else {
        diagnostics.push(Diagnostic::error(
            UNTERMINATED_BLOCK,
            "ko.frontmatter.unterminated_block",
            block_span,
        ));
        // 닫히지 않았으면 어디까지가 머리말인지 알 수 없다. 원문을 그대로 넘겨 문장 parser 가
        // 자기 진단을 내게 둔다 — 여기서 임의로 잘라 내면 그 결과가 더 혼란스럽다.
        return FrontmatterOutput {
            frontmatter: None,
            source: Cow::Borrowed(source),
            diagnostics,
        };
    };

    let lines = split_lines(source, inner, &mut diagnostics);
    let value = parse_block(&lines, block_span, &mut diagnostics);
    let frontmatter = build_frontmatter(&value, block_span, &mut diagnostics);

    FrontmatterOutput {
        frontmatter: Some(frontmatter),
        source: Cow::Owned(mask(source, block.start, block.end)),
        diagnostics,
    }
}

struct Block {
    /// 여는 `---` 의 시작.
    start: usize,
    /// 닫는 `---` 의 끝. 닫히지 않았으면 원문 끝.
    end: usize,
    /// 두 구분선 사이 본문의 byte 범위.
    inner: Option<(usize, usize)>,
}

/// 맨 앞 `---` 블록을 찾는다. 첫 줄이 구분선이 아니면 머리말이 없는 문서다.
fn locate(source: &str) -> Option<Block> {
    let mut offset = 0usize;
    let mut opening_end = None;

    for segment in source.split_inclusive('\n') {
        let line = trim_line_end(segment);
        if opening_end.is_none() {
            // 여는 구분선은 반드시 첫 줄이어야 한다. 앞에 무엇이든 있으면 머리말이 아니다.
            if line.trim_end() != DELIMITER {
                return None;
            }
            opening_end = Some(offset + segment.len());
            offset += segment.len();
            continue;
        }
        if line.trim_end() == DELIMITER {
            let inner_start = opening_end.expect("opening delimiter");
            return Some(Block {
                start: 0,
                end: offset + segment.len(),
                inner: Some((inner_start, offset)),
            });
        }
        offset += segment.len();
    }

    opening_end.map(|_| Block {
        start: 0,
        end: source.len(),
        inner: None,
    })
}

fn trim_line_end(segment: &str) -> &str {
    let line = segment.strip_suffix('\n').unwrap_or(segment);
    line.strip_suffix('\r').unwrap_or(line)
}

/// 머리말 구간을 같은 길이의 공백으로 덮는다. 줄바꿈은 남겨 줄 번호가 흔들리지 않게 한다.
fn mask(source: &str, start: usize, end: usize) -> String {
    let mut bytes = source.as_bytes().to_vec();
    for byte in &mut bytes[start..end] {
        if *byte != b'\n' {
            *byte = b' ';
        }
    }
    String::from_utf8(bytes).expect("masking replaces whole bytes with ASCII space")
}

#[derive(Clone, Copy, Debug)]
struct FmLine<'a> {
    indent: usize,
    /// 주석과 앞뒤 공백을 걷어낸 본문.
    content: &'a str,
    /// 본문 첫 글자의 파일 기준 byte offset.
    offset: usize,
}

impl FmLine<'_> {
    fn span(&self) -> Span {
        Span {
            start: self.offset,
            end: self.offset + self.content.len(),
        }
    }
}

fn split_lines<'a>(
    source: &'a str,
    inner: (usize, usize),
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<FmLine<'a>> {
    let (start, end) = inner;
    let mut lines = Vec::new();
    let mut offset = start;

    for segment in source[start..end].split_inclusive('\n') {
        let raw = trim_line_end(segment);
        let mut indent = 0usize;
        let mut prefix_end = 0usize;
        let mut tabbed = false;
        for (index, character) in raw.char_indices() {
            match character {
                ' ' => {
                    indent += 1;
                    prefix_end = index + 1;
                }
                '\t' => {
                    diagnostics.push(Diagnostic::error(
                        TAB_INDENTATION,
                        "ko.frontmatter.tab_indentation",
                        Span {
                            start: offset + index,
                            end: offset + index + 1,
                        },
                    ));
                    tabbed = true;
                    prefix_end = index + 1;
                }
                _ => break,
            }
        }

        let body = strip_comment(&raw[prefix_end..]).trim_end();
        // 탭을 쓴 줄은 들여쓰기 폭을 알 수 없다. 임의로 정해 나무에 끼우면 그 줄이 엉뚱한
        // 깊이에 붙어 두 번째 진단을 부른다.
        if !body.is_empty() && !tabbed {
            // `...` 는 YAML 의 문서 끝 표시다. 머리말은 문서 하나뿐이므로 받지 않는다.
            if body == "..." {
                diagnostics.push(
                    Diagnostic::error(
                        UNSUPPORTED_YAML_FEATURE,
                        "ko.frontmatter.unsupported_yaml_feature",
                        Span {
                            start: offset + prefix_end,
                            end: offset + prefix_end + body.len(),
                        },
                    )
                    .with_argument("feature", "document_end"),
                );
            } else {
                lines.push(FmLine {
                    indent,
                    content: body,
                    offset: offset + prefix_end,
                });
            }
        }
        offset += segment.len();
    }

    lines
}

/// 따옴표 밖의 `#` 부터를 주석으로 본다. 공백 뒤이거나 줄 첫 글자일 때만 주석이다.
fn strip_comment(line: &str) -> &str {
    let mut quoted = false;
    let mut escaped = false;
    let mut previous_is_space = true;
    for (index, character) in line.char_indices() {
        if quoted {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                quoted = false;
            }
            previous_is_space = false;
            continue;
        }
        match character {
            '"' => quoted = true,
            '#' if previous_is_space => return &line[..index],
            _ => {}
        }
        previous_is_space = character.is_whitespace();
    }
    line
}

#[derive(Clone, Debug)]
struct FmScalar {
    text: String,
    quoted: bool,
    span: Span,
}

#[derive(Clone, Debug)]
struct FmEntry {
    key: FmScalar,
    value: FmValue,
}

#[derive(Clone, Debug)]
enum FmValue {
    Scalar(FmScalar),
    Sequence {
        items: Vec<FmValue>,
        span: Span,
    },
    Mapping {
        entries: Vec<FmEntry>,
        span: Span,
    },
    /// `key:` 뒤에 아무것도 없는 자리.
    Empty {
        span: Span,
    },
    /// 이미 진단을 낸 자리.
    ///
    /// 한 실수는 진단 하나로 끝나야 한다. 받지 않는 표기를 거절해 놓고 그 자리를 다시
    /// "구조가 틀렸다"고 말하면, 읽는 사람은 있지도 않은 두 번째 문제를 찾으러 간다.
    Rejected {
        span: Span,
    },
}

impl FmValue {
    fn span(&self) -> Span {
        match self {
            FmValue::Scalar(scalar) => scalar.span,
            FmValue::Sequence { span, .. }
            | FmValue::Mapping { span, .. }
            | FmValue::Empty { span }
            | FmValue::Rejected { span } => *span,
        }
    }
}

fn parse_block(lines: &[FmLine], fallback: Span, diagnostics: &mut Vec<Diagnostic>) -> FmValue {
    let Some(first) = lines.first() else {
        return FmValue::Empty { span: fallback };
    };
    let indent = first.indent;
    let span = Span {
        start: first.offset,
        end: lines
            .last()
            .map(|line| line.offset + line.content.len())
            .unwrap_or(first.offset),
    };

    if is_sequence_item(first.content) {
        let items = parse_sequence(lines, indent, diagnostics);
        FmValue::Sequence { items, span }
    } else {
        let entries = parse_mapping(lines, indent, diagnostics);
        FmValue::Mapping { entries, span }
    }
}

fn is_sequence_item(content: &str) -> bool {
    content == "-" || content.starts_with("- ")
}

/// 이 블록에 속한 줄들을 항목 단위로 끊는다. 항목 다음의 더 깊은 줄은 그 항목의 몫이다.
fn child_extent(lines: &[FmLine], start: usize, indent: usize) -> usize {
    (start + 1..lines.len())
        .find(|index| lines[*index].indent <= indent)
        .unwrap_or(lines.len())
}

/// 들여쓰기가 어긋나 버린 줄이 삼켜야 할 범위.
///
/// 버려진 줄에 딸린 줄까지 함께 삼킨다. 한 줄이 어긋난 결과로 그 아래가 또 어긋났다고 말하면
/// 원인 하나에 진단이 둘이 된다. 기준은 버려진 줄과 지금 블록 중 **더 깊은 쪽**이다 — 더 얕게
/// 잡으면 뒤따르는 멀쩡한 형제까지 함께 삼킨다.
fn dropped_extent(lines: &[FmLine], start: usize, indent: usize) -> usize {
    child_extent(lines, start, lines[start].indent.max(indent))
}

fn parse_sequence(
    lines: &[FmLine],
    indent: usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<FmValue> {
    let mut items = Vec::new();
    let mut cursor = 0usize;

    while cursor < lines.len() {
        let line = &lines[cursor];
        if line.indent != indent {
            report_inconsistent_indent(line, diagnostics);
            // 자리를 남기지 않는다. 순서열에는 누가 기다리는 키가 없으므로 남길 이유가 없고,
            // 남기면 요소를 세는 쪽이 그 빈자리를 두고 한 번 더 말한다.
            cursor = dropped_extent(lines, cursor, indent);
            continue;
        }
        if !is_sequence_item(line.content) {
            diagnostics.push(
                Diagnostic::error(
                    INVALID_STRUCTURE,
                    "ko.frontmatter.invalid_structure",
                    line.span(),
                )
                .with_argument("expected", "sequence_item"),
            );
            cursor += 1;
            continue;
        }

        let end = child_extent(lines, cursor, indent);
        let children = &lines[cursor + 1..end];
        // `- ` 뒤에 남은 본문은 항목의 첫 줄이다. 그 자리의 열을 그대로 들여쓰기로 삼아야
        // 뒤따르는 줄들과 같은 블록으로 묶인다.
        let rest_index = line.content[1..]
            .find(|character: char| character != ' ')
            .map(|index| index + 1);

        let value = match rest_index {
            Some(index) => {
                let inline = FmLine {
                    indent: line.indent + index,
                    content: &line.content[index..],
                    offset: line.offset + index,
                };
                let mut block = Vec::with_capacity(1 + children.len());
                block.push(inline);
                block.extend_from_slice(children);
                parse_block(&block, line.span(), diagnostics)
            }
            None => parse_block(children, line.span(), diagnostics),
        };
        items.push(value);
        cursor = end;
    }

    items
}

fn parse_mapping(
    lines: &[FmLine],
    indent: usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<FmEntry> {
    let mut entries: Vec<FmEntry> = Vec::new();
    let mut cursor = 0usize;

    while cursor < lines.len() {
        let line = &lines[cursor];
        if line.indent != indent {
            report_inconsistent_indent(line, diagnostics);
            // 읽히는 키가 있으면 그 자리를 거절된 채로 남긴다. 버려진 줄이 하필 누가 기다리던
            // 키였을 수 있고, 통째로 버리면 그쪽이 "없다"고 한 번 더 말한다.
            if let Some((key_text, _)) = split_key(line.content) {
                let key_text = key_text.trim();
                // 순서열 항목은 키가 아니다. `- 버튼: ...` 에서 `- 버튼` 을 키로 남기면
                // 있지도 않은 이름의 자리가 생긴다.
                if !key_text.is_empty() && !is_sequence_item(line.content) {
                    entries.push(FmEntry {
                        key: FmScalar {
                            text: key_text.to_owned(),
                            quoted: false,
                            span: line.span(),
                        },
                        value: FmValue::Rejected { span: line.span() },
                    });
                }
            }
            cursor = dropped_extent(lines, cursor, indent);
            continue;
        }

        let end = child_extent(lines, cursor, indent);
        let Some((key_text, value_text)) = split_key(line.content) else {
            diagnostics.push(
                Diagnostic::error(
                    INVALID_STRUCTURE,
                    "ko.frontmatter.invalid_structure",
                    line.span(),
                )
                .with_argument("expected", "mapping_entry"),
            );
            cursor = end;
            continue;
        };

        let key_span = Span {
            start: line.offset,
            end: line.offset + key_text.len(),
        };
        if let Some(feature) = unsupported_key_feature(key_text) {
            diagnostics.push(
                Diagnostic::error(
                    UNSUPPORTED_YAML_FEATURE,
                    "ko.frontmatter.unsupported_yaml_feature",
                    key_span,
                )
                .with_argument("feature", feature),
            );
            // 자리는 남긴다. 통째로 버리면 이 키를 기다리던 쪽이 "없다"고 한 번 더 말한다.
            entries.push(FmEntry {
                key: FmScalar {
                    text: key_text.to_owned(),
                    quoted: false,
                    span: key_span,
                },
                value: FmValue::Rejected { span: key_span },
            });
            cursor = end;
            continue;
        }

        let key = read_scalar(key_text, key_span, diagnostics);
        let value_offset = line.offset + line.content.len() - value_text.len();
        let mut overdeep = Vec::new();
        let value = if value_text.is_empty() {
            parse_block(&lines[cursor + 1..end], line.span(), diagnostics)
        } else {
            // 인라인 값이 이미 있는데 더 깊은 줄이 딸려 왔다. 그 줄들은 값으로도, 이 매핑의
            // 항목으로도 들어갈 자리가 없다.
            overdeep = overdeep_slots(&lines[cursor + 1..end], diagnostics);
            parse_inline(value_text, value_offset, diagnostics)
        };

        let first_seen = entries
            .iter()
            .find(|entry| entry.key.text == key.text && !is_rejected(&entry.value))
            .map(|entry| entry.key.span.start);
        if let Some(first) = first_seen {
            diagnostics.push(
                Diagnostic::error(DUPLICATE_KEY, "ko.frontmatter.duplicate_key", key.span)
                    .with_argument("key", &key.text)
                    .with_argument("first", first),
            );
        } else {
            entries.push(FmEntry { key, value });
        }
        entries.extend(overdeep);
        cursor = end;
    }

    entries
}

/// 인라인 값을 가진 키 아래에 더 깊은 줄이 딸려 온 경우, 그 줄들의 자리.
///
/// 조용히 버리면 그 줄에 적힌 키를 기다리던 쪽이 "없다"고 말한다. 쓴 사람 눈에는 그 키가 분명히
/// 적혀 있으므로, 없다는 말은 거짓이고 눈앞의 것을 찾아 헤매게 만든다. 거짓인 진단은 막연한
/// 진단보다 나쁘다.
///
/// 그래서 버리는 대신 어긋난 줄을 짚어 말하고, 그 키의 자리를 거절된 채로 남긴다. 자리를 남겨야
/// 기다리던 쪽이 "없다"고 한 번 더 말하지 않는다 — 원인 하나에 진단은 하나다.
///
/// 딸려 온 블록의 **첫 단만** 본다. 그보다 깊은 줄은 어긋난 그 줄에 딸린 것이지 따로 어긋난
/// 것이 아니다. 키로 읽히지 않는 줄은 이름을 댈 수 없으므로 건드리지 않고, 그 경우에는 원래대로
/// "없다"가 나간다 — 막연하지만 참인 말이 정확하지만 거짓인 말보다 낫다.
fn overdeep_slots(lines: &[FmLine], diagnostics: &mut Vec<Diagnostic>) -> Vec<FmEntry> {
    let Some(first) = lines.first() else {
        return Vec::new();
    };
    let mut slots = Vec::new();
    for line in lines.iter().filter(|line| line.indent == first.indent) {
        if is_sequence_item(line.content) {
            continue;
        }
        let Some((key_text, _)) = split_key(line.content) else {
            continue;
        };
        let key_text = key_text.trim();
        if key_text.is_empty() {
            continue;
        }
        let key_span = Span {
            start: line.offset,
            end: line.offset + key_text.len(),
        };
        diagnostics.push(
            Diagnostic::error(
                KEY_INDENTED_TOO_DEEP,
                "ko.frontmatter.key_indented_too_deep",
                key_span,
            )
            .with_argument("key", key_text),
        );
        slots.push(FmEntry {
            key: FmScalar {
                text: key_text.to_owned(),
                quoted: false,
                span: key_span,
            },
            value: FmValue::Rejected { span: key_span },
        });
    }
    slots
}

fn report_inconsistent_indent(line: &FmLine, diagnostics: &mut Vec<Diagnostic>) {
    diagnostics.push(Diagnostic::error(
        INCONSISTENT_INDENT,
        "ko.frontmatter.inconsistent_indent",
        line.span(),
    ));
}

/// 따옴표와 flow 괄호 밖의 첫 `:` 를 키 경계로 본다.
fn split_key(content: &str) -> Option<(&str, &str)> {
    let mut quoted = false;
    let mut escaped = false;
    let mut depth = 0usize;
    for (index, character) in content.char_indices() {
        if quoted {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                quoted = false;
            }
            continue;
        }
        match character {
            '"' => quoted = true,
            '[' | '{' => depth += 1,
            ']' | '}' => depth = depth.saturating_sub(1),
            ':' if depth == 0 => {
                let key = content[..index].trim_end();
                let value = content[index + 1..].trim_start();
                if key.is_empty() {
                    return None;
                }
                return Some((key, value));
            }
            _ => {}
        }
    }
    None
}

fn unsupported_key_feature(key: &str) -> Option<&'static str> {
    match key.chars().next() {
        Some('?') => Some("complex_key"),
        Some('&') => Some("anchor"),
        Some('*') => Some("alias"),
        Some('!') => Some("tag"),
        Some('\'') => Some("single_quoted"),
        _ => None,
    }
}

fn unsupported_value_feature(value: &str) -> Option<&'static str> {
    match value.chars().next() {
        Some('&') => Some("anchor"),
        Some('*') => Some("alias"),
        Some('!') => Some("tag"),
        Some('|') | Some('>') => Some("block_scalar"),
        Some('\'') => Some("single_quoted"),
        _ => None,
    }
}

fn parse_inline(text: &str, offset: usize, diagnostics: &mut Vec<Diagnostic>) -> FmValue {
    let span = Span {
        start: offset,
        end: offset + text.len(),
    };
    if let Some(feature) = unsupported_value_feature(text) {
        diagnostics.push(
            Diagnostic::error(
                UNSUPPORTED_YAML_FEATURE,
                "ko.frontmatter.unsupported_yaml_feature",
                span,
            )
            .with_argument("feature", feature),
        );
        return FmValue::Rejected { span };
    }

    match text.chars().next() {
        Some('[') => parse_flow_sequence(text, offset, diagnostics),
        Some('{') => parse_flow_mapping(text, offset, diagnostics),
        _ => FmValue::Scalar(read_scalar(text, span, diagnostics)),
    }
}

/// flow collection 안의 항목을 depth 를 세며 끊는다.
fn split_flow(body: &str) -> Vec<(usize, &str)> {
    let mut parts = Vec::new();
    let mut quoted = false;
    let mut escaped = false;
    let mut depth = 0usize;
    let mut start = 0usize;
    for (index, character) in body.char_indices() {
        if quoted {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                quoted = false;
            }
            continue;
        }
        match character {
            '"' => quoted = true,
            '[' | '{' => depth += 1,
            ']' | '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                parts.push((start, &body[start..index]));
                start = index + 1;
            }
            _ => {}
        }
    }
    if start <= body.len() {
        parts.push((start, &body[start..]));
    }
    parts
}

fn flow_body(text: &str) -> &str {
    let inner = &text[1..];
    inner
        .strip_suffix(']')
        .or_else(|| inner.strip_suffix('}'))
        .unwrap_or(inner)
}

fn parse_flow_sequence(text: &str, offset: usize, diagnostics: &mut Vec<Diagnostic>) -> FmValue {
    let span = Span {
        start: offset,
        end: offset + text.len(),
    };
    let body = flow_body(text);
    let mut items = Vec::new();
    for (index, part) in split_flow(body) {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        let item_offset = offset + 1 + index + leading_space(part);
        items.push(parse_inline(trimmed, item_offset, diagnostics));
    }
    FmValue::Sequence { items, span }
}

fn parse_flow_mapping(text: &str, offset: usize, diagnostics: &mut Vec<Diagnostic>) -> FmValue {
    let span = Span {
        start: offset,
        end: offset + text.len(),
    };
    let body = flow_body(text);
    let mut entries: Vec<FmEntry> = Vec::new();
    for (index, part) in split_flow(body) {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        let part_offset = offset + 1 + index + leading_space(part);
        let Some((key_text, value_text)) = split_key(trimmed) else {
            diagnostics.push(
                Diagnostic::error(
                    INVALID_STRUCTURE,
                    "ko.frontmatter.invalid_structure",
                    Span {
                        start: part_offset,
                        end: part_offset + trimmed.len(),
                    },
                )
                .with_argument("expected", "mapping_entry"),
            );
            continue;
        };
        let key_span = Span {
            start: part_offset,
            end: part_offset + key_text.len(),
        };
        if let Some(feature) = unsupported_key_feature(key_text) {
            diagnostics.push(
                Diagnostic::error(
                    UNSUPPORTED_YAML_FEATURE,
                    "ko.frontmatter.unsupported_yaml_feature",
                    key_span,
                )
                .with_argument("feature", feature),
            );
            entries.push(FmEntry {
                key: FmScalar {
                    text: key_text.to_owned(),
                    quoted: false,
                    span: key_span,
                },
                value: FmValue::Rejected { span: key_span },
            });
            continue;
        }
        let key = read_scalar(key_text, key_span, diagnostics);
        let value_offset = part_offset + trimmed.len() - value_text.len();
        let value = if value_text.is_empty() {
            FmValue::Empty { span: key.span }
        } else {
            parse_inline(value_text, value_offset, diagnostics)
        };
        let first_seen = entries
            .iter()
            .find(|entry| entry.key.text == key.text && !is_rejected(&entry.value))
            .map(|entry| entry.key.span.start);
        if let Some(first) = first_seen {
            diagnostics.push(
                Diagnostic::error(DUPLICATE_KEY, "ko.frontmatter.duplicate_key", key.span)
                    .with_argument("key", &key.text)
                    .with_argument("first", first),
            );
        } else {
            entries.push(FmEntry { key, value });
        }
    }
    FmValue::Mapping { entries, span }
}

fn leading_space(part: &str) -> usize {
    part.len() - part.trim_start().len()
}

fn read_scalar(text: &str, span: Span, diagnostics: &mut Vec<Diagnostic>) -> FmScalar {
    if text.starts_with('"') {
        return match serde_json::from_str::<String>(text) {
            Ok(value) => FmScalar {
                text: value,
                quoted: true,
                span,
            },
            Err(_) => {
                diagnostics.push(
                    Diagnostic::error(
                        INVALID_SCALAR_TYPE,
                        "ko.frontmatter.invalid_scalar_type",
                        span,
                    )
                    .with_argument("expected", "string"),
                );
                FmScalar {
                    text: String::new(),
                    quoted: true,
                    span,
                }
            }
        };
    }
    FmScalar {
        text: text.to_owned(),
        quoted: false,
        span,
    }
}

// ---------------------------------------------------------------------------
// 스키마 해석
//
// 일반 YAML 값 나무를 머리말 스키마로 옮긴다. 값의 타입은 추론하지 않고 **그 자리가 기대하는
// 타입으로만** 읽는다. YAML 1.1 의 `yes`/`no` 같은 함정을 피하는 유일한 방법이다.
// ---------------------------------------------------------------------------

const LAYOUT_HEADER: &str = "머리말";
const LAYOUT_SECTION: &str = "구역";
const LAYOUT_HEADING: &str = "제목";
const LAYOUT_FORM: &str = "폼";
const LAYOUT_INPUT: &str = "입력";
const LAYOUT_LIST: &str = "목록";
const LAYOUT_BUTTON: &str = "버튼";
const LAYOUT_PLACEHOLDER: &str = "자리";

fn structure_error(span: Span, expected: &str) -> Diagnostic {
    Diagnostic::error(INVALID_STRUCTURE, "ko.frontmatter.invalid_structure", span)
        .with_argument("expected", expected)
}

fn scalar_error(span: Span, expected: &str) -> Diagnostic {
    Diagnostic::error(
        INVALID_SCALAR_TYPE,
        "ko.frontmatter.invalid_scalar_type",
        span,
    )
    .with_argument("expected", expected)
}

fn unknown_key(scalar: &FmScalar) -> Diagnostic {
    Diagnostic::error(UNKNOWN_KEY, "ko.frontmatter.unknown_key", scalar.span)
        .with_argument("key", &scalar.text)
}

fn expect_mapping<'a>(
    value: &'a FmValue,
    expected: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> &'a [FmEntry] {
    match value {
        FmValue::Mapping { entries, .. } => entries,
        FmValue::Empty { .. } | FmValue::Rejected { .. } => &[],
        other => {
            diagnostics.push(structure_error(other.span(), expected));
            &[]
        }
    }
}

fn expect_sequence<'a>(
    value: &'a FmValue,
    expected: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> &'a [FmValue] {
    match value {
        FmValue::Sequence { items, .. } => items,
        FmValue::Empty { .. } | FmValue::Rejected { .. } => &[],
        other => {
            diagnostics.push(structure_error(other.span(), expected));
            &[]
        }
    }
}

fn expect_scalar<'a>(
    value: &'a FmValue,
    expected: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<&'a FmScalar> {
    match value {
        FmValue::Scalar(scalar) => Some(scalar),
        FmValue::Rejected { .. } => None,
        other => {
            diagnostics.push(structure_error(other.span(), expected));
            None
        }
    }
}

fn is_rejected(value: &FmValue) -> bool {
    matches!(value, FmValue::Rejected { .. })
}

/// 거절된 키가 섞인 매핑에서는 "무엇이 없다"고 말하지 않는다. 거절된 키가 원래 그 자리였을
/// 수도 있는데, 그걸 알 방법이 없으므로 없다고 단정하면 없는 문제를 지어내는 것이 된다.
fn has_rejected(entries: &[FmEntry]) -> bool {
    entries.iter().any(|entry| is_rejected(&entry.value))
}

fn find<'a>(entries: &'a [FmEntry], key: &str) -> Option<&'a FmValue> {
    entries
        .iter()
        .find(|entry| entry.key.text == key)
        .map(|entry| &entry.value)
}

/// `표시 이름(stable_id)` 한 쌍. 문장 문법의 선언과 같은 표기를 쓴다.
fn read_named_id(scalar: &FmScalar, diagnostics: &mut Vec<Diagnostic>) -> Option<NamedIdAst> {
    if scalar.quoted {
        diagnostics.push(scalar_error(scalar.span, "named_id"));
        return None;
    }
    let text = scalar.text.trim();
    let open = text.rfind('(');
    let close = text.strip_suffix(')').map(|rest| rest.len());
    match (open, close) {
        (Some(open), Some(close)) if open < close => {
            let name = text[..open].trim_end();
            let id = &text[open + 1..close];
            if name.is_empty() || !is_id_like(id) {
                diagnostics.push(scalar_error(scalar.span, "named_id"));
                return None;
            }
            Some(NamedIdAst {
                name: name.to_owned(),
                id: id.to_owned(),
                span: scalar.span,
            })
        }
        _ => {
            diagnostics.push(scalar_error(scalar.span, "named_id"));
            None
        }
    }
}

fn is_id_like(text: &str) -> bool {
    !text.is_empty()
        && !text
            .chars()
            .any(|character| character.is_whitespace() || character == '.')
}

/// 다른 선언을 가리키는 자리.
///
/// 문장이 `장바구니 항목의 수량을 입력할 수 있다`라고 쓰면 머리말도 `- 입력: 수량`이라고 쓴다.
/// 한 문서에 이름 짓는 방법이 둘이면 기획자는 머리말에서만 영어 식별자로 갈아타야 한다.
/// 띄어쓰기가 있는 표시 이름도 그대로 받고, 따옴표는 써도 되지만 요구하지 않는다.
///
/// 여기서는 풀지 않는다. 적힌 글자를 그대로 실어 보내고, 없는 이름과 모호한 이름은 문장의
/// 참조를 푸는 바로 그 해석기가 판정한다.
fn read_ref(scalar: &FmScalar, diagnostics: &mut Vec<Diagnostic>) -> Option<FrontmatterRefAst> {
    let text = scalar.text.trim();
    if text.is_empty() {
        diagnostics.push(scalar_error(scalar.span, "reference"));
        return None;
    }
    Some(FrontmatterRefAst {
        text: text.to_owned(),
        span: scalar.span,
    })
}

fn read_ref_value(value: &FmValue, diagnostics: &mut Vec<Diagnostic>) -> Option<FrontmatterRefAst> {
    let scalar = expect_scalar(value, "reference", diagnostics)?;
    read_ref(scalar, diagnostics)
}

/// 문서가 스스로 짓는 이름. 버튼의 `id` 가 여기 해당하며 참조가 아니다.
fn read_stable_id(scalar: &FmScalar, diagnostics: &mut Vec<Diagnostic>) -> Option<String> {
    if scalar.quoted || !is_id_like(&scalar.text) {
        diagnostics.push(scalar_error(scalar.span, "stable_id"));
        return None;
    }
    Some(scalar.text.clone())
}

fn read_text_value(value: &FmValue, diagnostics: &mut Vec<Diagnostic>) -> Option<String> {
    let scalar = expect_scalar(value, "text", diagnostics)?;
    Some(scalar.text.clone())
}

fn build_frontmatter(
    value: &FmValue,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) -> FrontmatterAst {
    let mut frontmatter = FrontmatterAst {
        module: None,
        information_architecture: Vec::new(),
        screens: Vec::new(),
        paths: Vec::new(),
        span,
    };

    for entry in expect_mapping(value, "frontmatter", diagnostics) {
        if is_rejected(&entry.value) {
            continue;
        }
        match entry.key.text.as_str() {
            KEY_MODULE => {
                if let Some(scalar) = expect_scalar(&entry.value, "named_id", diagnostics) {
                    frontmatter.module = read_named_id(scalar, diagnostics);
                }
            }
            KEY_INFORMATION_ARCHITECTURE => {
                frontmatter.information_architecture = build_categories(&entry.value, diagnostics);
            }
            KEY_SCREENS => {
                frontmatter.screens = build_screen_layouts(&entry.value, diagnostics);
            }
            KEY_PATHS => {
                frontmatter.paths = build_paths(&entry.value, diagnostics);
            }
            _ => diagnostics.push(unknown_key(&entry.key)),
        }
    }

    frontmatter
}

/// 분류 나무. 깊이는 여기서 강제하지 않는다 — 관례를 벗어났는지는 의미 규칙이 아니라
/// 이후 단계의 안내다.
fn build_categories(value: &FmValue, diagnostics: &mut Vec<Diagnostic>) -> Vec<CategoryAst> {
    let mut categories = Vec::new();
    for entry in expect_mapping(value, "category_mapping", diagnostics) {
        if is_rejected(&entry.value) {
            continue;
        }
        let Some(declaration) = read_named_id(&entry.key, diagnostics) else {
            continue;
        };
        let mut category = CategoryAst {
            declaration,
            children: Vec::new(),
            screens: Vec::new(),
            span: entry.key.span,
        };
        match &entry.value {
            // 잎에는 화면 목록이 오고, 가지에는 하위 분류가 온다.
            FmValue::Sequence { items, .. } => {
                for item in items {
                    if let Some(reference) = read_ref_value(item, diagnostics) {
                        category.screens.push(reference);
                    }
                }
            }
            FmValue::Mapping { .. } => {
                category.children = build_categories(&entry.value, diagnostics);
            }
            FmValue::Empty { .. } => {}
            other => diagnostics.push(structure_error(other.span(), "category_body")),
        }
        categories.push(category);
    }
    categories
}

fn build_screen_layouts(
    value: &FmValue,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<ScreenLayoutAst> {
    let mut layouts = Vec::new();
    for entry in expect_mapping(value, "screen_mapping", diagnostics) {
        if is_rejected(&entry.value) {
            continue;
        }
        let Some(screen) = read_ref(&entry.key, diagnostics) else {
            continue;
        };
        let mut layout = ScreenLayoutAst {
            screen,
            kind: None,
            elements: Vec::new(),
            span: entry.key.span,
        };
        for field in expect_mapping(&entry.value, "screen_body", diagnostics) {
            if is_rejected(&field.value) {
                continue;
            }
            match field.key.text.as_str() {
                "유형" => {
                    if let Some(scalar) = expect_scalar(&field.value, "screen_kind", diagnostics) {
                        layout.kind = read_screen_kind(scalar, diagnostics);
                    }
                }
                "레이아웃" => {
                    layout.elements = build_elements(&field.value, diagnostics);
                }
                _ => diagnostics.push(unknown_key(&field.key)),
            }
        }
        layouts.push(layout);
    }
    layouts
}

fn read_screen_kind(
    scalar: &FmScalar,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<ScreenLayoutKindAst> {
    match scalar.text.as_str() {
        "page" => Some(ScreenLayoutKindAst::Page),
        "popup" => Some(ScreenLayoutKindAst::Popup),
        "tab" => Some(ScreenLayoutKindAst::Tab),
        "link" => Some(ScreenLayoutKindAst::Link),
        _ => {
            diagnostics.push(scalar_error(scalar.span, "screen_kind"));
            None
        }
    }
}

fn build_elements(value: &FmValue, diagnostics: &mut Vec<Diagnostic>) -> Vec<LayoutElementAst> {
    let mut elements = Vec::new();
    for item in expect_sequence(value, "layout_sequence", diagnostics) {
        if let Some(element) = build_element(item, diagnostics) {
            elements.push(element);
        }
    }
    elements
}

fn build_element(value: &FmValue, diagnostics: &mut Vec<Diagnostic>) -> Option<LayoutElementAst> {
    let entries = expect_mapping(value, "layout_element", diagnostics);
    let [entry] = entries else {
        // 요소 하나는 어휘 하나다. 두 개가 붙어 있으면 어느 쪽이 요소인지 정할 수 없다.
        if !entries.is_empty() {
            diagnostics.push(structure_error(value.span(), "layout_element"));
        }
        return None;
    };

    let span = entry.key.span;
    match entry.key.text.as_str() {
        LAYOUT_HEADER => Some(LayoutElementAst::Header {
            children: build_elements(&entry.value, diagnostics),
            span,
        }),
        LAYOUT_SECTION => Some(LayoutElementAst::Section {
            children: build_elements(&entry.value, diagnostics),
            span,
        }),
        LAYOUT_HEADING => Some(LayoutElementAst::Heading {
            text: read_text_value(&entry.value, diagnostics)?,
            span,
        }),
        LAYOUT_PLACEHOLDER => Some(LayoutElementAst::Placeholder {
            text: read_text_value(&entry.value, diagnostics)?,
            span,
        }),
        LAYOUT_INPUT => Some(LayoutElementAst::Input {
            field: read_ref_value(&entry.value, diagnostics)?,
            span,
        }),
        LAYOUT_FORM => {
            let mut inputs = Vec::new();
            for child in build_elements(&entry.value, diagnostics) {
                match child {
                    LayoutElementAst::Input { .. } => inputs.push(child),
                    // 폼 안에는 입력만 온다. 다른 어휘를 담으면 폼이 무엇을 모으는 자리인지가
                    // 흐려진다.
                    other => diagnostics.push(structure_error(element_span(&other), "입력")),
                }
            }
            Some(LayoutElementAst::Form { inputs, span })
        }
        LAYOUT_LIST => {
            let fields = expect_mapping(&entry.value, "list_body", diagnostics);
            let mut model = None;
            let mut field_refs = Vec::new();
            for field in fields {
                if is_rejected(&field.value) {
                    continue;
                }
                match field.key.text.as_str() {
                    "모델" => model = read_ref_value(&field.value, diagnostics),
                    "필드" => {
                        for item in expect_sequence(&field.value, "field_sequence", diagnostics) {
                            if let Some(reference) = read_ref_value(item, diagnostics) {
                                field_refs.push(reference);
                            }
                        }
                    }
                    _ => diagnostics.push(unknown_key(&field.key)),
                }
            }
            let Some(model) = model else {
                // 값을 읽지 못한 것과 아예 없는 것은 다르다. 읽지 못한 자리는 이미
                // 자기 진단을 냈으므로 "없다"고 한 번 더 말하지 않는다.
                if find(fields, "모델").is_none() && !has_rejected(fields) {
                    diagnostics.push(structure_error(span, "모델"));
                }
                return None;
            };
            Some(LayoutElementAst::List {
                model,
                fields: field_refs,
                span,
            })
        }
        LAYOUT_BUTTON => {
            let fields = expect_mapping(&entry.value, "button_body", diagnostics);
            let mut id = None;
            let mut name = None;
            let mut action = None;
            for field in fields {
                if is_rejected(&field.value) {
                    continue;
                }
                match field.key.text.as_str() {
                    "id" => {
                        id = expect_scalar(&field.value, "stable_id", diagnostics)
                            .and_then(|scalar| read_stable_id(scalar, diagnostics));
                    }
                    "이름" => name = read_text_value(&field.value, diagnostics),
                    "행동" => action = read_ref_value(&field.value, diagnostics),
                    _ => diagnostics.push(unknown_key(&field.key)),
                }
            }
            let Some(id) = id else {
                // 값을 읽지 못한 것과 아예 없는 것은 다르다. 읽지 못한 자리는 이미
                // 자기 진단을 냈으므로 "없다"고 한 번 더 말하지 않는다.
                if find(fields, "id").is_none() && !has_rejected(fields) {
                    diagnostics.push(structure_error(span, "id"));
                }
                return None;
            };
            let Some(name) = name else {
                // 값을 읽지 못한 것과 아예 없는 것은 다르다. 읽지 못한 자리는 이미
                // 자기 진단을 냈으므로 "없다"고 한 번 더 말하지 않는다.
                if find(fields, "이름").is_none() && !has_rejected(fields) {
                    diagnostics.push(structure_error(span, "이름"));
                }
                return None;
            };
            Some(LayoutElementAst::Button {
                id,
                name,
                action,
                span,
            })
        }
        _ => {
            diagnostics.push(
                Diagnostic::error(
                    UNKNOWN_LAYOUT_ELEMENT,
                    "ko.frontmatter.unknown_layout_element",
                    span,
                )
                .with_argument("element", &entry.key.text),
            );
            None
        }
    }
}

fn element_span(element: &LayoutElementAst) -> Span {
    match element {
        LayoutElementAst::Header { span, .. }
        | LayoutElementAst::Section { span, .. }
        | LayoutElementAst::Heading { span, .. }
        | LayoutElementAst::Form { span, .. }
        | LayoutElementAst::Input { span, .. }
        | LayoutElementAst::List { span, .. }
        | LayoutElementAst::Button { span, .. }
        | LayoutElementAst::Placeholder { span, .. } => *span,
    }
}

fn build_paths(value: &FmValue, diagnostics: &mut Vec<Diagnostic>) -> Vec<ScreenPathAst> {
    let mut paths = Vec::new();
    for item in expect_sequence(value, "path_sequence", diagnostics) {
        let entries = expect_mapping(item, "path_entry", diagnostics);
        if entries.is_empty() {
            continue;
        }
        for entry in entries {
            if is_rejected(&entry.value) {
                continue;
            }
            if !matches!(entry.key.text.as_str(), "출발" | "도착" | "설명") {
                diagnostics.push(unknown_key(&entry.key));
            }
        }

        let span = item.span();
        let Some(source) = find(entries, "출발") else {
            if !has_rejected(entries) {
                diagnostics.push(structure_error(span, "출발"));
            }
            continue;
        };
        let Some(target) = find(entries, "도착") else {
            if !has_rejected(entries) {
                diagnostics.push(structure_error(span, "도착"));
            }
            continue;
        };
        let Some((source_screen, source_element)) = read_element_path(source, diagnostics) else {
            continue;
        };
        let Some(target_screen) = read_ref_value(target, diagnostics) else {
            continue;
        };
        let label = find(entries, "설명").and_then(|value| read_text_value(value, diagnostics));

        paths.push(ScreenPathAst {
            source_screen,
            source_element,
            target_screen,
            label,
            span,
        });
    }
    paths
}

/// `화면ID.요소ID`. 출발은 화면이 아니라 화면 안의 요소다.
fn read_element_path(
    value: &FmValue,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<(FrontmatterRefAst, FrontmatterRefAst)> {
    let scalar = expect_scalar(value, "element_path", diagnostics)?;
    // 마지막 점에서 끊는다. 요소 id 는 문서가 지은 stable id 라 점을 담을 수 없지만, 화면
    // 이름은 한국어 표시 이름이라 점이 들어갈 수 있다. 첫 점에서 끊으면 그런 화면 이름이
    // 조용히 반토막 난다.
    let Some((screen, element)) = scalar.text.rsplit_once('.') else {
        diagnostics.push(scalar_error(scalar.span, "element_path"));
        return None;
    };
    let screen = screen.trim();
    if screen.is_empty() || !is_id_like(element) {
        diagnostics.push(scalar_error(scalar.span, "element_path"));
        return None;
    }
    let screen_span = Span {
        start: scalar.span.start,
        end: scalar.span.start + screen.len(),
    };
    let element_span = Span {
        start: screen_span.end + 1,
        end: scalar.span.end,
    };
    Some((
        FrontmatterRefAst {
            text: screen.to_owned(),
            span: screen_span,
        },
        FrontmatterRefAst {
            text: element.to_owned(),
            span: element_span,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"---
모듈: 장바구니(shopping)

정보구조:
  주문(order):
    결제(payment): [create_cart, create_item]
    조회(inquiry): [cart_detail]

화면:
  create_item:
    유형: page
    레이아웃:
      - 머리말:
          - 제목: "항목 추가"
      - 구역:
          - 폼:
              - 입력: 수량
              - 입력: 금액
          - 버튼: { id: submit, 이름: "담기", 행동: 항목_담기 }
          - 자리: "배송지 지도"

흐름:
  - 출발: create_item.submit
    도착: cart_detail
    설명: "담기 성공"
---

장바구니 항목 입력 화면(create_item)에서는 장바구니 항목을 생성할 수 있다.
"#;

    fn keys(diagnostics: &[Diagnostic]) -> Vec<&str> {
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.message_key.as_str())
            .collect()
    }

    #[test]
    fn reads_the_reference_document() {
        let output = read(SAMPLE);
        assert_eq!(keys(&output.diagnostics), Vec::<&str>::new());
        let frontmatter = output.frontmatter.expect("frontmatter");

        let module = frontmatter.module.expect("module");
        assert_eq!(module.name, "장바구니");
        assert_eq!(module.id, "shopping");

        // 분류는 한 갈래가 두 하위 분류를 가지고, 잎에 화면이 달린다.
        assert_eq!(frontmatter.information_architecture.len(), 1);
        let order = &frontmatter.information_architecture[0];
        assert_eq!(order.declaration.id, "order");
        assert_eq!(order.children.len(), 2);
        assert_eq!(order.children[0].declaration.id, "payment");
        let screens: Vec<&str> = order.children[0]
            .screens
            .iter()
            .map(|reference| reference.text.as_str())
            .collect();
        assert_eq!(screens, vec!["create_cart", "create_item"]);

        assert_eq!(frontmatter.screens.len(), 1);
        let layout = &frontmatter.screens[0];
        assert_eq!(layout.screen.text, "create_item");
        assert_eq!(layout.kind, Some(ScreenLayoutKindAst::Page));
        assert_eq!(layout.elements.len(), 2);

        let LayoutElementAst::Header { children, .. } = &layout.elements[0] else {
            panic!("첫 요소는 머리말이어야 한다: {:?}", layout.elements[0]);
        };
        assert!(matches!(
            children.as_slice(),
            [LayoutElementAst::Heading { text, .. }] if text == "항목 추가"
        ));

        let LayoutElementAst::Section { children, .. } = &layout.elements[1] else {
            panic!("둘째 요소는 구역이어야 한다");
        };
        assert_eq!(children.len(), 3);
        let LayoutElementAst::Form { inputs, .. } = &children[0] else {
            panic!("구역의 첫 자식은 폼이어야 한다");
        };
        assert_eq!(inputs.len(), 2);
        assert!(matches!(
            &children[1],
            LayoutElementAst::Button { id, name, action: Some(action), .. }
                if id == "submit" && name == "담기" && action.text == "항목_담기"
        ));
        // 선언할 수 없는 자리는 이름표만 남는다.
        assert!(matches!(
            &children[2],
            LayoutElementAst::Placeholder { text, .. } if text == "배송지 지도"
        ));

        assert_eq!(frontmatter.paths.len(), 1);
        let path = &frontmatter.paths[0];
        assert_eq!(path.source_screen.text, "create_item");
        assert_eq!(path.source_element.text, "submit");
        assert_eq!(path.target_screen.text, "cart_detail");
        assert_eq!(path.label.as_deref(), Some("담기 성공"));
    }

    #[test]
    fn document_without_frontmatter_is_untouched() {
        let source = "@모듈 재고(inventory)\n";
        let output = read(source);
        assert!(output.frontmatter.is_none());
        assert!(output.diagnostics.is_empty());
        assert!(matches!(output.source, Cow::Borrowed(_)));
    }

    #[test]
    fn masking_preserves_byte_length_and_newlines() {
        let output = read(SAMPLE);
        assert_eq!(output.source.len(), SAMPLE.len());
        assert_eq!(
            output.source.matches('\n').count(),
            SAMPLE.matches('\n').count()
        );
        // 머리말 구간은 전부 지워지고 본문 문장만 남는다.
        assert!(
            output
                .source
                .contains("장바구니 항목 입력 화면(create_item)")
        );
        assert!(!output.source.contains("정보구조"));
    }

    #[test]
    fn spans_point_at_the_original_file() {
        let source = "---\n모듈: 장바구니(shopping)\n없는키: 값\n---\n";
        let output = read(source);
        assert_eq!(
            keys(&output.diagnostics),
            vec!["ko.frontmatter.unknown_key"]
        );
        let span = output.diagnostics[0].span;
        // 머리말을 잘라 내지 않았으므로 span 이 파일 기준으로 그대로 맞아야 한다.
        assert_eq!(span.start, source.find("없는키").expect("key offset"));
        assert_eq!(&source[span.start..span.end], "없는키");
    }

    #[test]
    fn unterminated_block_is_reported() {
        let output = read("---\n모듈: 가(a)\n");
        assert_eq!(
            keys(&output.diagnostics),
            vec!["ko.frontmatter.unterminated_block"]
        );
        // 어디까지가 머리말인지 모르므로 원문을 건드리지 않는다.
        assert!(matches!(output.source, Cow::Borrowed(_)));
    }

    // 한 실수는 진단 하나다. 아래 묶음은 전부 "거절한 자리를 다시 구조 오류로 또 말하지
    // 않는다"를 고정한다 — 두 번째 진단은 없는 문제를 찾아가게 만드는 오탐이다.

    #[test]
    fn tab_indentation_is_rejected() {
        let output = read("---\n정보구조:\n\t주문(order): [s1]\n---\n");
        assert_eq!(
            keys(&output.diagnostics),
            vec!["ko.frontmatter.tab_indentation"]
        );
    }

    #[test]
    fn aliases_are_rejected() {
        let output = read("---\n모듈: *기준\n---\n");
        assert_eq!(
            keys(&output.diagnostics),
            vec!["ko.frontmatter.unsupported_yaml_feature"]
        );
        assert_eq!(output.diagnostics[0].argument("feature"), Some("alias"));
    }

    #[test]
    fn tags_are_rejected() {
        let output = read("---\n모듈: !!str 가\n---\n");
        assert_eq!(
            keys(&output.diagnostics),
            vec!["ko.frontmatter.unsupported_yaml_feature"]
        );
        assert_eq!(output.diagnostics[0].argument("feature"), Some("tag"));
    }

    #[test]
    fn single_quoted_strings_are_rejected() {
        let output = read("---\n모듈: '가(a)'\n---\n");
        assert_eq!(
            keys(&output.diagnostics),
            vec!["ko.frontmatter.unsupported_yaml_feature"]
        );
        assert_eq!(
            output.diagnostics[0].argument("feature"),
            Some("single_quoted")
        );
    }

    #[test]
    fn document_end_marker_is_rejected() {
        let output = read("---\n모듈: 가(a)\n...\n---\n");
        assert_eq!(
            keys(&output.diagnostics),
            vec!["ko.frontmatter.unsupported_yaml_feature"]
        );
        assert_eq!(
            output.diagnostics[0].argument("feature"),
            Some("document_end")
        );
    }

    #[test]
    fn rejected_key_is_not_also_reported_as_missing() {
        // 거절된 키가 원래 `출발` 이었을 수도 있다. 없다고 단정하지 않는다.
        let output = read("---\n흐름:\n  - &기준: s1.go\n    도착: s2\n---\n");
        assert_eq!(
            keys(&output.diagnostics),
            vec!["ko.frontmatter.unsupported_yaml_feature"]
        );
    }

    #[test]
    fn rejected_button_field_is_not_also_reported_as_unknown() {
        let output = read(
            "---\n화면:\n  s1:\n    레이아웃:\n      - 버튼: { id: go, 이름: \"가기\", 행동: *기준 }\n---\n",
        );
        assert_eq!(
            keys(&output.diagnostics),
            vec!["ko.frontmatter.unsupported_yaml_feature"]
        );
    }

    #[test]
    fn references_are_written_the_way_sentences_write_them() {
        // 머리말 전체가 한국어 표시 이름이다. 따옴표 없이, 띄어쓰기를 담은 채로.
        let source = concat!(
            "---\n",
            "모듈: 장바구니(shopping)\n",
            "정보구조:\n",
            "  주문(order): [장바구니 항목 입력 화면, 장바구니 상세 화면]\n",
            "화면:\n",
            "  장바구니 항목 입력 화면:\n",
            "    레이아웃:\n",
            "      - 폼:\n",
            "          - 입력: 수량\n",
            "      - 버튼: { id: submit, 이름: \"담기\", 행동: 항목 담기 }\n",
            "흐름:\n",
            "  - 출발: 장바구니 항목 입력 화면.submit\n",
            "    도착: 장바구니 상세 화면\n",
            "---\n"
        );
        let output = read(source);
        assert_eq!(keys(&output.diagnostics), Vec::<&str>::new());
        let frontmatter = output.frontmatter.expect("frontmatter");

        let screens: Vec<&str> = frontmatter.information_architecture[0]
            .screens
            .iter()
            .map(|reference| reference.text.as_str())
            .collect();
        assert_eq!(
            screens,
            vec!["장바구니 항목 입력 화면", "장바구니 상세 화면"]
        );
        assert_eq!(
            frontmatter.screens[0].screen.text,
            "장바구니 항목 입력 화면"
        );

        let path = &frontmatter.paths[0];
        assert_eq!(path.source_screen.text, "장바구니 항목 입력 화면");
        assert_eq!(path.source_element.text, "submit");
        // 따옴표 없이 띄어쓴 이름이 통째로 도착으로 실려야 한다.
        assert_eq!(path.target_screen.text, "장바구니 상세 화면");
    }

    #[test]
    fn element_path_splits_at_the_last_dot() {
        let output = read("---\n흐름:\n  - 출발: 버전 1.2 화면.submit\n    도착: 끝 화면\n---\n");
        assert_eq!(keys(&output.diagnostics), Vec::<&str>::new());
        let paths = output.frontmatter.expect("frontmatter").paths;
        // 화면 이름 안의 점은 이름의 일부다. 첫 점에서 끊으면 이름이 반토막 난다.
        assert_eq!(paths[0].source_screen.text, "버전 1.2 화면");
        assert_eq!(paths[0].source_element.text, "submit");
    }

    #[test]
    fn button_id_is_coined_not_referenced() {
        // 버튼의 id 는 문서가 스스로 짓는 이름이므로 표시 이름 규칙을 따르지 않는다.
        let output = read(
            "---\n화면:\n  s1:\n    레이아웃:\n      - 버튼: { id: 보내기 버튼, 이름: \"보내기\" }\n---\n",
        );
        assert_eq!(
            keys(&output.diagnostics),
            vec!["ko.frontmatter.invalid_scalar_type"]
        );
        assert_eq!(
            output.diagnostics[0].argument("expected"),
            Some("stable_id")
        );
    }

    #[test]
    fn inconsistent_indent_is_reported_once() {
        let output = read("---\n화면:\n  s1:\n      유형: page\n    레이아웃:\n---\n");
        assert_eq!(
            keys(&output.diagnostics),
            vec!["ko.frontmatter.inconsistent_indent"]
        );
    }

    #[test]
    fn duplicate_key_is_reported() {
        let output = read("---\n모듈: 가(a)\n모듈: 나(b)\n---\n");
        assert_eq!(
            keys(&output.diagnostics),
            vec!["ko.frontmatter.duplicate_key"]
        );
        // 먼저 쓴 쪽이 남는다.
        let module = output
            .frontmatter
            .expect("frontmatter")
            .module
            .expect("module");
        assert_eq!(module.id, "a");
    }

    #[test]
    fn anchors_are_rejected() {
        let output = read("---\n모듈: &기준 가(a)\n---\n");
        assert_eq!(
            keys(&output.diagnostics),
            vec!["ko.frontmatter.unsupported_yaml_feature"]
        );
        assert_eq!(output.diagnostics[0].argument("feature"), Some("anchor"));
    }

    #[test]
    fn block_scalars_are_rejected() {
        let output = read("---\n화면:\n  s1:\n    레이아웃: |\n      글\n---\n");
        assert!(
            keys(&output.diagnostics).contains(&"ko.frontmatter.unsupported_yaml_feature"),
            "{:?}",
            keys(&output.diagnostics)
        );
    }

    #[test]
    fn screen_kind_comes_from_a_closed_set() {
        let output = read("---\n화면:\n  s1:\n    유형: 팝업\n---\n");
        assert_eq!(
            keys(&output.diagnostics),
            vec!["ko.frontmatter.invalid_scalar_type"]
        );
        assert_eq!(
            output.diagnostics[0].argument("expected"),
            Some("screen_kind")
        );
    }

    #[test]
    fn unknown_layout_element_is_reported() {
        let output = read("---\n화면:\n  s1:\n    레이아웃:\n      - 지도: \"여기\"\n---\n");
        assert_eq!(
            keys(&output.diagnostics),
            vec!["ko.frontmatter.unknown_layout_element"]
        );
        assert_eq!(output.diagnostics[0].argument("element"), Some("지도"));
    }

    #[test]
    fn scalars_are_never_type_inferred() {
        // `page` 가 아닌 자리에서 `no` 는 그냥 글자다. YAML 1.1 처럼 거짓으로 읽지 않는다.
        let output = read("---\n화면:\n  s1:\n    레이아웃:\n      - 제목: no\n---\n");
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
        let frontmatter = output.frontmatter.expect("frontmatter");
        assert!(matches!(
            &frontmatter.screens[0].elements[0],
            LayoutElementAst::Heading { text, .. } if text == "no"
        ));
    }

    #[test]
    fn path_needs_both_endpoints() {
        let output = read("---\n흐름:\n  - 출발: s1.go\n---\n");
        assert_eq!(
            keys(&output.diagnostics),
            vec!["ko.frontmatter.invalid_structure"]
        );
        assert_eq!(output.diagnostics[0].argument("expected"), Some("도착"));
    }

    #[test]
    fn path_source_must_name_an_element() {
        let output = read("---\n흐름:\n  - 출발: s1\n    도착: s2\n---\n");
        assert_eq!(
            keys(&output.diagnostics),
            vec!["ko.frontmatter.invalid_scalar_type"]
        );
        assert_eq!(
            output.diagnostics[0].argument("expected"),
            Some("element_path")
        );
    }
}
