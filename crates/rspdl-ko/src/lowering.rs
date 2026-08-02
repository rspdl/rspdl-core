use std::collections::{BTreeMap, BTreeSet};

use rspdl_domain::{
    ActionDefinition, CanonicalId, CanonicalType, CanonicalValue, ConstraintDefinition,
    ConstraintOperand, DataModelDefinition, DerivationDefinition, DerivationExpression,
    EnumDefinition, EnumType, EnumVariantDefinition, FieldDefinition, FieldIntentDefinition,
    FieldIntentKind, PolicyDefinition, PolicyEffect, RelationOperator, RoleDefinition,
    ScreenDefinition, ScreenOperationDefinition, ScreenOperationKind, SemanticModule,
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
    let mut screens_ast = Vec::new();
    let mut derivations_ast = Vec::new();
    let mut recalculations_ast = Vec::new();
    let mut field_intents_ast = Vec::new();
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
            DeclarationAst::Screen(value) => screens_ast.push(value.clone()),
            DeclarationAst::SumDerivation(value) => derivations_ast.push(value.clone()),
            DeclarationAst::Recalculation(value) => recalculations_ast.push(value.clone()),
            DeclarationAst::FieldIntent(value) => field_intents_ast.push(value.clone()),
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
    let (screens, derivations, field_intents) = lower_data_usage(
        screens_ast,
        derivations_ast,
        recalculations_ast,
        field_intents_ast,
        &module_id,
        &models,
        &mut top_level_ids,
        &mut diagnostics,
    );
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
            screens,
            derivations,
            field_intents,
            constraints,
            roles,
            actions,
            policies,
        }),
        diagnostics,
    }
}

struct ResolvedDerivation {
    target_field_id: CanonicalId,
    source_field_id: CanonicalId,
    span: Span,
}

