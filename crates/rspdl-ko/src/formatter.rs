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

    if let Some(frontmatter) = &document.frontmatter {
        write_frontmatter(&mut output, frontmatter)?;
    }

    // 모듈은 머리말의 `모듈` 키에서 왔거나 `@모듈` 줄에서 왔다. 둘은 같은 사실이라 함께 적으면
    // reader 가 `ko.frontmatter.module_declared_twice` 로 거절한다.
    if document
        .frontmatter
        .as_ref()
        .is_none_or(|frontmatter| frontmatter.module.is_none())
    {
        if !output.is_empty() {
            output.push('\n');
        }
        output.push_str(&format!(
            "@모듈 {}({})\n",
            surface(&document.module.declaration.name),
            document.module.declaration.id
        ));
    }
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
            DeclarationAst::ActionDataMutation(value) => output.push_str(&format!(
                "{} 실행되면 {} {}.\n",
                marked(&value.action, "이", "가"),
                marked(&value.model, "을", "를"),
                match value.mutation {
                    DataMutationKindAst::Create => "생성한다",
                    DataMutationKindAst::Update => "수정한다",
                    DataMutationKindAst::Delete => "삭제한다",
                }
            )),
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
            DeclarationAst::Event(value) => output.push_str(&format!(
                "{}({}){} 사건이다.\n",
                surface(&value.declaration.name),
                value.declaration.id,
                if has_final_consonant(&value.declaration.name) {
                    "은"
                } else {
                    "는"
                }
            )),
            DeclarationAst::ActionInput(value) => {
                let input_type = match &value.kind {
                    ActionInputKindAst::ExistingModel { model } => {
                        format!("기존 {}", marked(model, "을", "를"))
                    }
                    ActionInputKindAst::Value { value_type } => {
                        marked(&type_reference(value_type), "을", "를")
                    }
                };
                output.push_str(&format!(
                    "{} {} {}({}){} 입력받는다.\n",
                    marked(&value.action, "은", "는"),
                    input_type,
                    surface(&value.declaration.name),
                    value.declaration.id,
                    directional_marker(&value.declaration.name),
                ));
            }
            DeclarationAst::EventInput(value) => {
                let input_type = match &value.kind {
                    ActionInputKindAst::ExistingModel { model } => {
                        format!("기존 {}", marked(model, "을", "를"))
                    }
                    ActionInputKindAst::Value { value_type } => {
                        marked(&type_reference(value_type), "을", "를")
                    }
                };
                output.push_str(&format!(
                    "{} {} {}({}){} 담는다.\n",
                    marked(&value.event, "은", "는"),
                    input_type,
                    surface(&value.declaration.name),
                    value.declaration.id,
                    directional_marker(&value.declaration.name)
                ));
            }
            DeclarationAst::CreationBranch(value) => {
                output.push_str(&format!(
                    "{}({}){} {}의 {} {} {} {}.\n",
                    surface(&value.declaration.name),
                    value.declaration.id,
                    if has_final_consonant(&value.declaration.name) {
                        "은"
                    } else {
                        "는"
                    },
                    surface(&value.action),
                    marked(&value.input, "이", "가"),
                    marked(&value.variant, "이면", "라면"),
                    marked(&value.output_model, "을", "를"),
                    match value.decision {
                        CreationDecisionAst::Create => "하나 생성한다",
                        CreationDecisionAst::Skip => "생성하지 않는다",
                    }
                ));
            }
            DeclarationAst::FieldProducer(value) => {
                if matches!(value.source, FieldProducerSourceAst::Template { .. })
                    && value.condition.is_some()
                {
                    return Err(FormatError::unsupported_constraint(
                        "조건부 template producer는 현재 Korean 문법으로 표현할 수 없습니다.",
                    ));
                }
                let source = match &value.source {
                    FieldProducerSourceAst::ActionInput { input } => marked(input, "을", "를"),
                    FieldProducerSourceAst::InputField { input, field } => {
                        format!("{}의 {}", surface(input), marked(field, "을", "를"))
                    }
                    FieldProducerSourceAst::Constant { literal } => {
                        format!("상수 {}", marked(&literal_text(literal), "을", "를"))
                    }
                    FieldProducerSourceAst::Template { value } => {
                        format!(
                            "{}를",
                            serde_json::to_string(value).expect("template string serializes")
                        )
                    }
                };
                let trigger = match &value.condition {
                    Some(condition) => format!(
                        "{}의 {} {}",
                        surface(&value.trigger.name),
                        marked(&condition.input, "이", "가"),
                        marked(&condition.variant, "이면", "라면"),
                    ),
                    None => format!(
                        "{} {} 때",
                        marked(&value.trigger.name, "이", "가"),
                        match value.trigger.kind {
                            ProducerTriggerKindAst::Action => "실행될",
                            ProducerTriggerKindAst::Event => "발생할",
                        },
                    ),
                };
                output.push_str(&format!(
                    "{}({}){} {} {} {}의 {} {}.\n",
                    surface(&value.declaration.name),
                    value.declaration.id,
                    if has_final_consonant(&value.declaration.name) {
                        "은"
                    } else {
                        "는"
                    },
                    trigger,
                    source,
                    surface(&value.output_model),
                    marked(&value.output_field, "으로", "로"),
                    if matches!(value.source, FieldProducerSourceAst::Template { .. }) {
                        "조합한다"
                    } else {
                        "기록한다"
                    },
                ));
            }
            DeclarationAst::RelationProducer(value) => {
                output.push_str(&format!(
                    "{}({}){} {} {} 때 {} {}의 {} 연결한다.\n",
                    surface(&value.declaration.name),
                    value.declaration.id,
                    if has_final_consonant(&value.declaration.name) {
                        "은"
                    } else {
                        "는"
                    },
                    marked(&value.trigger.name, "이", "가"),
                    match value.trigger.kind {
                        ProducerTriggerKindAst::Action => "실행될",
                        ProducerTriggerKindAst::Event => "발생할",
                    },
                    marked(&value.input, "을", "를"),
                    surface(&value.output_model),
                    marked(&value.relation, "으로", "로"),
                ));
            }
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

