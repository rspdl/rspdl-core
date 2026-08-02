use std::collections::{BTreeMap, BTreeSet};

use rspdl_domain::{
    ActionDefinition, CanonicalId, CanonicalType, CanonicalValue, ConstraintDefinition,
    ConstraintOperand, DataModelDefinition, EnumDefinition, EnumType, EnumVariantDefinition,
    FieldDefinition, PolicyDefinition, PolicyEffect, RelationOperator, RoleDefinition,
    SemanticModule,
};
use serde::Serialize;

use crate::ast::*;
use crate::{Diagnostic, Severity};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct LowerOutput {
    pub module: Option<SemanticModule>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn lower(document: &DocumentAst) -> LowerOutput {
    let mut diagnostics = Vec::new();
    let Some(module_id) = canonical(&document.module.declaration, &mut diagnostics) else {
        return LowerOutput {
            module: None,
            diagnostics,
        };
    };

    let mut top_level_ids = BTreeSet::new();
    let mut enums = Vec::new();
    let mut enum_names = BTreeMap::new();
    let mut models_ast = Vec::new();
    let mut constraints_ast = Vec::new();
    let mut roles = Vec::new();
    let mut role_names = BTreeMap::new();
    let mut actions = Vec::new();
    let mut action_names = BTreeMap::new();
    let mut policies_ast = Vec::new();

    for declaration in &document.declarations {
        match declaration {
            DeclarationAst::Enum(value) => {
                let Some(id) = canonical_member(&value.declaration, &module_id, &mut diagnostics)
                else {
                    continue;
                };
                duplicate_id(
                    &id,
                    value.declaration.span,
                    &mut top_level_ids,
                    &mut diagnostics,
                );
                if enum_names
                    .insert(value.declaration.name.clone(), id.clone())
                    .is_some()
                {
                    duplicate_name("열거형", &value.declaration, &mut diagnostics);
                }
                let mut variants = Vec::new();
                let mut variant_ids = BTreeSet::new();
                let mut variant_names = BTreeSet::new();
                for variant in &value.values {
                    let Some(local_id) = canonical(&variant.declaration, &mut diagnostics) else {
                        continue;
                    };
                    if !variant_ids.insert(local_id.clone()) {
                        diagnostics.push(lower_error(
                            format!("열거형 값 ID `{local_id}`가 중복 선언되었습니다."),
                            variant.declaration.span,
                        ));
                    }
                    if !variant_names.insert(variant.declaration.name.clone()) {
                        duplicate_name("열거형 값", &variant.declaration, &mut diagnostics);
                    }
                    let full_id = match CanonicalId::new(format!("{}.{}", id, local_id)) {
                        Ok(id) => id,
                        Err(error) => {
                            diagnostics
                                .push(lower_error(error.to_string(), variant.declaration.span));
                            continue;
                        }
                    };
                    variants.push(EnumVariantDefinition {
                        id: full_id,
                        local_id,
                        name: variant.declaration.name.clone(),
                    });
                }
                match EnumType::new(
                    id.clone(),
                    variants.iter().map(|variant| variant.id.clone()),
                ) {
                    Ok(enum_type) => enums.push(EnumDefinition {
                        id,
                        name: value.declaration.name.clone(),
                        enum_type,
                        variants,
                    }),
                    Err(error) => {
                        diagnostics.push(lower_error(error.to_string(), value.declaration.span))
                    }
                }
            }
            DeclarationAst::DataModel(value) => models_ast.push(value.clone()),
            DeclarationAst::Constraint(value) => constraints_ast.push(value.clone()),
            DeclarationAst::Role(value) => {
                let Some(id) = canonical_member(&value.declaration, &module_id, &mut diagnostics)
                else {
                    continue;
                };
                duplicate_id(
                    &id,
                    value.declaration.span,
                    &mut top_level_ids,
                    &mut diagnostics,
                );
                if role_names
                    .insert(value.declaration.name.clone(), id.clone())
                    .is_some()
                {
                    duplicate_name("역할", &value.declaration, &mut diagnostics);
                }
                roles.push(RoleDefinition {
                    id,
                    name: value.declaration.name.clone(),
                });
            }
            DeclarationAst::Action(value) => {
                let Some(id) = canonical_member(&value.declaration, &module_id, &mut diagnostics)
                else {
                    continue;
                };
                duplicate_id(
                    &id,
                    value.declaration.span,
                    &mut top_level_ids,
                    &mut diagnostics,
                );
                if action_names
                    .insert(value.declaration.name.clone(), id.clone())
                    .is_some()
                {
                    duplicate_name("행동", &value.declaration, &mut diagnostics);
                }
                actions.push(ActionDefinition {
                    id,
                    name: value.declaration.name.clone(),
                });
            }
            DeclarationAst::Policy(value) => policies_ast.push(value.clone()),
        }
    }

    let enum_by_name = enums
        .iter()
        .map(|definition| (definition.name.clone(), definition.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut models = Vec::new();
    let mut model_names = BTreeMap::new();
    for value in models_ast {
        let Some(id) = canonical_member(&value.declaration, &module_id, &mut diagnostics) else {
            continue;
        };
        duplicate_id(
            &id,
            value.declaration.span,
            &mut top_level_ids,
            &mut diagnostics,
        );
        if model_names
            .insert(value.declaration.name.clone(), id.clone())
            .is_some()
        {
            duplicate_name("데이터 모델", &value.declaration, &mut diagnostics);
        }
        let mut fields = Vec::new();
        let mut local_ids = BTreeSet::new();
        let mut names = BTreeSet::new();
        for field in value.fields {
            let Some(local_id) = canonical(&field.declaration, &mut diagnostics) else {
                continue;
            };
            if !local_ids.insert(local_id.clone()) {
                duplicate_name("필드 ID", &field.declaration, &mut diagnostics);
            }
            if !names.insert(field.declaration.name.clone()) {
                duplicate_name("필드", &field.declaration, &mut diagnostics);
            }
            let value_type = match field.value_type {
                TypeReferenceAst::String => Some(CanonicalType::String),
                TypeReferenceAst::Integer => Some(CanonicalType::Integer),
                TypeReferenceAst::Boolean => Some(CanonicalType::Boolean),
                TypeReferenceAst::Named(name) => enum_by_name
                    .get(&name)
                    .map(|definition| CanonicalType::Enum(definition.enum_type.clone()))
                    .or_else(|| {
                        diagnostics.push(lower_error(
                            format!("열거형 `{name}`을 찾을 수 없습니다."),
                            field.declaration.span,
                        ));
                        None
                    }),
            };
            let Some(value_type) = value_type else {
                continue;
            };
            let full_id = match CanonicalId::new(format!("{}.{}", id, local_id)) {
                Ok(id) => id,
                Err(error) => {
                    diagnostics.push(lower_error(error.to_string(), field.declaration.span));
                    continue;
                }
            };
            fields.push(FieldDefinition {
                id: full_id,
                local_id,
                name: field.declaration.name,
                required: field.required,
                value_type,
            });
        }
        models.push(DataModelDefinition {
            id,
            name: value.declaration.name,
            fields,
        });
    }

    let models_by_name = models
        .iter()
        .map(|model| (model.name.clone(), model.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut constraints = Vec::new();
    for value in constraints_ast {
        let Some(id) = canonical_member(&value.declaration, &module_id, &mut diagnostics) else {
            continue;
        };
        duplicate_id(
            &id,
            value.declaration.span,
            &mut top_level_ids,
            &mut diagnostics,
        );
        let Some(model) = models_by_name.get(&value.expression.model) else {
            diagnostics.push(lower_error(
                format!(
                    "데이터 모델 `{}`을 찾을 수 없습니다.",
                    value.expression.model
                ),
                value.expression.span,
            ));
            continue;
        };
        let Some(left) = resolve_operand(
            &value.expression.left,
            model,
            None,
            &enum_by_name,
            value.expression.span,
            &mut diagnostics,
        ) else {
            continue;
        };
        let left_type = operand_type(&left, model);
        let Some(right) = resolve_operand(
            &value.expression.right,
            model,
            left_type.as_ref(),
            &enum_by_name,
            value.expression.span,
            &mut diagnostics,
        ) else {
            continue;
        };
        let right_type = operand_type(&right, model);
        if left_type != right_type {
            diagnostics.push(lower_error(
                "제약의 양쪽 operand 타입이 다릅니다.",
                value.expression.span,
            ));
            continue;
        }
        let operator = relation(value.expression.operator);
        if matches!(
            operator,
            RelationOperator::LessThan
                | RelationOperator::LessThanOrEqual
                | RelationOperator::GreaterThan
                | RelationOperator::GreaterThanOrEqual
        ) && left_type != Some(CanonicalType::Integer)
        {
            diagnostics.push(lower_error(
                "대소 비교는 정수 필드에만 사용할 수 있습니다.",
                value.expression.span,
            ));
            continue;
        }
        constraints.push(ConstraintDefinition {
            id,
            name: value.declaration.name,
            model_id: model.id.clone(),
            left,
            operator,
            right,
        });
    }

    let mut policies = Vec::new();
    for value in policies_ast {
        let Some(id) = canonical_member(&value.declaration, &module_id, &mut diagnostics) else {
            continue;
        };
        duplicate_id(
            &id,
            value.declaration.span,
            &mut top_level_ids,
            &mut diagnostics,
        );
        let Some(role_id) = role_names.get(&value.role).cloned() else {
            diagnostics.push(lower_error(
                format!("역할 `{}`을 찾을 수 없습니다.", value.role),
                value.span,
            ));
            continue;
        };
        let Some(model) = models_by_name.get(&value.model) else {
            diagnostics.push(lower_error(
                format!("데이터 모델 `{}`을 찾을 수 없습니다.", value.model),
                value.span,
            ));
            continue;
        };
        let Some(field) = model.fields.iter().find(|field| field.name == value.field) else {
            diagnostics.push(lower_error(
                format!("필드 `{}`을 찾을 수 없습니다.", value.field),
                value.span,
            ));
            continue;
        };
        let Some(action_id) = action_names.get(&value.action).cloned() else {
            diagnostics.push(lower_error(
                format!("행동 `{}`을 찾을 수 없습니다.", value.action),
                value.span,
            ));
            continue;
        };
        policies.push(PolicyDefinition {
            id,
            name: value.declaration.name,
            role_id,
            model_id: model.id.clone(),
            field_id: field.id.clone(),
            action_id,
            effect: match value.effect {
                PolicyEffectAst::Allow => PolicyEffect::Allow,
                PolicyEffectAst::Deny => PolicyEffect::Deny,
            },
        });
    }

    if diagnostics.iter().any(Diagnostic::is_error) {
        return LowerOutput {
            module: None,
            diagnostics,
        };
    }
    enums.sort_by(|left, right| left.id.cmp(&right.id));
    models.sort_by(|left, right| left.id.cmp(&right.id));
    constraints.sort_by(|left, right| left.id.cmp(&right.id));
    roles.sort_by(|left, right| left.id.cmp(&right.id));
    actions.sort_by(|left, right| left.id.cmp(&right.id));
    policies.sort_by(|left, right| left.id.cmp(&right.id));
    LowerOutput {
        module: Some(SemanticModule {
            id: module_id,
            name: document.module.declaration.name.clone(),
            enums,
            models,
            constraints,
            roles,
            actions,
            policies,
        }),
        diagnostics,
    }
}

fn resolve_operand(
    operand: &OperandAst,
    model: &DataModelDefinition,
    expected: Option<&CanonicalType>,
    enums: &BTreeMap<String, EnumDefinition>,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<ConstraintOperand> {
    match operand {
        OperandAst::Field(name) => model
            .fields
            .iter()
            .find(|field| field.name == *name)
            .map(|field| ConstraintOperand::Field(field.id.clone()))
            .or_else(|| {
                diagnostics.push(lower_error(
                    format!("필드 `{name}`을 찾을 수 없습니다."),
                    span,
                ));
                None
            }),
        OperandAst::Literal(literal) => {
            let Some(expected) = expected else {
                diagnostics.push(lower_error("literal 타입을 결정할 수 없습니다.", span));
                return None;
            };
            literal_value(literal, expected, enums, span, diagnostics)
                .map(ConstraintOperand::Constant)
        }
    }
}

fn literal_value(
    literal: &LiteralAst,
    expected: &CanonicalType,
    enums: &BTreeMap<String, EnumDefinition>,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<CanonicalValue> {
    let result = match (literal, expected) {
        (LiteralAst::String(value), CanonicalType::String) => {
            Some(Ok(CanonicalValue::string(value)))
        }
        (LiteralAst::Integer(value), CanonicalType::Integer) => {
            Some(CanonicalValue::integer_from_decimal(value))
        }
        (LiteralAst::Boolean(value), CanonicalType::Boolean) => {
            Some(Ok(CanonicalValue::boolean(*value)))
        }
        (LiteralAst::Named(name), CanonicalType::Enum(enum_type)) => {
            let variant = enums
                .values()
                .find(|definition| definition.id == *enum_type.id())
                .and_then(|definition| {
                    definition
                        .variants
                        .iter()
                        .find(|variant| variant.name == *name)
                        .map(|variant| variant.id.clone())
                });
            match variant {
                Some(variant) => Some(CanonicalValue::enum_variant(enum_type.clone(), variant)),
                None => {
                    diagnostics.push(lower_error(
                        format!("열거형 값 `{name}`을 찾을 수 없습니다."),
                        span,
                    ));
                    return None;
                }
            }
        }
        _ => None,
    };
    match result {
        Some(Ok(value)) => Some(value),
        Some(Err(error)) => {
            diagnostics.push(lower_error(error.to_string(), span));
            None
        }
        None => {
            diagnostics.push(lower_error(
                format!("literal이 필드 타입 `{expected}`과 맞지 않습니다."),
                span,
            ));
            None
        }
    }
}

fn operand_type(operand: &ConstraintOperand, model: &DataModelDefinition) -> Option<CanonicalType> {
    match operand {
        ConstraintOperand::Field(id) => model
            .fields
            .iter()
            .find(|field| field.id == *id)
            .map(|field| field.value_type.clone()),
        ConstraintOperand::Constant(value) => Some(value.value_type().clone()),
    }
}

fn canonical(declaration: &NamedIdAst, diagnostics: &mut Vec<Diagnostic>) -> Option<CanonicalId> {
    match CanonicalId::new(&declaration.id) {
        Ok(id) => Some(id),
        Err(error) => {
            diagnostics.push(lower_error(error.to_string(), declaration.span));
            None
        }
    }
}

fn canonical_member(
    declaration: &NamedIdAst,
    module_id: &CanonicalId,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<CanonicalId> {
    let local = canonical(declaration, diagnostics)?;
    if declaration.id.contains('.') {
        return Some(local);
    }
    match CanonicalId::new(format!("{module_id}.{local}")) {
        Ok(id) => Some(id),
        Err(error) => {
            diagnostics.push(lower_error(error.to_string(), declaration.span));
            None
        }
    }
}

fn duplicate_id(
    id: &CanonicalId,
    span: Span,
    ids: &mut BTreeSet<CanonicalId>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !ids.insert(id.clone()) {
        diagnostics.push(lower_error(
            format!("stable ID `{id}`가 중복 선언되었습니다."),
            span,
        ));
    }
}

fn duplicate_name(kind: &str, declaration: &NamedIdAst, diagnostics: &mut Vec<Diagnostic>) {
    diagnostics.push(lower_error(
        format!(
            "{kind} 표시 이름 `{}`이 중복 선언되었습니다.",
            declaration.name
        ),
        declaration.span,
    ));
}

fn relation(operator: RelationOperatorAst) -> RelationOperator {
    match operator {
        RelationOperatorAst::Equal => RelationOperator::Equal,
        RelationOperatorAst::NotEqual => RelationOperator::NotEqual,
        RelationOperatorAst::LessThan => RelationOperator::LessThan,
        RelationOperatorAst::LessThanOrEqual => RelationOperator::LessThanOrEqual,
        RelationOperatorAst::GreaterThan => RelationOperator::GreaterThan,
        RelationOperatorAst::GreaterThanOrEqual => RelationOperator::GreaterThanOrEqual,
    }
}

fn lower_error(message: impl Into<String>, span: Span) -> Diagnostic {
    Diagnostic {
        rule_id: "RSPDL-KO-LOWER-001".into(),
        severity: Severity::Error,
        message: message.into(),
        span,
    }
}

#[cfg(test)]
mod tests {
    use crate::parse;

    use super::*;

    #[test]
    fn resolves_surface_names_to_canonical_ids() {
        let source = r#"@모듈 승인(expense)
신청(request)은 다음 필드들로 구성되어 있다.
    금액(amount): 필수 정수
신청의 금액은 0보다 커야 한다.
@역할 관리자(manager)
@행동 변경(change)
관리자는 신청의 금액을 변경할 수 있다.
"#;
        let parsed = parse(source);
        assert!(parsed.diagnostics.is_empty());
        let lowered = lower(&parsed.document.unwrap());
        assert!(lowered.diagnostics.is_empty(), "{:?}", lowered.diagnostics);
        let module = lowered.module.unwrap();
        assert_eq!(module.constraints[0].model_id.as_str(), "expense.request");
        assert_eq!(
            module.policies[0].field_id.as_str(),
            "expense.request.amount"
        );
    }

    #[test]
    fn duplicate_names_and_unknown_references_are_rejected() {
        let source = r#"@모듈 승인(approval)
신청(request_one)은 다음 필드들로 구성되어 있다.
    금액(amount): 필수 정수
신청(request_two)은 다음 필드들로 구성되어 있다.
    이름(name): 필수 문자열
없음의 금액은 0보다 커야 한다.
"#;
        let parsed = parse(source);
        let lowered = lower(&parsed.document.unwrap());
        assert!(lowered.module.is_none());
        assert!(
            lowered
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("중복 선언"))
        );
        assert!(
            lowered
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("찾을 수 없습니다"))
        );
    }
}