#[allow(clippy::too_many_arguments)]
fn lower_data_usage(
    screens_ast: Vec<ScreenAst>,
    derivations_ast: Vec<SumDerivationAst>,
    recalculations_ast: Vec<RecalculationAst>,
    field_intents_ast: Vec<FieldIntentAst>,
    module_id: &CanonicalId,
    models: &[DataModelDefinition],
    top_level_ids: &mut BTreeSet<CanonicalId>,
    diagnostics: &mut Vec<Diagnostic>,
) -> (
    Vec<ScreenDefinition>,
    Vec<DerivationDefinition>,
    Vec<FieldIntentDefinition>,
) {
    if screens_ast.is_empty()
        && derivations_ast.is_empty()
        && recalculations_ast.is_empty()
        && field_intents_ast.is_empty()
    {
        return (Vec::new(), Vec::new(), Vec::new());
    }

    let mut screen_map = BTreeMap::<CanonicalId, ScreenDefinition>::new();
    let mut model_creators = BTreeSet::new();
    let mut model_uses = Vec::new();
    let mut input_fields = BTreeSet::new();
    let mut read_fields = BTreeSet::new();
    let mut consumers = Vec::new();
    let mut producer_spans = BTreeMap::new();

    for screen in screens_ast {
        let Some(screen_id) = canonical_member(&screen.declaration, module_id, diagnostics) else {
            continue;
        };
        if let Some(existing) = screen_map.get(&screen_id) {
            if existing.name != screen.declaration.name {
                diagnostics.push(data_diagnostic(
                    "RSPDL-DATA-004",
                    Severity::Error,
                    format!(
                        "화면 ID `{screen_id}`가 `{}`와 `{}` 두 이름으로 사용되었습니다.",
                        existing.name, screen.declaration.name
                    ),
                    screen.span,
                ));
                continue;
            }
        } else {
            duplicate_id(
                &screen_id,
                screen.declaration.span,
                top_level_ids,
                diagnostics,
            );
            screen_map.insert(
                screen_id.clone(),
                ScreenDefinition {
                    id: screen_id.clone(),
                    name: screen.declaration.name.clone(),
                    operations: Vec::new(),
                },
            );
        }

        let Some(model) = resolve_data_model(models, &screen.model, screen.span, diagnostics)
        else {
            continue;
        };
        let kind = match screen.operation {
            ScreenOperationKindAst::Create => ScreenOperationKind::Create,
            ScreenOperationKindAst::Read => ScreenOperationKind::Read,
            ScreenOperationKindAst::Input => ScreenOperationKind::Input,
            ScreenOperationKindAst::Update => ScreenOperationKind::Update,
            ScreenOperationKindAst::Delete => ScreenOperationKind::Delete,
        };
        let mut field_ids = Vec::new();
        for field_name in &screen.fields {
            let Some(field) = resolve_data_field(model, field_name, screen.span, diagnostics)
            else {
                continue;
            };
            field_ids.push(field.id.clone());
            match screen.operation {
                ScreenOperationKindAst::Input => {
                    input_fields.insert(field.id.clone());
                    producer_spans
                        .entry(field.id.clone())
                        .or_insert(screen.span);
                }
                ScreenOperationKindAst::Read => {
                    read_fields.insert(field.id.clone());
                    consumers.push((field.id.clone(), screen.span));
                }
                ScreenOperationKindAst::Update => {
                    consumers.push((field.id.clone(), screen.span));
                }
                ScreenOperationKindAst::Create | ScreenOperationKindAst::Delete => {}
            }
        }
        field_ids.sort();
        field_ids.dedup();
        let operation = ScreenOperationDefinition {
            kind,
            model_id: model.id.clone(),
            field_ids,
        };
        let definition = screen_map
            .get_mut(&screen_id)
            .expect("screen was inserted above");
        if definition.operations.contains(&operation) {
            diagnostics.push(data_diagnostic(
                "RSPDL-DATA-004",
                Severity::Error,
                format!("화면 `{screen_id}`의 데이터 동작이 중복 선언되었습니다."),
                screen.span,
            ));
        } else {
            definition.operations.push(operation);
        }
        if screen.operation == ScreenOperationKindAst::Create {
            model_creators.insert(model.id.clone());
        } else {
            model_uses.push((model.id.clone(), screen.span));
        }
    }

    let mut resolved_derivations = Vec::new();
    let mut derivation_targets = BTreeSet::new();
    for derivation in derivations_ast {
        let Some(target_model) = resolve_data_model(
            models,
            &derivation.target_model,
            derivation.span,
            diagnostics,
        ) else {
            continue;
        };
        let Some(target_field) = resolve_data_field(
            target_model,
            &derivation.target_field,
            derivation.span,
            diagnostics,
        ) else {
            continue;
        };
        let Some(source_model) = resolve_data_model(
            models,
            &derivation.source_model,
            derivation.span,
            diagnostics,
        ) else {
            continue;
        };
        let Some(source_field) = resolve_data_field(
            source_model,
            &derivation.source_field,
            derivation.span,
            diagnostics,
        ) else {
            continue;
        };
        if target_field.value_type != CanonicalType::Integer
            || source_field.value_type != CanonicalType::Integer
        {
            diagnostics.push(data_diagnostic(
                "RSPDL-DATA-005",
                Severity::Error,
                "합계의 원본과 결과 필드는 모두 정수여야 합니다.",
                derivation.span,
            ));
            continue;
        }
        if input_fields.contains(&target_field.id) {
            diagnostics.push(data_diagnostic(
                "RSPDL-DATA-004",
                Severity::Error,
                format!(
                    "계산 필드 `{}`은 화면 입력과 계산 결과를 동시에 생산자로 가질 수 없습니다.",
                    target_field.id
                ),
                derivation.span,
            ));
            continue;
        }
        if !derivation_targets.insert(target_field.id.clone()) {
            diagnostics.push(data_diagnostic(
                "RSPDL-DATA-004",
                Severity::Error,
                format!("필드 `{}`의 계산식이 중복 선언되었습니다.", target_field.id),
                derivation.span,
            ));
            continue;
        }
        if target_model.id != source_model.id {
            diagnostics.push(data_diagnostic(
                "RSPDL-DATA-W002",
                Severity::Warning,
                "교차 모델 합계의 레코드 선택 관계가 아직 정의되지 않아 의존성만 보존하고 계산 범위는 `unknown`으로 둡니다.",
                derivation.span,
            ));
        }
        producer_spans
            .entry(target_field.id.clone())
            .or_insert(derivation.span);
        consumers.push((source_field.id.clone(), derivation.span));
        model_uses.push((target_model.id.clone(), derivation.span));
        model_uses.push((source_model.id.clone(), derivation.span));
        resolved_derivations.push(ResolvedDerivation {
            target_field_id: target_field.id.clone(),
            source_field_id: source_field.id.clone(),
            span: derivation.span,
        });
    }

    let mut refreshes = BTreeMap::<CanonicalId, Vec<(CanonicalId, Span)>>::new();
    for recalculation in recalculations_ast {
        let Some(target_model) = resolve_data_model(
            models,
            &recalculation.target_model,
            recalculation.span,
            diagnostics,
        ) else {
            continue;
        };
        let Some(target_field) = resolve_data_field(
            target_model,
            &recalculation.target_field,
            recalculation.span,
            diagnostics,
        ) else {
            continue;
        };
        let Some(source_model) = resolve_data_model(
            models,
            &recalculation.source_model,
            recalculation.span,
            diagnostics,
        ) else {
            continue;
        };
        let Some(source_field) = resolve_data_field(
            source_model,
            &recalculation.source_field,
            recalculation.span,
            diagnostics,
        ) else {
            continue;
        };
        refreshes
            .entry(target_field.id.clone())
            .or_default()
            .push((source_field.id.clone(), recalculation.span));
    }

    let mut derivations = Vec::new();
    for derivation in &resolved_derivations {
        let declarations = refreshes
            .remove(&derivation.target_field_id)
            .unwrap_or_default();
        if declarations.len() != 1 {
            diagnostics.push(data_diagnostic(
                "RSPDL-DATA-003",
                Severity::Error,
                format!(
                    "계산 필드 `{}`은 재계산 시점을 정확히 하나 선언해야 합니다.",
                    derivation.target_field_id
                ),
                declarations
                    .first()
                    .map_or(derivation.span, |(_, span)| *span),
            ));
            continue;
        }
        if declarations[0].0 != derivation.source_field_id {
            diagnostics.push(data_diagnostic(
                "RSPDL-DATA-004",
                Severity::Error,
                "재계산 조건의 원본 필드가 계산식의 원본 필드와 다릅니다.",
                declarations[0].1,
            ));
            continue;
        }
        derivations.push(DerivationDefinition {
            target_field_id: derivation.target_field_id.clone(),
            expression: DerivationExpression::Sum {
                source_field_id: derivation.source_field_id.clone(),
            },
            recalculate_when_changed_field_ids: vec![derivation.source_field_id.clone()],
        });
    }
    for (target, declarations) in refreshes {
        diagnostics.push(data_diagnostic(
            "RSPDL-DATA-004",
            Severity::Error,
            format!("필드 `{target}`의 재계산 조건에 대응하는 계산식이 없습니다."),
            declarations[0].1,
        ));
    }

    let mut intents = Vec::<FieldIntentDefinition>::new();
    let mut intentional_non_reads = BTreeSet::new();
    for intent in field_intents_ast {
        let Some(model) = resolve_data_model(models, &intent.model, intent.span, diagnostics)
        else {
            continue;
        };
        let Some(field) = resolve_data_field(model, &intent.field, intent.span, diagnostics) else {
            continue;
        };
        let kind = match intent.intent {
            FieldIntentKindAst::Internal => FieldIntentKind::Internal,
            FieldIntentKindAst::Hidden => FieldIntentKind::Hidden,
        };
        let definition = FieldIntentDefinition {
            field_id: field.id.clone(),
            intent: kind,
        };
        if let Some(existing) = intents
            .iter()
            .find(|existing| existing.field_id == definition.field_id)
        {
            let message = if existing.intent == definition.intent {
                format!("필드 `{}`의 사용 의도가 중복 선언되었습니다.", field.id)
            } else {
                format!(
                    "필드 `{}`에 내부 관리와 비표시 의도를 함께 선언할 수 없습니다.",
                    field.id
                )
            };
            diagnostics.push(data_diagnostic(
                "RSPDL-DATA-004",
                Severity::Error,
                message,
                intent.span,
            ));
        } else {
            intents.push(definition);
            intentional_non_reads.insert(field.id.clone());
        }
    }

    let mut available = input_fields.clone();
    loop {
        let before = available.len();
        for derivation in &resolved_derivations {
            if available.contains(&derivation.source_field_id) {
                available.insert(derivation.target_field_id.clone());
            }
        }
        if available.len() == before {
            break;
        }
    }
    for (field, span) in consumers {
        if !available.contains(&field) {
            diagnostics.push(data_diagnostic(
                "RSPDL-DATA-001",
                Severity::Error,
                format!("필드 `{field}`을 만드는 화면 입력 또는 계산이 없습니다."),
                span,
            ));
        }
    }
    for (model, span) in model_uses {
        if !model_creators.contains(&model) {
            diagnostics.push(data_diagnostic(
                "RSPDL-DATA-002",
                Severity::Error,
                format!("데이터 모델 `{model}`을 생성하는 화면이 없습니다."),
                span,
            ));
        }
    }
    for field in available {
        if !read_fields.contains(&field) && !intentional_non_reads.contains(&field) {
            diagnostics.push(data_diagnostic(
                "RSPDL-DATA-W001",
                Severity::Warning,
                format!("필드 `{field}`은 만들어지지만 어떤 화면에서도 조회되지 않습니다. 내부 관리용 또는 비표시 의도를 명시해 주세요."),
                producer_spans.get(&field).copied().unwrap_or_default(),
            ));
        }
    }

    let mut screens = screen_map.into_values().collect::<Vec<_>>();
    for screen in &mut screens {
        screen.operations.sort();
    }
    derivations.sort_by(|left, right| left.target_field_id.cmp(&right.target_field_id));
    intents.sort();
    (screens, derivations, intents)
}