pub(crate) fn type_reference(value: &TypeReferenceAst) -> String {
    match value {
        TypeReferenceAst::String => "문자열".into(),
        TypeReferenceAst::Integer => "정수".into(),
        TypeReferenceAst::Boolean => "불리언".into(),
        TypeReferenceAst::Decimal => "소수".into(),
        TypeReferenceAst::Date => "날짜".into(),
        TypeReferenceAst::Time => "시간".into(),
        TypeReferenceAst::DateTime => "날짜시간".into(),
        TypeReferenceAst::Duration => "기간".into(),
        TypeReferenceAst::Latitude => "위도".into(),
        TypeReferenceAst::Longitude => "경도".into(),
        TypeReferenceAst::Money(currency) => format!("통화({currency})"),
        TypeReferenceAst::Percentage => "백분율".into(),
        TypeReferenceAst::Quantity(unit) => format!("수량({unit})"),
        TypeReferenceAst::Coordinate => "좌표".into(),
        TypeReferenceAst::LocalDateTime => "지역 날짜시간".into(),
        TypeReferenceAst::ZonedDateTime => "시간대 날짜시간".into(),
        TypeReferenceAst::CalendarDuration => "달력 기간".into(),
        TypeReferenceAst::Uuid => "UUID".into(),
        TypeReferenceAst::Email => "이메일".into(),
        TypeReferenceAst::Url => "URL".into(),
        TypeReferenceAst::PhoneNumber => "전화번호".into(),
        TypeReferenceAst::IpAddress => "IP".into(),
        TypeReferenceAst::Cidr => "CIDR".into(),
        TypeReferenceAst::CountryCode => "국가 코드".into(),
        TypeReferenceAst::LanguageCode => "언어 코드".into(),
        TypeReferenceAst::CurrencyCode => "통화 코드".into(),
        TypeReferenceAst::List(element) => format!("목록({})", type_reference(element)),
        TypeReferenceAst::Set(element) => format!("집합({})", type_reference(element)),
        TypeReferenceAst::Map(key, value) => {
            format!("맵({}, {})", type_reference(key), type_reference(value))
        }
        TypeReferenceAst::Reference(model) => format!("참조({})", surface(model)),
        TypeReferenceAst::Named(value) => surface(value),
    }
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

fn literal_text(value: &LiteralAst) -> String {
    match value {
        LiteralAst::String(value) => serde_json::to_string(value).expect("string serializes"),
        LiteralAst::Integer(value) => value.clone(),
        LiteralAst::Boolean(true) => "참".into(),
        LiteralAst::Boolean(false) => "거짓".into(),
        LiteralAst::Named(value) => surface(value),
    }
}

// ---------------------------------------------------------------------------
// 머리말 내보내기
//
// `frontmatter.rs` 의 reader 를 정확히 뒤집는다. 여기서 한 자리라도 빠뜨리면 `rspdl fmt` 가
// 사용자의 정보구조·레이아웃·흐름을 조용히 지운다.
// ---------------------------------------------------------------------------

/// 한 단 들여쓰기.
const STEP: usize = 2;

fn pad(indent: usize) -> String {
    " ".repeat(indent)
}

fn write_frontmatter(output: &mut String, frontmatter: &FrontmatterAst) -> Result<(), FormatError> {
    output.push_str("---\n");
    let mut written = false;

    if let Some(module) = &frontmatter.module {
        output.push_str(&format!("모듈: {}\n", named_id(module, false)?));
        written = true;
    }

    if !frontmatter.information_architecture.is_empty() {
        separate(output, &mut written);
        output.push_str("정보구조:\n");
        write_categories(output, &frontmatter.information_architecture, STEP)?;
    }

    if !frontmatter.screens.is_empty() {
        separate(output, &mut written);
        output.push_str("화면:\n");
        for layout in &frontmatter.screens {
            write_screen_layout(output, layout, STEP);
        }
    }

    if !frontmatter.paths.is_empty() {
        separate(output, &mut written);
        output.push_str("흐름:\n");
        for path in &frontmatter.paths {
            write_path(output, path, STEP);
        }
    }

    output.push_str("---\n");
    Ok(())
}

/// 절 사이에만 빈 줄을 둔다. 첫 절 앞에는 두지 않는다.
fn separate(output: &mut String, written: &mut bool) {
    if *written {
        output.push('\n');
    }
    *written = true;
}

/// `이름(id)`. reader 의 `read_named_id` 가 이 자리의 따옴표를 거절하므로 반드시 맨몸으로 적는다.
///
/// 따옴표로 도망칠 수 없는 유일한 자리라, 맨몸으로 적었을 때 다시 읽히지 않을 이름은 조용히
/// 내보내지 않고 여기서 한 번 실패한다.
fn named_id(declaration: &NamedIdAst, is_key: bool) -> Result<String, FormatError> {
    let name = &declaration.name;
    let unwritable = name.is_empty()
        || name.trim() != name.as_str()
        || name.chars().any(char::is_control)
        || name.starts_with(['&', '*', '!', '|', '>', '\'', '[', '{', '"', '#', '-', '?'])
        || name.contains('#')
        // key 자리는 첫 `:` 에서 잘리므로 이름이 `:` 를 품을 수 없다. value 자리는 이미 잘린
        // 뒤라 품어도 된다.
        || (is_key && name.contains(':'));
    if unwritable {
        return Err(FormatError::unsupported_constraint(format!(
            "머리말에 그대로 적을 수 없는 이름입니다: {name}"
        )));
    }
    Ok(format!("{name}({})", declaration.id))
}

/// 맨몸으로 적으면 다시 읽을 때 다른 글자가 되는 자리만 따옴표로 감싼다.
///
/// reader 가 따옴표 안을 JSON 문자열로 읽으므로(`read_scalar`), 내보낼 때 JSON 문자열로 적는 것이
/// 정확한 역연산이다. 제어문자와 따옴표는 그 덕에 따로 다룰 필요가 없다.
fn scalar(text: &str) -> String {
    if needs_quotes(text) {
        serde_json::to_string(text).expect("문자열 직렬화는 실패하지 않는다")
    } else {
        text.to_owned()
    }
}

/// 자리마다 다르게 판단하지 않는다. 규칙이 사람 머릿속에만 남으면 언젠가 어긋난다.
///
/// `:` 는 key 와 value 를 가르고, `,` 는 flow collection 의 항목을 가르고, `#` 는 주석을 연다.
/// 셋 중 하나라도 품었으면 어느 자리에 놓이든 감싼다.
fn needs_quotes(text: &str) -> bool {
    text.is_empty()
        || text.trim() != text
        || text.chars().any(char::is_control)
        || text.starts_with(['&', '*', '!', '|', '>', '\'', '[', '{', '"', '#', '-', '?'])
        || text.contains([':', ',', '#'])
}

fn write_categories(
    output: &mut String,
    categories: &[CategoryAst],
    indent: usize,
) -> Result<(), FormatError> {
    for category in categories {
        let head = format!("{}{}:", pad(indent), named_id(&category.declaration, true)?);
        if !category.children.is_empty() {
            output.push_str(&head);
            output.push('\n');
            write_categories(output, &category.children, indent + STEP)?;
        } else if !category.screens.is_empty() {
            output.push_str(&format!("{head} [{}]\n", references(&category.screens)));
        } else {
            // 잎도 가지도 아닌 분류. 빈 목록을 지어내지 않고 비어 있는 그대로 둔다.
            output.push_str(&head);
            output.push('\n');
        }
    }
    Ok(())
}

fn references(values: &[FrontmatterRefAst]) -> String {
    values
        .iter()
        .map(|value| scalar(&value.text))
        .collect::<Vec<_>>()
        .join(", ")
}

fn write_screen_layout(output: &mut String, layout: &ScreenLayoutAst, indent: usize) {
    output.push_str(&format!(
        "{}{}:\n",
        pad(indent),
        scalar(&layout.screen.text)
    ));
    let body = indent + STEP;
    // 적히지 않은 `유형` 은 적히지 않은 채로 둔다. 여기서 `page` 를 채우면 저자가 하지 않은
    // 결정을 문서가 한 것이 된다.
    if let Some(kind) = layout.kind {
        output.push_str(&format!("{}유형: {}\n", pad(body), screen_kind(kind)));
    }
    if !layout.elements.is_empty() {
        output.push_str(&format!("{}레이아웃:\n", pad(body)));
        write_elements(output, &layout.elements, body + STEP);
    }
}

fn screen_kind(kind: ScreenLayoutKindAst) -> &'static str {
    match kind {
        ScreenLayoutKindAst::Page => "page",
        ScreenLayoutKindAst::Popup => "popup",
        ScreenLayoutKindAst::Tab => "tab",
        ScreenLayoutKindAst::Link => "link",
    }
}

