//! Locale-independent linking, type checking, and semantic analysis.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::{
    ActionDefinition, CanonicalId, CanonicalType, CanonicalValue, ConstraintDefinition,
    ConstraintOperand, DataModelDefinition, DerivationDefinition, DerivationExpression, Diagnostic,
    EnumDefinition, EnumType, EnumVariantDefinition, FieldDefinition, FieldIntentDefinition,
    PolicyDefinition, PolicyEffect, RelationOperator, RoleDefinition, ScreenDefinition,
    ScreenOperationDefinition, ScreenOperationKind, SemanticModule, Severity, SurfaceRef,
    TextRange, UnlinkedConstraint, UnlinkedDataModel, UnlinkedDeclaration, UnlinkedFieldIntent,
    UnlinkedLiteral, UnlinkedModule, UnlinkedOperand, UnlinkedPolicy, UnlinkedRecalculation,
    UnlinkedScreen, UnlinkedSumDerivation, UnlinkedTypeReference,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AnalysisOutput {
    pub module: Option<SemanticModule>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Links and analyzes unresolved semantic intent from any conforming frontend.
pub fn analyze(module: UnlinkedModule) -> AnalysisOutput {
    let mut diagnostics = Vec::new();
    let Some(module_id) = canonical_required(&module.declaration, &mut diagnostics) else {
        return AnalysisOutput {
            module: None,
            diagnostics,
        };
    };

    let mut top_level_ids = BTreeSet::new();
    let mut enums = Vec::new();
    let mut enum_names = BTreeMap::new();
    for value in module.enums {
        let Some(id) = canonical_member(&value.declaration, &module_id, &mut diagnostics) else {
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
        for variant in value.variants {
            let Some(local_id) = canonical_required(&variant.declaration, &mut diagnostics) else {
                continue;
            };
            if !variant_ids.insert(local_id.clone()) {
                diagnostics.push(link_error(
                    format!("열거형 값 ID `{local_id}`가 중복 선언되었습니다."),
                    variant.declaration.span,
                ));
            }
            if !variant_names.insert(variant.declaration.name.clone()) {
                duplicate_name("열거형 값", &variant.declaration, &mut diagnostics);
            }
            let full_id = match CanonicalId::new(format!("{id}.{local_id}")) {
                Ok(id) => id,
                Err(error) => {
                    diagnostics.push(link_error(error.to_string(), variant.declaration.span));
                    continue;
                }
            };
            variants.push(EnumVariantDefinition {
                id: full_id,
                local_id,
                name: variant.declaration.name,
            });
        }
        match EnumType::new(
            id.clone(),
            variants.iter().map(|variant| variant.id.clone()),
        ) {
            Ok(enum_type) => enums.push(EnumDefinition {
                id,
                name: value.declaration.name,
                enum_type,
                variants,
            }),
            Err(error) => diagnostics.push(link_error(error.to_string(), value.declaration.span)),
        }
    }

    let mut roles = Vec::new();
    let mut role_names = BTreeMap::new();
    for value in module.roles {
        let Some(id) = canonical_member(&value.declaration, &module_id, &mut diagnostics) else {
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
            name: value.declaration.name,
        });
    }

    let mut actions = Vec::new();
    let mut action_names = BTreeMap::new();
    for value in module.actions {
        let Some(id) = canonical_member(&value.declaration, &module_id, &mut diagnostics) else {
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
            name: value.declaration.name,
        });
    }

    let enum_by_name = enums
        .iter()
        .map(|definition| (definition.name.clone(), definition.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut models = Vec::new();
    let mut model_names = BTreeMap::new();
    for value in module.models {
        lower_model(
            value,
            &module_id,
            &enum_by_name,
            &mut top_level_ids,
            &mut model_names,
            &mut models,
            &mut diagnostics,
        );
    }
    let models_by_name = models
        .iter()
        .map(|model| (model.name.clone(), model.clone()))
        .collect::<BTreeMap<_, _>>();

    let (screens, derivations, field_intents) = analyze_data_usage(
        module.screens,
        module.derivations,
        module.recalculations,
        module.field_intents,
        &module_id,
        &models,
        &mut top_level_ids,
        &mut diagnostics,
    );

    let mut constraints = Vec::new();
    for value in module.constraints {
        if let Some(definition) = link_constraint(
            value,
            &module_id,
            &models_by_name,
            &enum_by_name,
            &mut top_level_ids,
            &mut diagnostics,
        ) {
            constraints.push(definition);
        }
    }

    let mut policies = Vec::new();
    for value in module.policies {
        if let Some(definition) = link_policy(
            value,
            &module_id,
            &role_names,
            &action_names,
            &models_by_name,
            &mut top_level_ids,
            &mut diagnostics,
        ) {
            policies.push(definition);
        }
    }

    diagnostics.sort_by(|left, right| {
        (left.span.start, left.span.end, &left.rule_id, &left.message).cmp(&(
            right.span.start,
            right.span.end,
            &right.rule_id,
            &right.message,
        ))
    });
    if diagnostics.iter().any(Diagnostic::is_error) {
        return AnalysisOutput {
            module: None,
            diagnostics,
        };
    }

    AnalysisOutput {
        module: Some(SemanticModule {
            id: module_id,
            name: module.declaration.name,
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

#[allow(clippy::too_many_arguments)]
fn lower_model(
    value: UnlinkedDataModel,
    module_id: &CanonicalId,
    enums: &BTreeMap<String, EnumDefinition>,
    top_level_ids: &mut BTreeSet<CanonicalId>,
    model_names: &mut BTreeMap<String, CanonicalId>,
    models: &mut Vec<DataModelDefinition>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(id) = canonical_member(&value.declaration, module_id, diagnostics) else {
        return;
    };
    duplicate_id(&id, value.declaration.span, top_level_ids, diagnostics);
    if model_names
        .insert(value.declaration.name.clone(), id.clone())
        .is_some()
    {
        duplicate_name("데이터 모델", &value.declaration, diagnostics);
    }

    let mut fields = Vec::new();
    let mut local_ids = BTreeSet::new();
    let mut names = BTreeSet::new();
    for field in value.fields {
        let Some(local_id) = canonical_required(&field.declaration, diagnostics) else {
            continue;
        };
        if !local_ids.insert(local_id.clone()) {
            duplicate_name("필드 ID", &field.declaration, diagnostics);
        }
        if !names.insert(field.declaration.name.clone()) {
            duplicate_name("필드", &field.declaration, diagnostics);
        }
        let value_type = match field.value_type {
            UnlinkedTypeReference::String => Some(CanonicalType::String),
            UnlinkedTypeReference::Integer => Some(CanonicalType::Integer),
            UnlinkedTypeReference::Boolean => Some(CanonicalType::Boolean),
            UnlinkedTypeReference::Named(reference) => enums
                .get(&reference.text)
                .map(|definition| CanonicalType::Enum(definition.enum_type.clone()))
                .or_else(|| {
                    diagnostics.push(link_error(
                        format!("열거형 `{}`을 찾을 수 없습니다.", reference.text),
                        reference.span,
                    ));
                    None
                }),
        };
        let Some(value_type) = value_type else {
            continue;
        };
        let full_id = match CanonicalId::new(format!("{id}.{local_id}")) {
            Ok(id) => id,
            Err(error) => {
                diagnostics.push(link_error(error.to_string(), field.declaration.span));
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

#[allow(clippy::too_many_arguments)]
fn link_constraint(
    value: UnlinkedConstraint,
    module_id: &CanonicalId,
    models: &BTreeMap<String, DataModelDefinition>,
    enums: &BTreeMap<String, EnumDefinition>,
    top_level_ids: &mut BTreeSet<CanonicalId>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<ConstraintDefinition> {
    let model = resolve_model(models, &value.model, diagnostics)?;
    let left = resolve_operand(&value.left, model, None, enums, diagnostics)?;
    let left_type = operand_type(&left, model);
    let right = resolve_operand(&value.right, model, left_type.as_ref(), enums, diagnostics)?;
    let right_type = operand_type(&right, model);
    if left_type != right_type {
        diagnostics.push(type_error(
            "제약의 양쪽 operand 타입이 다릅니다.",
            value.span,
        ));
        return None;
    }
    if matches!(
        value.operator,
        RelationOperator::LessThan
            | RelationOperator::LessThanOrEqual
            | RelationOperator::GreaterThan
            | RelationOperator::GreaterThanOrEqual
    ) && left_type != Some(CanonicalType::Integer)
    {
        diagnostics.push(type_error(
            "대소 비교는 정수 필드에만 사용할 수 있습니다.",
            value.span,
        ));
        return None;
    }

    let local_id = value
        .declaration
        .id
        .clone()
        .unwrap_or_else(|| generated_constraint_id(&model.id, &left, value.operator, &right));
    let declaration = UnlinkedDeclaration {
        id: Some(local_id),
        ..value.declaration
    };
    let id = canonical_member(&declaration, module_id, diagnostics)?;
    duplicate_id(&id, declaration.span, top_level_ids, diagnostics);
    Some(ConstraintDefinition {
        id,
        name: declaration.name,
        model_id: model.id.clone(),
        left,
        operator: value.operator,
        right,
    })
}

#[allow(clippy::too_many_arguments)]
fn link_policy(
    value: UnlinkedPolicy,
    module_id: &CanonicalId,
    roles: &BTreeMap<String, CanonicalId>,
    actions: &BTreeMap<String, CanonicalId>,
    models: &BTreeMap<String, DataModelDefinition>,
    top_level_ids: &mut BTreeSet<CanonicalId>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<PolicyDefinition> {
    let role_id = resolve_named_id("역할", roles, &value.role, diagnostics)?;
    let model = resolve_model(models, &value.model, diagnostics)?;
    let field = resolve_field(model, &value.field, diagnostics)?;
    let action_id = resolve_named_id("행동", actions, &value.action, diagnostics)?;
    let local_id = value.declaration.id.clone().unwrap_or_else(|| {
        generated_policy_id(&role_id, &model.id, &field.id, &action_id, value.effect)
    });
    let declaration = UnlinkedDeclaration {
        id: Some(local_id),
        ..value.declaration
    };
    let id = canonical_member(&declaration, module_id, diagnostics)?;
    duplicate_id(&id, declaration.span, top_level_ids, diagnostics);
    Some(PolicyDefinition {
        id,
        name: declaration.name,
        role_id,
        model_id: model.id.clone(),
        field_id: field.id.clone(),
        action_id,
        effect: value.effect,
    })
}

struct ResolvedDerivation {
    target_field_id: CanonicalId,
    source_field_id: CanonicalId,
    span: TextRange,
}

#[allow(clippy::too_many_arguments)]
fn analyze_data_usage(
    screens: Vec<UnlinkedScreen>,
    derivations: Vec<UnlinkedSumDerivation>,
    recalculations: Vec<UnlinkedRecalculation>,
    field_intents: Vec<UnlinkedFieldIntent>,
    module_id: &CanonicalId,
    models: &[DataModelDefinition],
    top_level_ids: &mut BTreeSet<CanonicalId>,
    diagnostics: &mut Vec<Diagnostic>,
) -> (
    Vec<ScreenDefinition>,
    Vec<DerivationDefinition>,
    Vec<FieldIntentDefinition>,
) {
    if screens.is_empty()
        && derivations.is_empty()
        && recalculations.is_empty()
        && field_intents.is_empty()
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

    for screen in screens {
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

        let Some(model) = find_model(models, &screen.model, diagnostics) else {
            continue;
        };
        let mut field_ids = Vec::new();
        for field_ref in &screen.fields {
            let Some(field) = resolve_field(model, field_ref, diagnostics) else {
                continue;
            };
            field_ids.push(field.id.clone());
            match screen.operation {
                ScreenOperationKind::Input => {
                    input_fields.insert(field.id.clone());
                    producer_spans
                        .entry(field.id.clone())
                        .or_insert(screen.span);
                }
                ScreenOperationKind::Read => {
                    read_fields.insert(field.id.clone());
                    consumers.push((field.id.clone(), screen.span));
                }
                ScreenOperationKind::Update => {
                    consumers.push((field.id.clone(), screen.span));
                }
                ScreenOperationKind::Create | ScreenOperationKind::Delete => {}
            }
        }
        field_ids.sort();
        field_ids.dedup();
        let operation = ScreenOperationDefinition {
            kind: screen.operation,
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
        if screen.operation == ScreenOperationKind::Create {
            model_creators.insert(model.id.clone());
        } else {
            model_uses.push((model.id.clone(), screen.span));
        }
    }

    let mut resolved_derivations = Vec::new();
    let mut derivation_targets = BTreeSet::new();
    for derivation in derivations {
        let Some(target_model) = find_model(models, &derivation.target_model, diagnostics) else {
            continue;
        };
        let Some(target_field) = resolve_field(target_model, &derivation.target_field, diagnostics)
        else {
            continue;
        };
        let Some(source_model) = find_model(models, &derivation.source_model, diagnostics) else {
            continue;
        };
        let Some(source_field) = resolve_field(source_model, &derivation.source_field, diagnostics)
        else {
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

    let mut refreshes = BTreeMap::<CanonicalId, Vec<(CanonicalId, TextRange)>>::new();
    for recalculation in recalculations {
        let Some(target_model) = find_model(models, &recalculation.target_model, diagnostics)
        else {
            continue;
        };
        let Some(target_field) =
            resolve_field(target_model, &recalculation.target_field, diagnostics)
        else {
            continue;
        };
        let Some(source_model) = find_model(models, &recalculation.source_model, diagnostics)
        else {
            continue;
        };
        let Some(source_field) =
            resolve_field(source_model, &recalculation.source_field, diagnostics)
        else {
            continue;
        };
        refreshes
            .entry(target_field.id.clone())
            .or_default()
            .push((source_field.id.clone(), recalculation.span));
    }

    let mut derivation_definitions = Vec::new();
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
        derivation_definitions.push(DerivationDefinition {
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
    for intent in field_intents {
        let Some(model) = find_model(models, &intent.model, diagnostics) else {
            continue;
        };
        let Some(field) = resolve_field(model, &intent.field, diagnostics) else {
            continue;
        };
        let definition = FieldIntentDefinition {
            field_id: field.id.clone(),
            intent: intent.intent,
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

    let mut screen_definitions = screen_map.into_values().collect::<Vec<_>>();
    for screen in &mut screen_definitions {
        screen.operations.sort();
    }
    derivation_definitions.sort_by(|left, right| left.target_field_id.cmp(&right.target_field_id));
    intents.sort();
    (screen_definitions, derivation_definitions, intents)
}

fn resolve_operand(
    operand: &UnlinkedOperand,
    model: &DataModelDefinition,
    expected: Option<&CanonicalType>,
    enums: &BTreeMap<String, EnumDefinition>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<ConstraintOperand> {
    match operand {
        UnlinkedOperand::Field(reference) => resolve_field(model, reference, diagnostics)
            .map(|field| ConstraintOperand::Field(field.id.clone())),
        UnlinkedOperand::Literal(literal) => {
            let Some(expected) = expected else {
                diagnostics.push(type_error(
                    "literal 타입을 결정할 수 없습니다.",
                    literal_span(literal),
                ));
                return None;
            };
            literal_value(literal, expected, enums, diagnostics).map(ConstraintOperand::Constant)
        }
    }
}

fn literal_value(
    literal: &UnlinkedLiteral,
    expected: &CanonicalType,
    enums: &BTreeMap<String, EnumDefinition>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<CanonicalValue> {
    let result = match (literal, expected) {
        (UnlinkedLiteral::String { value, .. }, CanonicalType::String) => {
            Some(Ok(CanonicalValue::string(value)))
        }
        (UnlinkedLiteral::Integer { value, .. }, CanonicalType::Integer) => {
            Some(CanonicalValue::integer_from_decimal(value))
        }
        (UnlinkedLiteral::Boolean { value, .. }, CanonicalType::Boolean) => {
            Some(Ok(CanonicalValue::boolean(*value)))
        }
        (UnlinkedLiteral::Named(reference), CanonicalType::Enum(enum_type)) => {
            let variant = enums
                .values()
                .find(|definition| definition.id == *enum_type.id())
                .and_then(|definition| {
                    definition
                        .variants
                        .iter()
                        .find(|variant| variant.name == reference.text)
                        .map(|variant| variant.id.clone())
                });
            match variant {
                Some(variant) => Some(CanonicalValue::enum_variant(enum_type.clone(), variant)),
                None => {
                    diagnostics.push(link_error(
                        format!("열거형 값 `{}`을 찾을 수 없습니다.", reference.text),
                        reference.span,
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
            diagnostics.push(type_error(error.to_string(), literal_span(literal)));
            None
        }
        None => {
            diagnostics.push(type_error(
                format!("literal이 필드 타입 `{expected}`과 맞지 않습니다."),
                literal_span(literal),
            ));
            None
        }
    }
}

fn literal_span(literal: &UnlinkedLiteral) -> TextRange {
    match literal {
        UnlinkedLiteral::Named(reference) => reference.span,
        UnlinkedLiteral::String { span, .. }
        | UnlinkedLiteral::Integer { span, .. }
        | UnlinkedLiteral::Boolean { span, .. } => *span,
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

fn resolve_model<'a>(
    models: &'a BTreeMap<String, DataModelDefinition>,
    reference: &SurfaceRef,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<&'a DataModelDefinition> {
    models.get(&reference.text).or_else(|| {
        diagnostics.push(link_error(
            format!("데이터 모델 `{}`을 찾을 수 없습니다.", reference.text),
            reference.span,
        ));
        None
    })
}

fn find_model<'a>(
    models: &'a [DataModelDefinition],
    reference: &SurfaceRef,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<&'a DataModelDefinition> {
    models
        .iter()
        .find(|model| model.name == reference.text)
        .or_else(|| {
            diagnostics.push(data_diagnostic(
                "RSPDL-DATA-006",
                Severity::Error,
                format!("데이터 모델 `{}`을 찾을 수 없습니다.", reference.text),
                reference.span,
            ));
            None
        })
}

fn resolve_field<'a>(
    model: &'a DataModelDefinition,
    reference: &SurfaceRef,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<&'a FieldDefinition> {
    model
        .fields
        .iter()
        .find(|field| field.name == reference.text)
        .or_else(|| {
            diagnostics.push(link_error(
                format!(
                    "데이터 모델 `{}`에서 필드 `{}`을 찾을 수 없습니다.",
                    model.id, reference.text
                ),
                reference.span,
            ));
            None
        })
}

fn resolve_named_id(
    kind: &str,
    definitions: &BTreeMap<String, CanonicalId>,
    reference: &SurfaceRef,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<CanonicalId> {
    definitions.get(&reference.text).cloned().or_else(|| {
        diagnostics.push(link_error(
            format!("{kind} `{}`을 찾을 수 없습니다.", reference.text),
            reference.span,
        ));
        None
    })
}

fn canonical_required(
    declaration: &UnlinkedDeclaration,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<CanonicalId> {
    let Some(value) = declaration.id.as_ref() else {
        diagnostics.push(link_error(
            "선언에 stable ID가 필요합니다.",
            declaration.span,
        ));
        return None;
    };
    match CanonicalId::new(value) {
        Ok(id) => Some(id),
        Err(error) => {
            diagnostics.push(link_error(error.to_string(), declaration.span));
            None
        }
    }
}

fn canonical_member(
    declaration: &UnlinkedDeclaration,
    module_id: &CanonicalId,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<CanonicalId> {
    let local = canonical_required(declaration, diagnostics)?;
    if local.as_str().contains('.') {
        return Some(local);
    }
    match CanonicalId::new(format!("{module_id}.{local}")) {
        Ok(id) => Some(id),
        Err(error) => {
            diagnostics.push(link_error(error.to_string(), declaration.span));
            None
        }
    }
}

fn duplicate_id(
    id: &CanonicalId,
    span: TextRange,
    ids: &mut BTreeSet<CanonicalId>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !ids.insert(id.clone()) {
        diagnostics.push(link_error(
            format!("stable ID `{id}`가 중복 선언되었습니다."),
            span,
        ));
    }
}

fn duplicate_name(
    kind: &str,
    declaration: &UnlinkedDeclaration,
    diagnostics: &mut Vec<Diagnostic>,
) {
    diagnostics.push(link_error(
        format!(
            "{kind} 표시 이름 `{}`이 중복 선언되었습니다.",
            declaration.name
        ),
        declaration.span,
    ));
}

fn generated_constraint_id(
    model_id: &CanonicalId,
    left: &ConstraintOperand,
    operator: RelationOperator,
    right: &ConstraintOperand,
) -> String {
    generated_id(
        "constraint",
        &format!(
            "{model_id}\0{}\0{}\0{}",
            operand_identity(left),
            relation_identity(operator),
            operand_identity(right)
        ),
    )
}

fn generated_policy_id(
    role_id: &CanonicalId,
    model_id: &CanonicalId,
    field_id: &CanonicalId,
    action_id: &CanonicalId,
    effect: PolicyEffect,
) -> String {
    generated_id(
        "policy",
        &format!(
            "{role_id}\0{model_id}\0{field_id}\0{action_id}\0{}",
            match effect {
                PolicyEffect::Allow => "allow",
                PolicyEffect::Deny => "deny",
            }
        ),
    )
}

fn generated_id(kind: &str, identity: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in identity.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{kind}_{hash:016x}")
}

fn operand_identity(operand: &ConstraintOperand) -> String {
    match operand {
        ConstraintOperand::Field(id) => format!("field:{id}"),
        ConstraintOperand::Constant(value) => {
            if let Some(value) = value.as_string() {
                format!("string:{}:{value}", value.len())
            } else if let Some(value) = value.as_integer() {
                format!("integer:{value}")
            } else if let Some(value) = value.as_boolean() {
                format!("boolean:{value}")
            } else if let Some(value) = value.as_enum_variant() {
                format!("enum:{value}")
            } else {
                unreachable!("every canonical value representation is covered")
            }
        }
    }
}

fn relation_identity(operator: RelationOperator) -> &'static str {
    match operator {
        RelationOperator::Equal => "equal",
        RelationOperator::NotEqual => "not_equal",
        RelationOperator::LessThan => "less_than",
        RelationOperator::LessThanOrEqual => "less_than_or_equal",
        RelationOperator::GreaterThan => "greater_than",
        RelationOperator::GreaterThanOrEqual => "greater_than_or_equal",
    }
}

fn link_error(message: impl Into<String>, span: TextRange) -> Diagnostic {
    Diagnostic::error("RSPDL-LINK-003", message, span)
}

fn type_error(message: impl Into<String>, span: TextRange) -> Diagnostic {
    Diagnostic::error("RSPDL-TYPE-001", message, span)
}

fn data_diagnostic(
    rule_id: &str,
    severity: Severity,
    message: impl Into<String>,
    span: TextRange,
) -> Diagnostic {
    Diagnostic::new(rule_id, severity, message, span)
}