fn resolve_data_model<'a>(
    models: &'a [DataModelDefinition],
    name: &str,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<&'a DataModelDefinition> {
    models.iter().find(|model| model.name == name).or_else(|| {
        diagnostics.push(data_diagnostic(
            "RSPDL-DATA-006",
            Severity::Error,
            format!("데이터 모델 `{name}`을 찾을 수 없습니다."),
            span,
        ));
        None
    })
}

fn resolve_data_field<'a>(
    model: &'a DataModelDefinition,
    name: &str,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<&'a FieldDefinition> {
    model
        .fields
        .iter()
        .find(|field| field.name == name)
        .or_else(|| {
            diagnostics.push(data_diagnostic(
                "RSPDL-DATA-006",
                Severity::Error,
                format!(
                    "데이터 모델 `{}`에서 필드 `{name}`을 찾을 수 없습니다.",
                    model.id
                ),
                span,
            ));
            None
        })
}

fn data_diagnostic(
    rule_id: &str,
    severity: Severity,
    message: impl Into<String>,
    span: Span,
) -> Diagnostic {
    Diagnostic {
        rule_id: rule_id.into(),
        severity,
        message: message.into(),
        span,
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

    const DATA_USAGE_SOURCE: &str = r#"@모듈 장바구니(shopping)
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

    #[test]
    fn lowers_screen_producers_consumers_and_cross_model_sum_dependencies() {
        let parsed = parse(DATA_USAGE_SOURCE);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let lowered = lower(&parsed.document.unwrap());
        assert!(lowered.module.is_some(), "{:?}", lowered.diagnostics);
        assert_eq!(
            lowered
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.rule_id == "RSPDL-DATA-W002")
                .count(),
            1
        );
        let module = lowered.module.unwrap();
        assert_eq!(module.screens.len(), 6);
        assert_eq!(module.derivations.len(), 1);
        let DerivationExpression::Sum { source_field_id } = &module.derivations[0].expression;
        assert_eq!(source_field_id.as_str(), "shopping.item.amount");
        assert_eq!(
            module.derivations[0].target_field_id.as_str(),
            "shopping.cart.total"
        );
    }

    #[test]
    fn rejects_consumers_without_a_field_producer_and_derivations_without_refresh() {
        let missing_producer = DATA_USAGE_SOURCE.replace(
            "장바구니 항목 입력 화면(create_item)에서는 장바구니 항목의 수량, 금액을 입력할 수 있다.\n",
            "",
        );
        let parsed = parse(&missing_producer);
        let lowered = lower(&parsed.document.unwrap());
        assert!(lowered.module.is_none());
        assert!(
            lowered
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.rule_id == "RSPDL-DATA-001")
        );

        let missing_refresh = DATA_USAGE_SOURCE.replace(
            "장바구니 항목의 금액이 바뀔 때 장바구니의 결제 예정 금액을 다시 계산한다.\n",
            "",
        );
        let parsed = parse(&missing_refresh);
        let lowered = lower(&parsed.document.unwrap());
        assert!(
            lowered
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.rule_id == "RSPDL-DATA-003")
        );
    }

    #[test]
    fn warns_for_unread_inputs_unless_non_display_intent_is_explicit() {
        let unread = DATA_USAGE_SOURCE.replace(
            "장바구니 항목 화면(item_detail)에서는 장바구니 항목의 수량, 금액을 조회할 수 있다.",
            "장바구니 항목 화면(item_detail)에서는 장바구니 항목의 수량을 조회할 수 있다.",
        );
        let parsed = parse(&unread);
        let lowered = lower(&parsed.document.unwrap());
        assert!(lowered.module.is_some());
        assert!(lowered.diagnostics.iter().any(|diagnostic| {
            diagnostic.rule_id == "RSPDL-DATA-W001"
                && diagnostic.message.contains("shopping.item.amount")
        }));

        let intentional = unread.replace(
            "장바구니 항목의 금액이 바뀔 때",
            "장바구니 항목의 금액은 내부 관리에만 사용한다.\n장바구니 항목의 금액이 바뀔 때",
        );
        let parsed = parse(&intentional);
        let lowered = lower(&parsed.document.unwrap());
        assert!(lowered.module.is_some(), "{:?}", lowered.diagnostics);
        assert!(!lowered.diagnostics.iter().any(|diagnostic| {
            diagnostic.rule_id == "RSPDL-DATA-W001"
                && diagnostic.message.contains("shopping.item.amount")
        }));
    }

    #[test]
    fn rejects_multiple_field_producers_and_conflicting_non_display_intents() {
        let multiple_producers = DATA_USAGE_SOURCE.replace(
            "장바구니 작성 화면(create_cart)에서는 장바구니를 생성할 수 있다.\n",
            "장바구니 작성 화면(create_cart)에서는 장바구니를 생성할 수 있다.\n장바구니 작성 화면(create_cart)에서는 장바구니의 결제 예정 금액을 입력할 수 있다.\n",
        );
        let parsed = parse(&multiple_producers);
        let lowered = lower(&parsed.document.unwrap());
        assert!(lowered.module.is_none());
        assert!(lowered.diagnostics.iter().any(|diagnostic| {
            diagnostic.rule_id == "RSPDL-DATA-004"
                && diagnostic.message.contains("화면 입력과 계산 결과")
        }));

        let conflicting_intents = DATA_USAGE_SOURCE.replace(
            "장바구니 항목의 금액이 바뀔 때",
            "장바구니 항목의 금액은 내부 관리에만 사용한다.\n장바구니 항목의 금액은 사용자 화면에서 조회하지 않는다.\n장바구니 항목의 금액이 바뀔 때",
        );
        let parsed = parse(&conflicting_intents);
        let lowered = lower(&parsed.document.unwrap());
        assert!(lowered.module.is_none());
        assert!(lowered.diagnostics.iter().any(|diagnostic| {
            diagnostic.rule_id == "RSPDL-DATA-004"
                && diagnostic.message.contains("함께 선언할 수 없습니다")
        }));
    }
}