fn write_elements(output: &mut String, elements: &[LayoutElementAst], indent: usize) {
    for element in elements {
        let bullet = format!("{}- ", pad(indent));
        // `- ` 뒤 본문의 열이 그 항목의 들여쓰기가 된다(reader 의 항목 읽기). 그래서 자식은
        // 하이픈 기준으로 두 단 더 들어간다.
        let nested = indent + STEP + STEP;
        match element {
            LayoutElementAst::Header { children, .. } => {
                output.push_str(&format!("{bullet}머리말:\n"));
                write_elements(output, children, nested);
            }
            LayoutElementAst::Section { children, .. } => {
                output.push_str(&format!("{bullet}구역:\n"));
                write_elements(output, children, nested);
            }
            LayoutElementAst::Form { inputs, .. } => {
                output.push_str(&format!("{bullet}폼:\n"));
                write_elements(output, inputs, nested);
            }
            LayoutElementAst::Heading { text, .. } => {
                output.push_str(&format!("{bullet}제목: {}\n", scalar(text)));
            }
            LayoutElementAst::Placeholder { text, .. } => {
                output.push_str(&format!("{bullet}자리: {}\n", scalar(text)));
            }
            LayoutElementAst::Input { field, .. } => {
                output.push_str(&format!("{bullet}입력: {}\n", scalar(&field.text)));
            }
            LayoutElementAst::List { model, fields, .. } => {
                let mut body = format!("모델: {}", scalar(&model.text));
                if !fields.is_empty() {
                    body.push_str(&format!(", 필드: [{}]", references(fields)));
                }
                output.push_str(&format!("{bullet}목록: {{ {body} }}\n"));
            }
            LayoutElementAst::Button {
                id, name, action, ..
            } => {
                let mut body = format!("id: {id}, 이름: {}", scalar(name));
                if let Some(action) = action {
                    body.push_str(&format!(", 행동: {}", scalar(&action.text)));
                }
                output.push_str(&format!("{bullet}버튼: {{ {body} }}\n"));
            }
        }
    }
}

