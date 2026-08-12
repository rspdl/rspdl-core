use crate::ast::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FormatError {
    message: String,
}

impl FormatError {
    fn unsupported_constraint(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for FormatError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for FormatError {}

pub fn format_document(document: &DocumentAst) -> Result<String, FormatError> {
    let mut output = String::new();
    output.push_str(&format!(
        "@모듈 {}({})\n",
        surface(&document.module.declaration.name),
        document.module.declaration.id
    ));
    for declaration in &document.declarations {
        output.push('\n');
        match declaration {
            DeclarationAst::Enum(value) => {
                output.push_str(&format!(
                    "{}({}){} 다음 값 중 하나다.\n",
                    surface(&value.declaration.name),
                    value.declaration.id,
                    if has_final_consonant(&value.declaration.name) {
                        "은"
                    } else {
                        "는"
                    }
                ));
                for variant in &value.values {
                    output.push_str(&format!(
                        "    {}({})\n",
                        surface(&variant.declaration.name),
                        variant.declaration.id
                    ));
                }
            }
            DeclarationAst::DataModel(value) => {
                if value.fields.is_empty() {
                    return Err(FormatError::unsupported_constraint(
                        "필드가 없는 데이터 모델은 Korean 문법으로 표현할 수 없습니다.",
                    ));
                } else {
                    output.push_str(&format!(
                        "{}({}){} 다음 필드들로 구성되어 있다.\n",
                        surface(&value.declaration.name),
                        value.declaration.id,
                        if has_final_consonant(&value.declaration.name) {
                            "은"
                        } else {
                            "는"
                        }
                    ));
                    for field in &value.fields {
                        output.push_str(&format!(
                            "    {}({}): {} {}\n",
                            surface(&field.declaration.name),
                            field.declaration.id,
                            if field.required { "필수" } else { "선택" },
                            type_reference(&field.value_type)
                        ));
                    }
                }
            }
            DeclarationAst::Relation(value) => match value.parameter_models.as_slice() {
                [model] => output.push_str(&format!(
                    "{} {}({})에 해당할 수 있다.\n",
                    marked(model, "은", "는"),
                    surface(&value.declaration.name),
                    value.declaration.id,
                )),
                [source, target] => output.push_str(&format!(
                    "{} {} {}({}){} 가질 수 있다.\n",
                    marked(source, "은", "는"),
                    marked(target, "을", "를"),
                    surface(&value.declaration.name),
                    value.declaration.id,
                    directional_marker(&value.declaration.name),
                )),
                _ => {
                    return Err(FormatError::unsupported_constraint(
                        "Korean 문법은 단항 관계와 이항 관계만 표현할 수 있습니다.",
                    ));
                }
            },
            DeclarationAst::RelationalConstraint(value) => match &value.constraint {
                RelationalConstraintKindAst::NonEmpty { model } => output.push_str(&format!(
                    "{} 하나 이상 존재해야 한다.\n",
                    marked(model, "은", "는")
                )),
                RelationalConstraintKindAst::Required { model, relation } => {
                    output.push_str(&format!(
                        "모든 {} {} 하나 이상 가져야 한다.\n",
                        marked(model, "은", "는"),
                        marked(relation, "을", "를")
                    ));
                }
                RelationalConstraintKindAst::Unique { model, relation } => {
                    output.push_str(&format!(
                        "각 {} {} 최대 하나만 가질 수 있다.\n",
                        marked(model, "은", "는"),
                        marked(relation, "을", "를")
                    ));
                }
                RelationalConstraintKindAst::Exclusive { relations } => {
                    output.push_str(&format!(
                        "{} 중 둘 이상은 동시에 성립할 수 없다.\n",
                        reference_list(relations)
                    ));
                }
                RelationalConstraintKindAst::Exhaustive { relations } => {
                    output.push_str(&format!(
                        "{} 중 하나 이상은 항상 성립해야 한다.\n",
                        reference_list(relations)
                    ));
                }
                RelationalConstraintKindAst::Coexistent { relations } => {
                    output.push_str(&format!(
                        "{} 동시에 성립할 수 있다.\n",
                        topic_list(relations)
                    ));
                }
            },
            DeclarationAst::Screen(value) => {
                let screen = format!(
                    "{}({})에서는",
                    surface(&value.declaration.name),
                    value.declaration.id
                );
                let operation = match value.operation {
                    ScreenOperationKindAst::Create => "생성할",
                    ScreenOperationKindAst::Read => "조회할",
                    ScreenOperationKindAst::Input => "입력할",
                    ScreenOperationKindAst::Update => "수정할",
                    ScreenOperationKindAst::Delete => "삭제할",
                };
                if value.fields.is_empty() {
                    output.push_str(&format!(
                        "{screen} {} {operation} 수 있다.\n",
                        marked(&value.model, "을", "를")
                    ));
                } else {
                    output.push_str(&format!(
                        "{screen} {}의 {} {operation} 수 있다.\n",
                        surface(&value.model),
                        object_list(&value.fields)
                    ));
                }
            }
            DeclarationAst::SumDerivation(value) => output.push_str(&format!(
                "{}의 {} {}의 {}의 합계로 계산한다.\n",
                surface(&value.target_model),
                marked(&value.target_field, "은", "는"),
                surface(&value.source_model),
                surface(&value.source_field)
            )),
            DeclarationAst::Recalculation(value) => output.push_str(&format!(
                "{}의 {} 바뀔 때 {}의 {} 다시 계산한다.\n",
                surface(&value.source_model),
                marked(&value.source_field, "이", "가"),
                surface(&value.target_model),
                marked(&value.target_field, "을", "를")
            )),
            DeclarationAst::FieldIntent(value) => output.push_str(&format!(
                "{}의 {} {}.\n",
                surface(&value.model),
                marked(&value.field, "은", "는"),
                match value.intent {
                    FieldIntentKindAst::Internal => "내부 관리에만 사용한다",
                    FieldIntentKindAst::Hidden => "사용자 화면에서 조회하지 않는다",
                }
            )),
            DeclarationAst::Constraint(value) => {
                output.push_str(&format!("{}\n", constraint(&value.expression)?));
            }
            DeclarationAst::Role(value) => output.push_str(&format!(
                "{}({}){} 역할이다.\n",
                surface(&value.declaration.name),
                value.declaration.id,
                if has_final_consonant(&value.declaration.name) {
                    "은"
                } else {
                    "는"
                }
            )),
            DeclarationAst::Action(value) => output.push_str(&format!(
                "{}({}){} 행동이다.\n",
                surface(&value.declaration.name),
                value.declaration.id,
                if has_final_consonant(&value.declaration.name) {
                    "은"
                } else {
                    "는"
                }
            )),
            DeclarationAst::Policy(value) => {
                let role = marked(&value.role, "은", "는");
                let field = marked(&value.field, "을", "를");
                output.push_str(&format!(
                    "{} {}의 {} {}할 수 {}.\n",
                    role,
                    surface(&value.model),
                    field,
                    surface(&value.action),
                    match value.effect {
                        PolicyEffectAst::Allow => "있다",
                        PolicyEffectAst::Deny => "없다",
                    }
                ));
            }
        }
    }
    Ok(output)
}

fn reference_list(references: &[String]) -> String {
    references
        .iter()
        .map(|reference| surface(reference))
        .collect::<Vec<_>>()
        .join(", ")
}

fn topic_list(references: &[String]) -> String {
    let Some((last, rest)) = references.split_last() else {
        return String::new();
    };
    rest.iter()
        .map(|reference| surface(reference))
        .chain([marked(last, "은", "는")])
        .collect::<Vec<_>>()
        .join(", ")
}

fn object_list(fields: &[String]) -> String {
    let Some((last, rest)) = fields.split_last() else {
        return String::new();
    };
    rest.iter()
        .map(|field| surface(field))
        .chain([marked(last, "을", "를")])
        .collect::<Vec<_>>()
        .join(", ")
}

fn constraint(expression: &ConstraintExpressionAst) -> Result<String, FormatError> {
    let model = format!("{}의", surface(&expression.model));
    match (&expression.left, &expression.right) {
        (OperandAst::Field(left), OperandAst::Field(right)) => match expression.operator {
            RelationOperatorAst::Equal | RelationOperatorAst::NotEqual => Ok(format!(
                "{} {} {} {} 한다.",
                model,
                marked(left, "과", "와"),
                marked(right, "은", "는"),
                match expression.operator {
                    RelationOperatorAst::Equal => "같아야",
                    RelationOperatorAst::NotEqual => "달라야",
                    _ => unreachable!("operator was checked above"),
                },
            )),
            operator => Err(FormatError::unsupported_constraint(format!(
                "필드끼리의 `{operator:?}` 제약은 Korean v0.1 문법으로 format할 수 없습니다."
            ))),
        },
        (OperandAst::Field(left), OperandAst::Literal(literal)) => {
            let left = marked(left, "은", "는");
            let literal = literal_text(literal);
            match expression.operator {
                RelationOperatorAst::GreaterThan => {
                    Ok(format!("{model} {left} {literal}보다 커야 한다."))
                }
                RelationOperatorAst::GreaterThanOrEqual => {
                    Ok(format!("{model} {left} {literal} 이상이어야 한다."))
                }
                RelationOperatorAst::LessThan => {
                    Ok(format!("{model} {left} {literal}보다 작아야 한다."))
                }
                RelationOperatorAst::LessThanOrEqual => {
                    Ok(format!("{model} {left} {literal} 이하여야 한다."))
                }
                RelationOperatorAst::Equal => Ok(format!("{model} {left} {literal}이어야 한다.")),
                RelationOperatorAst::NotEqual => Ok(format!(
                    "{model} {left} {} 달라야 한다.",
                    marked(&literal, "과", "와")
                )),
            }
        }
        _ => Err(FormatError::unsupported_constraint(
            "Korean v0.1 문법은 제약의 왼쪽 피연산자로 필드만 지원합니다.",
        )),
    }
}

fn type_reference(value: &TypeReferenceAst) -> String {
    match value {
        TypeReferenceAst::String => "문자열".into(),
        TypeReferenceAst::Integer => "정수".into(),
        TypeReferenceAst::Boolean => "불리언".into(),
        TypeReferenceAst::Named(value) => surface(value),
    }
}

fn literal_text(value: &LiteralAst) -> String {
    match value {
        LiteralAst::String(value) => serde_json::to_string(value).expect("string serializes"),
        LiteralAst::Integer(value) => value.clone(),
        LiteralAst::Boolean(true) => "참".into(),
        LiteralAst::Boolean(false) => "거짓".into(),
        LiteralAst::Named(value) => surface(value),
    }
}

fn surface(value: &str) -> String {
    if value.chars().all(|character| {
        !character.is_control()
            && !matches!(character, '`' | '[' | ']' | '(' | ')' | ':' | '.' | '#')
    }) {
        value.to_owned()
    } else {
        format!("`{value}`")
    }
}

fn marked(value: &str, consonant: &str, vowel: &str) -> String {
    format!(
        "{}{}",
        surface(value),
        if has_final_consonant(value) {
            consonant
        } else {
            vowel
        }
    )
}

fn directional_marker(value: &str) -> &'static str {
    let Some(last) = value
        .chars()
        .last()
        .filter(|character| ('가'..='힣').contains(character))
    else {
        return "로";
    };
    let final_consonant = (last as u32 - '가' as u32) % 28;
    if matches!(final_consonant, 0 | 8) {
        "로"
    } else {
        "으로"
    }
}

fn has_final_consonant(value: &str) -> bool {
    value
        .chars()
        .last()
        .filter(|character| ('가'..='힣').contains(character))
        .is_some_and(|character| (character as u32 - '가' as u32) % 28 != 0)
}

#[cfg(test)]
mod tests {
    use rspdl_domain::{SemanticModule, analyze};

