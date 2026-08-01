use serde::Serialize;

use crate::ast::*;
use crate::scanner::{Token, TokenKind, scan};
use crate::{Diagnostic, Severity};

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
            "문서는 모듈 선언으로 시작해야 합니다.",
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
                "최상위 선언이 아닌 위치에 들여쓴 항목이 있습니다.",
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
            Some(DeclarationKind::Constraint) => {
                parse_constraint(line, body, &mut diagnostics).map(DeclarationAst::Constraint)
            }
            Some(DeclarationKind::Role) => parse_role(line, &mut diagnostics)
                .map(|declaration| DeclarationAst::Role(RoleAst { declaration })),
            Some(DeclarationKind::Action) => parse_action(line, &mut diagnostics)
                .map(|declaration| DeclarationAst::Action(ActionAst { declaration })),
            Some(DeclarationKind::Policy) => {
                parse_policy(line, body, &mut diagnostics).map(DeclarationAst::Policy)
            }
            _ => Err(Diagnostic::error(
                "RSPDL-KO-SYN-003",
                "알 수 없는 최상위 선언입니다.",
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
    Constraint,
    Role,
    Action,
    Policy,
}

fn declaration_kind(line: &Line) -> Option<DeclarationKind> {
    match word_at(line, 0) {
        Some("@열거형") => Some(DeclarationKind::Enum),
        Some("@역할") => Some(DeclarationKind::Role),
        Some("@행동") => Some(DeclarationKind::Action),
        _ if is_data_model_header(line) => Some(DeclarationKind::DataModel),
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
            "문서는 `@모듈 표시 이름(stable_id)` 선언으로 시작해야 합니다.",
            line.span,
        ));
    }
    let declaration = parse_annotated_name(line, "@모듈")?;
    Ok(ModuleAst { declaration })
}

fn parse_enum(
    line: &Line,
    body: &[Line],
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<EnumAst, Diagnostic> {
    let declaration = parse_natural_block_header(
        line,
        Some("@열거형"),
        &["다음", "값", "중", "하나다"],
        diagnostics,
    )?;
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
                "선언 항목에는 마침표를 사용하지 않습니다.",
                item.span,
            ));
        }
        let id_index = item.tokens.len().checked_sub(1).ok_or_else(|| {
            Diagnostic::error("RSPDL-KO-SYN-005", "열거형 값이 필요합니다.", item.span)
        })?;
        let declaration = parse_cfg_item_name(item, id_index)?;
        values.push(EnumValueAst { declaration });
    }
    Ok(EnumAst {
        declaration,
        values,
    })
}

fn parse_model(
    line: &Line,
    body: &[Line],
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<DataModelAst, Diagnostic> {
    let declaration = parse_natural_block_header(
        line,
        None,
        &["다음", "필드들로", "구성되어", "있다"],
        diagnostics,
    )?;
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
                    "필드 표시 이름 뒤에 `:`이 필요합니다.",
                    item.span,
                )
            })?;
        let id_index = colon_index.checked_sub(1).ok_or_else(|| {
            Diagnostic::error(
                "RSPDL-KO-SYN-010",
                "필드는 `표시 이름(local_id): 필수|선택 타입` 형식이어야 합니다.",
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
                "선언 항목에는 마침표를 사용하지 않습니다.",
                item.span,
            ));
        }
        let required = match word_at(item, colon_index + 1) {
            Some("필수") => true,
            Some("선택") => false,
            _ => {
                return Err(Diagnostic::error(
                    "RSPDL-KO-SYN-010",
                    "필드는 `필수` 또는 `선택`을 선언해야 합니다.",
                    item.span,
                ));
            }
        };
        let value_type = parse_type_reference(item, colon_index + 2)?;
        fields.push(FieldAst {
            declaration: field,
            required,
            value_type,
        });
    }
    Ok(DataModelAst {
        declaration,
        fields,
    })
}