fn write_path(output: &mut String, path: &ScreenPathAst, indent: usize) {
    // 하이픈 뒤 본문의 열이 이 매핑의 들여쓰기다. 뒤따르는 키들이 그 열에 맞아야 한 항목이 된다.
    let body = indent + STEP;
    output.push_str(&format!(
        "{}- 출발: {}\n",
        pad(indent),
        scalar(&format!(
            "{}.{}",
            path.source_screen.text, path.source_element.text
        ))
    ));
    output.push_str(&format!(
        "{}도착: {}\n",
        pad(body),
        scalar(&path.target_screen.text)
    ));
    // 적히지 않은 설명은 빈 문자열이 아니다.
    if let Some(label) = &path.label {
        output.push_str(&format!("{}설명: {}\n", pad(body), scalar(label)));
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
        .is_some_and(|character| !(character as u32 - '가' as u32).is_multiple_of(28))
}

#[cfg(test)]
mod tests {
    use rspdl_domain::analyze;
    use serde_json::Value;

    use crate::{lower, parse};

    use super::*;

    fn semantic_module(document: &DocumentAst) -> Value {
        let lowered = lower(document);
        let analyzed = analyze(lowered.module.expect("parsed document should lower"));
        let module = analyzed
            .module
            .unwrap_or_else(|| panic!("{:?}", analyzed.diagnostics));
        let mut value = serde_json::to_value(module).unwrap();
        remove_source_spans(&mut value);
        value
    }

    fn remove_source_spans(value: &mut Value) {
        match value {
            Value::Object(object) => {
                object.remove("span");
                for value in object.values_mut() {
                    remove_source_spans(value);
                }
            }
            Value::Array(values) => {
                for value in values {
                    remove_source_spans(value);
                }
            }
            _ => {}
        }
    }

    fn assert_only_module_uses_annotation(source: &str) {
        let annotations = source
            .lines()
            .filter(|line| line.trim_start().starts_with('@'))
            .collect::<Vec<_>>();
        assert_eq!(annotations.len(), 1, "{source}");
        assert!(annotations[0].starts_with("@모듈 "), "{source}");
    }

    /// 머리말을 span 을 뺀 모양으로 본다. 다시 써 내면 위치는 달라지지만 뜻은 같아야 한다.
    fn frontmatter_shape(source: &str) -> Value {
        let parsed = parse(source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let document = parsed.document.expect("문서가 파싱되어야 한다");
        let mut value = serde_json::to_value(&document.frontmatter).unwrap();
        remove_source_spans(&mut value);
        value
    }

    fn reformat(source: &str) -> String {
        let document = parse(source).document.expect("문서가 파싱되어야 한다");
        format_document(&document).unwrap()
    }

    const FRONTMATTER_SOURCE: &str = concat!(
        "---\n",
        "모듈: 장바구니(shopping)\n",
        "\n",
        "정보구조:\n",
        "  주문(order):\n",
        "    결제(payment): [장바구니 항목 입력 화면]\n",
        "    조회(inquiry):\n",
        "\n",
        "화면:\n",
        "  장바구니 항목 입력 화면:\n",
        "    유형: page\n",
        "    레이아웃:\n",
        "      - 머리말:\n",
        "          - 제목: \"항목 추가\"\n",
        "      - 구역:\n",
        "          - 폼:\n",
        "              - 입력: 수량\n",
        "              - 입력: 금액\n",
        "          - 목록: { 모델: 장바구니 항목, 필드: [수량, 금액] }\n",
        "          - 자리: \"지도\"\n",
        "          - 버튼: { id: submit, 이름: \"담기\" }\n",
        "\n",
        "흐름:\n",
        "  - 출발: 장바구니 항목 입력 화면.submit\n",
        "    도착: 장바구니 상세 화면\n",
        "    설명: \"담기 성공\"\n",
        "---\n",
        "\n",
        "장바구니 항목(item)은 다음 필드들로 구성되어 있다.\n",
        "    수량(quantity): 필수 정수\n",
        "    금액(amount): 필수 정수\n",
        "\n",
        "장바구니 항목 입력 화면(create_item)에서는 장바구니 항목을 생성할 수 있다.\n",
        "장바구니 항목 입력 화면(create_item)에서는 장바구니 항목의 수량, 금액을 입력할 수 있다.\n",
    );

    #[test]
    fn frontmatter_round_trips_and_is_idempotent() {
        let first = reformat(FRONTMATTER_SOURCE);
        // 뜻이 남는가. 문자열이 아니라 읽어 낸 모양으로 본다.
        assert_eq!(
            frontmatter_shape(FRONTMATTER_SOURCE),
            frontmatter_shape(&first)
        );
        // 두 번 돌린 결과가 한 번 돌린 결과와 바이트까지 같은가.
        assert_eq!(first, reformat(&first));
        // 문장 쪽도 그대로 남는가.
        assert!(first.contains(
            "장바구니 항목 입력 화면(create_item)에서는 장바구니 항목을 생성할 수 있다."
        ));
    }

    #[test]
    fn module_from_the_frontmatter_is_not_repeated_as_an_annotation() {
        let first = reformat(FRONTMATTER_SOURCE);
        assert!(
            first.starts_with("---\n모듈: 장바구니(shopping)\n"),
            "{first}"
        );
        // 두 자리에 같은 사실을 적으면 reader 가 거절한다.
        assert!(!first.contains("@모듈"), "{first}");
    }

    #[test]
    fn a_module_declared_by_annotation_survives_a_frontmatter_without_one() {
        let source = "---\n정보구조:\n  주문(order): [장바구니 작성 화면]\n---\n\n@모듈 장바구니(shopping)\n";
        let first = reformat(source);
        assert_eq!(frontmatter_shape(source), frontmatter_shape(&first));
        assert!(first.contains("@모듈 장바구니(shopping)"), "{first}");
        assert_eq!(first, reformat(&first));
    }

    #[test]
    fn unspecified_kind_and_label_are_not_filled_in() {
        let source = concat!(
            "---\n",
            "모듈: 장바구니(shopping)\n",
            "\n",
            "화면:\n",
            "  장바구니 작성 화면:\n",
            "    레이아웃:\n",
            "      - 버튼: { id: submit, 이름: \"담기\" }\n",
            "\n",
            "흐름:\n",
            "  - 출발: 장바구니 작성 화면.submit\n",
            "    도착: 장바구니 상세 화면\n",
            "---\n",
        );
        let first = reformat(source);
        // 적히지 않은 것을 채우면 저자가 하지 않은 결정을 문서가 한 것이 된다.
        assert!(!first.contains("유형"), "{first}");
        assert!(!first.contains("설명"), "{first}");
        assert_eq!(frontmatter_shape(source), frontmatter_shape(&first));
        assert_eq!(first, reformat(&first));
    }

    #[test]
    fn colons_commas_and_leading_hyphens_survive_the_round_trip() {
        let source = concat!(
            "---\n",
            "모듈: 장바구니(shopping)\n",
            "\n",
            "화면:\n",
            "  장바구니 작성 화면:\n",
            "    레이아웃:\n",
            "      - 제목: \"가격: 표\"\n",
            "      - 자리: \"-지도\"\n",
            "      - 버튼: { id: submit, 이름: \"담기, 그리고 계속\" }\n",
            "---\n",
        );
        let first = reformat(source);
        assert_eq!(frontmatter_shape(source), frontmatter_shape(&first));
        assert_eq!(first, reformat(&first));
        // 감싸지 않으면 `:` 는 키를 가르고 `,` 는 항목을 가른다.
        assert!(first.contains("제목: \"가격: 표\""), "{first}");
        assert!(first.contains("자리: \"-지도\""), "{first}");
        assert!(first.contains("이름: \"담기, 그리고 계속\""), "{first}");
    }

    #[test]
    fn documents_without_a_frontmatter_are_written_exactly_as_before() {
        let source = "@모듈 승인(approval)\n신청(request)은 다음 필드들로 구성되어 있다.\n    금액(amount): 필수 정수\n";
        let first = reformat(source);
        assert!(first.starts_with("@모듈 승인(approval)\n"), "{first}");
        assert!(!first.contains("---"), "{first}");
        assert_only_module_uses_annotation(&first);
    }

    #[test]
    fn formatting_is_idempotent() {
        let source = "@모듈 승인(approval)\n상태(state)는 다음 값 중 하나다.\n  작성 중(draft)\n신청(request)은 다음 필드들로 구성되어 있다.\n  금액(amount): 필수 정수\n신청의 금액은 0보다 커야 한다.\n관리자(manager)는 역할이다.\n등록(register)은 행동이다.\n변경(change)은 행동이다.\n등록은 기존 신청을 대상 신청(request)으로 입력받는다.\n등록은 문자열을 요청 메모(note)로 입력받는다.\n등록이 실행되면 신청을 생성한다.\n변경이 실행되면 신청을 수정한다.\n관리자는 신청의 금액을 변경할 수 있다.\n";
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
    fn event_header_input_and_creation_branches_round_trip() {
        let source = "@모듈 사건(event)\n상태(status)는 다음 값 중 하나다.\n    접수됨(received)\n    보류됨(held)\n알림(notice)은 다음 필드들로 구성되어 있다.\n    내용(content): 선택 문자열\n요청 접수됨(request_received)은 사건이다.\n요청 접수됨은 상태를 요청 상태(request_status)로 담는다.\n접수 알림 생성(received_create)은 요청 접수됨의 요청 상태가 접수됨이면 알림을 하나 생성한다.\n보류 알림 미생성(held_skip)은 요청 접수됨의 요청 상태가 보류됨이면 알림을 생성하지 않는다.\n";
        let original = parse(source);
        assert!(
            original.diagnostics.is_empty(),
            "{:?}",
            original.diagnostics
        );
        let first = format_document(&original.document.unwrap()).unwrap();
        assert!(first.contains("요청 접수됨(request_received)은 사건이다."));
        assert!(first.contains("요청 접수됨은 상태를 요청 상태(request_status)로 담는다."));
        let reparsed = parse(&first);
        assert!(
            reparsed.diagnostics.is_empty(),
            "{:?}\n{first}",
            reparsed.diagnostics
        );
        assert_eq!(first, format_document(&reparsed.document.unwrap()).unwrap());
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
    fn field_producer_sentences_round_trip_and_are_idempotent() {
        let source = "@모듈 기록(binding)\n알림 제목 기록(title_binding)은 점검 요청 전달이 실행될 때 알림 제목을 점검 요청 전달 알림의 제목으로 기록한다.\n요청 제목 기록(request_title_binding)은 점검 요청 전달이 실행될 때 대상 요청의 제목을 점검 요청 전달 알림의 요청 제목으로 기록한다.\n재시도 횟수 기록(retry_binding)은 점검 요청 전달이 실행될 때 상수 0을 점검 요청 전달 알림의 재시도 횟수로 기록한다.\n접수 제목 기록(received_title)은 점검 요청 전달의 요청 상태가 접수됨이면 상수 \"접수됨\"를 점검 요청 전달 알림의 제목으로 기록한다.\n";
        let original = parse(source).document.unwrap();
        let first = format_document(&original).unwrap();
        let reparsed = parse(&first);
        assert!(
            reparsed.diagnostics.is_empty(),
            "{:?}\n{first}",
            reparsed.diagnostics
        );
        assert_eq!(first, format_document(&reparsed.document.unwrap()).unwrap());
    }

    #[test]
    fn event_payload_producer_sentences_round_trip_and_are_idempotent() {
        let source = "@모듈 사건(event)\n제목 기록(title_binding)은 요청 접수됨이 발생할 때 알림 제목을 알림의 제목으로 기록한다.\n수신자 연결(recipient_binding)은 요청 접수됨이 발생할 때 수신 기술자를 알림의 수신자로 연결한다.\n";
        let original = parse(source).document.unwrap();
        let first = format_document(&original).unwrap();
        assert!(first.contains("요청 접수됨이 발생할 때"));
        let reparsed = parse(&first);
        assert!(
            reparsed.diagnostics.is_empty(),
            "{:?}\n{first}",
            reparsed.diagnostics
        );
        assert_eq!(first, format_document(&reparsed.document.unwrap()).unwrap());
    }

    #[test]
    fn template_strings_with_quotes_and_literal_braces_round_trip() {
        let source = r#"@모듈 기록(binding)
알림 내용 조합(content_template)은 점검 요청 전달이 실행될 때 "\"{알림 제목}\" {{원문}}"를 점검 요청 전달 알림의 내용으로 조합한다.
"#;
        let original = parse(source);
        assert!(
            original.diagnostics.is_empty(),
            "{:?}",
            original.diagnostics
        );
        let document = original.document.unwrap();
        let first = format_document(&document).unwrap();
        assert!(first.contains("\\\"{알림 제목}\\\" {{원문}}"));
        let reparsed = parse(&first);
        assert!(
            reparsed.diagnostics.is_empty(),
            "{:?}\n{first}",
            reparsed.diagnostics
        );
        assert_eq!(first, format_document(&reparsed.document.unwrap()).unwrap());

        let mut conditional = document;
        let DeclarationAst::FieldProducer(producer) = &mut conditional.declarations[0] else {
            panic!()
        };
        producer.condition = Some(FieldProducerConditionAst {
            input: "상태".into(),
            variant: "접수됨".into(),
        });
        assert!(format_document(&conditional).is_err());
    }

    #[test]
    fn extended_scalar_types_and_ordered_literals_round_trip() {
        let source = "@모듈 확장(extended)\n이벤트(event)는 다음 필드들로 구성되어 있다.\n  금액(amount): 필수 소수\n  날짜(date): 필수 날짜\n  시간(time): 필수 시간\n  시점(date_time): 필수 날짜시간\n  기간(duration): 필수 기간\n  위도(latitude): 필수 위도\n  경도(longitude): 필수 경도\n이벤트의 날짜는 \"2026-08-13\" 이상이어야 한다.\n";
        let parsed = parse(source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let document = parsed.document.unwrap();
        let original_module = semantic_module(&document);
        let formatted = format_document(&document).unwrap();
        let reparsed = parse(&formatted);
        assert!(
            reparsed.diagnostics.is_empty(),
            "{:?}",
            reparsed.diagnostics
        );
        assert_eq!(
            original_module,
            semantic_module(&reparsed.document.unwrap())
        );
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

    #[test]
    fn conditional_creation_sentences_round_trip_and_are_idempotent() {
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
        let original = parse(source);
        assert!(
            original.diagnostics.is_empty(),
            "{:?}",
            original.diagnostics
        );
        let original = original.document.unwrap();
        let original_module = semantic_module(&original);
        let first = format_document(&original).unwrap();
        assert!(first.contains("접수 상태 알림 생성(received_notice_create)은 점검 요청 전달의 요청 상태가 접수됨이면 점검 요청 전달 알림을 하나 생성한다."));
        assert!(first.contains("보류 상태 알림 미생성(on_hold_notice_skip)은 점검 요청 전달의 요청 상태가 보류됨이면 점검 요청 전달 알림을 생성하지 않는다."));
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