    use crate::{lower, parse};

    use super::*;

    fn semantic_module(document: &DocumentAst) -> SemanticModule {
        let lowered = lower(document);
        let analyzed = analyze(lowered.module.expect("parsed document should lower"));
        analyzed
            .module
            .unwrap_or_else(|| panic!("{:?}", analyzed.diagnostics))
    }

    fn assert_only_module_uses_annotation(source: &str) {
        let annotations = source
            .lines()
            .filter(|line| line.trim_start().starts_with('@'))
            .collect::<Vec<_>>();
        assert_eq!(annotations.len(), 1, "{source}");
        assert!(annotations[0].starts_with("@모듈 "), "{source}");
    }

    #[test]
    fn formatting_is_idempotent() {
        let source = "@모듈 승인(approval)\n상태(state)는 다음 값 중 하나다.\n  작성 중(draft)\n신청(request)은 다음 필드들로 구성되어 있다.\n  금액(amount): 필수 정수\n신청의 금액은 0보다 커야 한다.\n관리자(manager)는 역할이다.\n변경(change)은 행동이다.\n관리자는 신청의 금액을 변경할 수 있다.\n";
        let original = parse(source).document.unwrap();
        let original_module = semantic_module(&original);
        let first = format_document(&original).unwrap();
        assert_only_module_uses_annotation(&first);
        let formatted = parse(&first).document.unwrap();
        let second = format_document(&formatted).unwrap();
        assert_eq!(first, second);
        assert_eq!(original_module, semantic_module(&formatted));
    }