fn parse_constraint(
    line: &Line,
    body: &[Line],
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<ConstraintAst, Diagnostic> {
    if !body.is_empty() {
        return Err(Diagnostic::error(
            "RSPDL-KO-SYN-020",
            "제약 문장 아래에는 별도 블록을 둘 수 없습니다.",
            line.span,
        ));
    }
    let expression = parse_constraint_sentence(line, diagnostics)?;
    let declaration = internal_constraint_declaration(&expression, line.span);
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
            "정책 문장 아래에는 별도 블록을 둘 수 없습니다.",
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
        _ => return Err(cursor.error("정책은 `수 있다` 또는 `수 없다`로 끝나야 합니다.")),
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
    let declaration =
        internal_policy_declaration(&role, &model, &field, &action, effect, body_line.span);
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
            _ => return Err(cursor.error("필드 비교는 `같아야` 또는 `달라야`를 사용해야 합니다.")),
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

fn internal_constraint_declaration(expression: &ConstraintExpressionAst, span: Span) -> NamedIdAst {
    let identity = format!(
        "{}\u{0}{}\u{0}{}\u{0}{}",
        expression.model,
        operand_identity(&expression.left),
        operator_identity(expression.operator),
        operand_identity(&expression.right)
    );
    internal_declaration("constraint", &identity, span)
}

fn internal_policy_declaration(
    role: &str,
    model: &str,
    field: &str,
    action: &str,
    effect: PolicyEffectAst,
    span: Span,
) -> NamedIdAst {
    let effect = match effect {
        PolicyEffectAst::Allow => "allow",
        PolicyEffectAst::Deny => "deny",
    };
    let identity = format!("{role}\u{0}{model}\u{0}{field}\u{0}{action}\u{0}{effect}");
    internal_declaration("policy", &identity, span)
}

fn internal_declaration(kind: &str, identity: &str, span: Span) -> NamedIdAst {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in identity.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    NamedIdAst {
        name: String::new(),
        id: format!("{kind}_{hash:016x}"),
        span,
    }
}

fn operand_identity(operand: &OperandAst) -> String {
    match operand {
        OperandAst::Field(value) => format!("field:{value}"),
        OperandAst::Literal(LiteralAst::String(value)) => {
            format!(
                "string:{}",
                serde_json::to_string(value).expect("a string always serializes")
            )
        }
        OperandAst::Literal(LiteralAst::Integer(value)) => format!("integer:{value}"),
        OperandAst::Literal(LiteralAst::Boolean(value)) => format!("boolean:{value}"),
        OperandAst::Literal(LiteralAst::Named(value)) => format!("named:{value}"),
    }
}

fn operator_identity(operator: RelationOperatorAst) -> &'static str {
    match operator {
        RelationOperatorAst::Equal => "equal",
        RelationOperatorAst::NotEqual => "not_equal",
        RelationOperatorAst::LessThan => "less_than",
        RelationOperatorAst::LessThanOrEqual => "less_than_or_equal",
        RelationOperatorAst::GreaterThan => "greater_than",
        RelationOperatorAst::GreaterThanOrEqual => "greater_than_or_equal",
    }
}

fn sentence_tokens(line: &Line) -> Result<&[Token], Diagnostic> {
    if !matches!(
        line.tokens.last().map(|token| &token.kind),
        Some(TokenKind::Period)
    ) {
        return Err(Diagnostic::error(
            "RSPDL-KO-SYN-040",
            "제약과 정책 문장은 마침표로 끝나야 합니다.",
            line.span,
        ));
    }
    Ok(&line.tokens[..line.tokens.len() - 1])
}

fn parse_role(line: &Line, _diagnostics: &mut Vec<Diagnostic>) -> Result<NamedIdAst, Diagnostic> {
    parse_annotated_name(line, "@역할")
}

fn parse_action(line: &Line, _diagnostics: &mut Vec<Diagnostic>) -> Result<NamedIdAst, Diagnostic> {
    parse_annotated_name(line, "@행동")
}

fn parse_annotated_name(line: &Line, keyword: &str) -> Result<NamedIdAst, Diagnostic> {
    if word_at(line, 0) != Some(keyword) {
        return Err(Diagnostic::error(
            "RSPDL-KO-SYN-004",
            format!("`{keyword} 표시 이름(stable_id)` 선언이 필요합니다."),
            line.span,
        ));
    }
    if line
        .tokens
        .iter()
        .any(|token| matches!(token.kind, TokenKind::Period | TokenKind::Colon))
    {
        return Err(Diagnostic::error(
            "RSPDL-KO-SYN-004",
            "선언 줄에는 마침표나 콜론을 사용하지 않습니다.",
            line.span,
        ));
    }
    let id_index =
        line.tokens.len().checked_sub(1).ok_or_else(|| {
            Diagnostic::error("RSPDL-KO-SYN-006", "선언 ID가 필요합니다.", line.span)
        })?;
    parse_name_with_id(line, 1, id_index)
}

fn parse_natural_block_header(
    line: &Line,
    keyword: Option<&str>,
    predicate: &[&str],
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<NamedIdAst, Diagnostic> {
    let name_start = if let Some(keyword) = keyword {
        if word_at(line, 0) != Some(keyword) {
            return Err(Diagnostic::error(
                "RSPDL-KO-SYN-004",
                format!("`{keyword} 표시 이름(local_id)` 선언이 필요합니다."),
                line.span,
            ));
        }
        1
    } else {
        0
    };
    if !matches!(
        line.tokens.last().map(|token| &token.kind),
        Some(TokenKind::Period)
    ) {
        return Err(Diagnostic::error(
            "RSPDL-KO-SYN-004",
            "데이터와 열거형 header는 마침표로 끝나는 문장이어야 합니다.",
            line.span,
        ));
    }
    let id_index = line
        .tokens
        .iter()
        .position(|token| matches!(token.kind, TokenKind::CanonicalId(_)))
        .ok_or_else(|| Diagnostic::error("RSPDL-KO-SYN-006", "선언 ID가 필요합니다.", line.span))?;
    let declaration = parse_name_with_id(line, name_start, id_index)?;
    let sentence = &line.tokens[id_index + 1..line.tokens.len() - 1];
    let mut cursor = BodyCursor::new(sentence, line.span);
    let marker = cursor
        .next_word()
        .filter(|marker| matches!(*marker, "은" | "는"))
        .ok_or_else(|| cursor.error("선언 이름 뒤에 `은` 또는 `는`이 필요합니다."))?;
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

fn canonical_id_at(line: &Line, index: usize) -> Result<String, Diagnostic> {
    match line.tokens.get(index).map(|token| &token.kind) {
        Some(TokenKind::CanonicalId(value)) if !value.is_empty() => Ok(value.clone()),
        _ => Err(Diagnostic::error(
            "RSPDL-KO-SYN-006",
            "선언에 `(stable_id)`가 필요합니다.",
            line.span,
        )),
    }
}

fn surface_name(line: &Line, start: usize, end: usize) -> Result<(String, Span), Diagnostic> {
    if start >= end {
        return Err(Diagnostic::error(
            "RSPDL-KO-SYN-005",
            "표시 이름이 필요합니다.",
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
                    "표시 이름 형식이 올바르지 않습니다.",
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
            "블록에는 하나 이상의 항목이 필요합니다.",
            span,
        ))
    } else {
        Ok(())
    }
}

fn bad_block_indent(line: &Line) -> Diagnostic {
    Diagnostic::error(
        "RSPDL-KO-SYN-009",
        "블록 항목의 들여쓰기 깊이가 일정하지 않습니다.",
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
            "필드 타입이 필요합니다.",
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
                    "필드 타입 형식이 올바르지 않습니다.",
                    token.span,
                ));
            }
        }
    }
    let name = parts.join(" ");
    match name.as_str() {
        "문자열" => Ok(TypeReferenceAst::String),
        "정수" => Ok(TypeReferenceAst::Integer),
        "불리언" => Ok(TypeReferenceAst::Boolean),
        "" => Err(Diagnostic::error(
            "RSPDL-KO-SYN-011",
            "필드 타입이 필요합니다.",
            line.span,
        )),
        _ => Ok(TypeReferenceAst::Named(name)),
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

    fn marked_ref(&mut self, markers: &[&str]) -> Result<(String, String), Diagnostic> {
        let token = self
            .tokens
            .get(self.index)
            .ok_or_else(|| self.error("문장에 필요한 이름과 조사가 누락되었습니다."))?;
        match &token.kind {
            TokenKind::QuotedIdentifier(value) => {
                self.index += 1;
                let marker = self
                    .next_word()
                    .ok_or_else(|| self.error("인용된 이름 뒤에 구조 marker가 필요합니다."))?;
                if markers.contains(&marker) {
                    Ok((value.clone(), marker.to_owned()))
                } else {
                    Err(self.error(format!(
                        "`{}` 뒤에는 {} 중 하나가 필요합니다.",
                        value,
                        markers.join("/")
                    )))
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
                Err(self.error(format!(
                    "`{}`에서 {} marker를 찾을 수 없습니다.",
                    parts.join(" "),
                    markers.join("/")
                )))
            }
            _ => Err(self.error("표면 이름이 필요합니다.")),
        }
    }

    fn comparison_literal(&mut self) -> Result<(RelationOperatorAst, LiteralAst), Diagnostic> {
        let token = self
            .tokens
            .get(self.index)
            .ok_or_else(|| self.error("제약의 비교 값이 누락되었습니다."))?;
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
                    return Err(self.error("`과/와 달라야 한다` 문형이 필요합니다."));
                }
            }
            if let Some(number) = value.strip_suffix("보다") {
                if is_integer(number) {
                    self.index += 1;
                    let operator = match self.next_word() {
                        Some("커야") => RelationOperatorAst::GreaterThan,
                        Some("작아야") => RelationOperatorAst::LessThan,
                        _ => {
                            return Err(
                                self.error("`<정수>보다 커야/작아야 한다` 문형이 필요합니다.")
                            );
                        }
                    };
                    return Ok((operator, LiteralAst::Integer(number.to_owned())));
                }
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
                        _ => return Err(self.error("`보다 커야/작아야`가 필요합니다.")),
                    },
                    Some("이어야") => RelationOperatorAst::Equal,
                    _ => return Err(self.error("지원하지 않는 정수 비교 문형입니다.")),
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
            _ => return Err(self.error("지원하지 않는 literal입니다.")),
        };
        self.index += 1;
        self.expect_word("이어야")?;
        Ok((RelationOperatorAst::Equal, literal))
    }

    fn expect_word(&mut self, expected: &str) -> Result<(), Diagnostic> {
        match self.next_word() {
            Some(actual) if actual == expected => Ok(()),
            _ => Err(self.error(format!("`{expected}`가 필요합니다."))),
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

    fn expect_end(&self) -> Result<(), Diagnostic> {
        if self.index == self.tokens.len() {
            Ok(())
        } else {
            Err(self.error("문장 뒤에 예상하지 못한 표현이 있습니다."))
        }
    }

    fn error(&self, message: impl Into<String>) -> Diagnostic {
        let span = self
            .tokens
            .get(self.index)
            .map(|token| token.span)
            .unwrap_or(self.span);
        Diagnostic::error("RSPDL-KO-SYN-041", message, span)
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
    let expected = if (last as u32 - '가' as u32) % 28 == 0 {
        vowel
    } else {
        consonant
    };
    if actual != expected {
        diagnostics.push(Diagnostic {
            rule_id: "RSPDL-KO-W001".into(),
            severity: Severity::Warning,
            message: format!("`{name}{actual}`보다 `{name}{expected}`이 자연스럽습니다."),
            span,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str = r#"@모듈 비용 승인(expense)

@열거형 비용 상태(status)는 다음 값 중 하나다.
    작성 중(draft)
    승인됨(approved)

비용 신청(request)은 다음 필드들로 구성되어 있다.
    식별자(id): 필수 문자열
    금액(amount): 필수 정수
    상태(status): 필수 비용 상태

비용 신청의 금액은 0보다 커야 한다.

@역할 회계 관리자(accounting_manager)
@행동 변경(change)

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
        assert!(constraint.declaration.id.starts_with("constraint_"));
        let DeclarationAst::Policy(policy) = &document.declarations[5] else {
            panic!("last declaration should be a policy sentence");
        };
        assert!(policy.declaration.name.is_empty());
        assert!(policy.declaration.id.starts_with("policy_"));
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
}