    #[test]
    fn literal_not_equal_constraints_round_trip() {
        let source = "@모듈 비교(comparison)\n항목(item)은 다음 필드들로 구성되어 있다.\n  값(value): 필수 정수\n항목의 값은 0과 달라야 한다.\n";
        let original = parse(source).document.unwrap();
        let original_module = semantic_module(&original);

        let formatted = format_document(&original).unwrap();
        let reparsed = parse(&formatted).document.unwrap();

        assert_eq!(original_module, semantic_module(&reparsed));
    }

    #[test]
    fn screen_and_sum_sentences_round_trip_without_blocks() {
        let source = "@모듈 집계(summary)\n항목(item)은 다음 필드들로 구성되어 있다.\n  금액(amount): 필수 정수\n  합계(total): 필수 정수\n  내부 메모(internal_note): 필수 문자열\n  공개 설명(public_description): 필수 문자열\n항목 작성 화면(create_item)에서는 항목을 생성할 수 있다.\n항목 작성 화면(create_item)에서는 항목의 금액, 내부 메모, 공개 설명을 입력할 수 있다.\n항목 상세 화면(item_detail)에서는 항목의 금액, 합계를 조회할 수 있다.\n항목의 합계는 항목의 금액의 합계로 계산한다.\n항목의 금액이 바뀔 때 항목의 합계를 다시 계산한다.\n항목의 내부 메모는 내부 관리에만 사용한다.\n항목의 공개 설명은 사용자 화면에서 조회하지 않는다.\n";
        let parsed = parse(source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let document = parsed.document.unwrap();
        let original_module = semantic_module(&document);
        let first = format_document(&document).unwrap();
        let reparsed = parse(&first);
        assert!(
            reparsed.diagnostics.is_empty(),
            "{:?}\n{first}",
            reparsed.diagnostics
        );
        let reparsed_document = reparsed.document.unwrap();
        assert_eq!(original_module, semantic_module(&reparsed_document));
        let second = format_document(&reparsed_document).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn relations_and_meta_rules_round_trip() {
        let source = "@모듈 관계(relations)\n프로젝트(project)는 다음 필드들로 구성되어 있다.\n  이름(name): 필수 문자열\n사용자(user)는 다음 필드들로 구성되어 있다.\n  이름(name): 필수 문자열\n프로젝트는 사용자를 소유자(owner)로 가질 수 있다.\n프로젝트는 하나 이상 존재해야 한다.\n모든 프로젝트는 소유자를 하나 이상 가져야 한다.\n각 프로젝트는 소유자를 최대 하나만 가질 수 있다.\n";
        let original = parse(source).document.unwrap();
        let original_module = semantic_module(&original);
        let first = format_document(&original).unwrap();
        assert_only_module_uses_annotation(&first);
        let reparsed = parse(&first).document.unwrap();

        assert_eq!(original_module, semantic_module(&reparsed));
        assert_eq!(first, format_document(&reparsed).unwrap());
    }

    #[test]
    fn unary_relation_groups_round_trip() {
        let source = "@모듈 분류(classification)\n사용자(user)는 다음 필드들로 구성되어 있다.\n  이름(name): 필수 문자열\n사용자는 내부(internal)에 해당할 수 있다.\n사용자는 외부(external)에 해당할 수 있다.\n내부, 외부 중 둘 이상은 동시에 성립할 수 없다.\n내부, 외부 중 하나 이상은 항상 성립해야 한다.\n";
        let original = parse(source).document.unwrap();
        let original_module = semantic_module(&original);

        let first = format_document(&original).unwrap();
        assert_only_module_uses_annotation(&first);
        let reparsed = parse(&first);
        assert!(
            reparsed.diagnostics.is_empty(),
            "{:?}\n{first}",
            reparsed.diagnostics
        );
        let reparsed = reparsed.document.unwrap();

        assert_eq!(original_module, semantic_module(&reparsed));
        assert_eq!(first, format_document(&reparsed).unwrap());
    }

    #[test]
    fn coexistent_relation_group_round_trips() {
        let source = "@모듈 협업(collaboration)\n프로젝트(project)는 다음 필드들로 구성되어 있다.\n  이름(name): 필수 문자열\n사용자(user)는 다음 필드들로 구성되어 있다.\n  이름(name): 필수 문자열\n프로젝트는 사용자를 소유자(owner)로 가질 수 있다.\n프로젝트는 사용자를 검토자(reviewer)로 가질 수 있다.\n소유자, 검토자는 동시에 성립할 수 있다.\n";
        let original = parse(source).document.unwrap();
        let original_module = semantic_module(&original);

        let first = format_document(&original).unwrap();
        assert_only_module_uses_annotation(&first);
        let reparsed = parse(&first);
        assert!(
            reparsed.diagnostics.is_empty(),
            "{:?}\n{first}",
            reparsed.diagnostics
        );
        let reparsed = reparsed.document.unwrap();

        assert_eq!(original_module, semantic_module(&reparsed));
        assert_eq!(first, format_document(&reparsed).unwrap());
    }
}
