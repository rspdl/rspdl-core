//! Locale-independent linking, type checking, and semantic analysis.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::{
    ActionDefinition, CanonicalId, CanonicalType, CanonicalValue, ConstraintDefinition,
    ConstraintOperand, DataModelDefinition, DerivationDefinition, DerivationExpression, Diagnostic,
    EnumDefinition, EnumType, EnumVariantDefinition, FieldDefinition, FieldIntentDefinition,
    ModelError, PolicyDefinition, PolicyEffect, RelationOperator, RoleDefinition, ScreenDefinition,
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
            duplicate_name("enum", &value.declaration, &mut diagnostics);
        }
        let mut variants = Vec::new();
        let mut variant_ids = BTreeSet::new();
        let mut variant_names = BTreeSet::new();
        for variant in value.variants {
            let Some(local_id) = canonical_required(&variant.declaration, &mut diagnostics) else {
                continue;
            };
            if !variant_ids.insert(local_id.clone()) {
                diagnostics.push(
                    link_error(
                        "semantic.enum.duplicate_variant_id",
                        variant.declaration.span,
                    )
                    .with_argument("id", &local_id),
                );
            }
            if !variant_names.insert(variant.declaration.name.clone()) {
                duplicate_name("enum_variant", &variant.declaration, &mut diagnostics);
            }
            let full_id = match CanonicalId::new(format!("{id}.{local_id}")) {
                Ok(id) => id,
                Err(error) => {
                    diagnostics.push(model_error(
                        "RSPDL-LINK-003",
                        error,
                        variant.declaration.span,
                    ));
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
            Err(error) => {
                diagnostics.push(model_error("RSPDL-LINK-003", error, value.declaration.span))
            }
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
            duplicate_name("role", &value.declaration, &mut diagnostics);
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
            duplicate_name("action", &value.declaration, &mut diagnostics);
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
    let mut model_names = BTreeSet::new();
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

    diagnostics.sort_by(Diagnostic::stable_cmp);
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
    model_names: &mut BTreeSet<String>,
    models: &mut Vec<DataModelDefinition>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(id) = canonical_member(&value.declaration, module_id, diagnostics) else {
        return;
    };
    duplicate_id(&id, value.declaration.span, top_level_ids, diagnostics);
    if !model_names.insert(value.declaration.name.clone()) {
        duplicate_name("data_model", &value.declaration, diagnostics);
    }

    let mut fields = Vec::new();
    let mut local_ids = BTreeSet::new();
    let mut names = BTreeSet::new();
    for field in value.fields {
        let Some(local_id) = canonical_required(&field.declaration, diagnostics) else {
            continue;
        };
        if !local_ids.insert(local_id.clone()) {
            diagnostics.push(
                link_error("semantic.field.duplicate_local_id", field.declaration.span)
                    .with_argument("id", &local_id),
            );
        }
        if !names.insert(field.declaration.name.clone()) {
            duplicate_name("field", &field.declaration, diagnostics);
        }
        let value_type = match field.value_type {
            UnlinkedTypeReference::String => Some(CanonicalType::String),
            UnlinkedTypeReference::Integer => Some(CanonicalType::Integer),
            UnlinkedTypeReference::Boolean => Some(CanonicalType::Boolean),
            UnlinkedTypeReference::Named(reference) => {
                resolve_enum(enums.values(), &reference, diagnostics)
                    .map(|definition| CanonicalType::Enum(definition.enum_type.clone()))
            }
        };
        let Some(value_type) = value_type else {
            continue;
        };
        let full_id = match CanonicalId::new(format!("{id}.{local_id}")) {
            Ok(id) => id,
            Err(error) => {
                diagnostics.push(model_error("RSPDL-LINK-003", error, field.declaration.span));
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
            "semantic.constraint.operand_type_mismatch",
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
            "semantic.constraint.order_requires_integer",
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
    let role_id = resolve_named_id("role", roles, &value.role, diagnostics)?;
    let model = resolve_model(models, &value.model, diagnostics)?;
    let field = resolve_field(model, &value.field, diagnostics)?;
    let action_id = resolve_named_id("action", actions, &value.action, diagnostics)?;
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
                diagnostics.push(
                    data_diagnostic(
                        "RSPDL-DATA-004",
                        Severity::Error,
                        "semantic.screen.id_name_conflict",
                        screen.span,
                    )
                    .with_argument("screen_id", &screen_id)
                    .with_argument("existing_name", &existing.name)
                    .with_argument("new_name", &screen.declaration.name),
                );
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
            diagnostics.push(
                data_diagnostic(
                    "RSPDL-DATA-004",
                    Severity::Error,
                    "semantic.screen.duplicate_operation",
                    screen.span,
                )
                .with_argument("screen_id", &screen_id),
            );
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
                "semantic.derivation.sum_requires_integer",
                derivation.span,
            ));
            continue;
        }
        if input_fields.contains(&target_field.id) {
            diagnostics.push(
                data_diagnostic(
                    "RSPDL-DATA-004",
                    Severity::Error,
                    "semantic.derivation.multiple_producers",
                    derivation.span,
                )
                .with_argument("field_id", &target_field.id),
            );
            continue;
        }
        if !derivation_targets.insert(target_field.id.clone()) {
            diagnostics.push(
                data_diagnostic(
                    "RSPDL-DATA-004",
                    Severity::Error,
                    "semantic.derivation.duplicate_target",
                    derivation.span,
                )
                .with_argument("field_id", &target_field.id),
            );
            continue;
        }
        if target_model.id != source_model.id {
            diagnostics.push(data_diagnostic(
                "RSPDL-DATA-W002",
                Severity::Warning,
                "semantic.derivation.cross_model_scope_unknown",
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
            diagnostics.push(
                data_diagnostic(
                    "RSPDL-DATA-003",
                    Severity::Error,
                    "semantic.recalculation.exactly_one_required",
                    declarations
                        .first()
                        .map_or(derivation.span, |(_, span)| *span),
                )
                .with_argument("field_id", &derivation.target_field_id)
                .with_argument("actual", declarations.len()),
            );
            continue;
        }
        if declarations[0].0 != derivation.source_field_id {
            diagnostics.push(
                data_diagnostic(
                    "RSPDL-DATA-004",
                    Severity::Error,
                    "semantic.recalculation.source_mismatch",
                    declarations[0].1,
                )
                .with_argument("expected_field_id", &derivation.source_field_id)
                .with_argument("actual_field_id", &declarations[0].0),
            );
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
        diagnostics.push(
            data_diagnostic(
                "RSPDL-DATA-004",
                Severity::Error,
                "semantic.recalculation.derivation_missing",
                declarations[0].1,
            )
            .with_argument("field_id", target),
        );
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
            let message_key = if existing.intent == definition.intent {
                "semantic.field_intent.duplicate"
            } else {
                "semantic.field_intent.conflict"
            };
            diagnostics.push(
                data_diagnostic("RSPDL-DATA-004", Severity::Error, message_key, intent.span)
                    .with_argument("field_id", &field.id),
            );
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
            diagnostics.push(
                data_diagnostic(
                    "RSPDL-DATA-001",
                    Severity::Error,
                    "semantic.lifecycle.field_producer_missing",
                    span,
                )
                .with_argument("field_id", field),
            );
        }
    }
    for (model, span) in model_uses {
        if !model_creators.contains(&model) {
            diagnostics.push(
                data_diagnostic(
                    "RSPDL-DATA-002",
                    Severity::Error,
                    "semantic.lifecycle.model_creator_missing",
                    span,
                )
                .with_argument("model_id", model),
            );
        }
    }
    for field in available {
        if !read_fields.contains(&field) && !intentional_non_reads.contains(&field) {
            diagnostics.push(
                data_diagnostic(
                    "RSPDL-DATA-W001",
                    Severity::Warning,
                    "semantic.lifecycle.produced_field_unread",
                    producer_spans.get(&field).copied().unwrap_or_default(),
                )
                .with_argument("field_id", field),
            );
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
                    "semantic.literal.type_undetermined",
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
            if !validate_reference(reference, "RSPDL-LINK-003", diagnostics) {
                return None;
            }
            let variant = enums
                .values()
                .find(|definition| definition.id == *enum_type.id())
                .and_then(|definition| {
                    definition
                        .variants
                        .iter()
                        .find(|variant| {
                            member_reference_matches(&variant.id, &variant.local_id, reference)
                        })
                        .map(|variant| variant.id.clone())
                });
            match variant {
                Some(variant) => Some(CanonicalValue::enum_variant(enum_type.clone(), variant)),
                None => {
                    diagnostics.push(
                        link_error("semantic.enum.variant_not_found", reference.span())
                            .with_argument("reference", reference.id()),
                    );
                    return None;
                }
            }
        }
        _ => None,
    };
    match result {
        Some(Ok(value)) => Some(value),
        Some(Err(error)) => {
            diagnostics.push(model_error("RSPDL-TYPE-001", error, literal_span(literal)));
            None
        }
        None => {
            diagnostics.push(
                type_error("semantic.literal.type_mismatch", literal_span(literal))
                    .with_argument("expected_type", expected),
            );
            None
        }
    }
}

fn literal_span(literal: &UnlinkedLiteral) -> TextRange {
    match literal {
        UnlinkedLiteral::Named(reference) => reference.span(),
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
    if !validate_reference(reference, "RSPDL-LINK-003", diagnostics) {
        return None;
    }
    let matches = models
        .values()
        .filter(|model| top_level_reference_matches(&model.id, reference))
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [model] => Some(*model),
        [] => {
            diagnostics.push(
                link_error("semantic.model.not_found", reference.span())
                    .with_argument("reference", reference.id()),
            );
            None
        }
        _ => {
            diagnostics.push(ambiguous_reference(
                "RSPDL-LINK-003",
                "model",
                reference,
                matches.iter().map(|model| &model.id),
            ));
            None
        }
    }
}

fn find_model<'a>(
    models: &'a [DataModelDefinition],
    reference: &SurfaceRef,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<&'a DataModelDefinition> {
    if !validate_reference(reference, "RSPDL-DATA-006", diagnostics) {
        return None;
    }
    let matches = models
        .iter()
        .filter(|model| top_level_reference_matches(&model.id, reference))
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [model] => Some(*model),
        [] => {
            diagnostics.push(
                data_diagnostic(
                    "RSPDL-DATA-006",
                    Severity::Error,
                    "semantic.model.not_found",
                    reference.span(),
                )
                .with_argument("reference", reference.id()),
            );
            None
        }
        _ => {
            diagnostics.push(ambiguous_reference(
                "RSPDL-DATA-006",
                "model",
                reference,
                matches.iter().map(|model| &model.id),
            ));
            None
        }
    }
}

fn resolve_field<'a>(
    model: &'a DataModelDefinition,
    reference: &SurfaceRef,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<&'a FieldDefinition> {
    if !validate_reference(reference, "RSPDL-LINK-003", diagnostics) {
        return None;
    }
    model
        .fields
        .iter()
        .find(|field| member_reference_matches(&field.id, &field.local_id, reference))
        .or_else(|| {
            diagnostics.push(
                link_error("semantic.field.not_found", reference.span())
                    .with_argument("model_id", &model.id)
                    .with_argument("reference", reference.id()),
            );
            None
        })
}

fn resolve_named_id(
    kind: &str,
    definitions: &BTreeMap<String, CanonicalId>,
    reference: &SurfaceRef,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<CanonicalId> {
    if !validate_reference(reference, "RSPDL-LINK-003", diagnostics) {
        return None;
    }
    let matches = definitions
        .iter()
        .filter(|(_, id)| top_level_reference_matches(id, reference))
        .map(|(_, id)| id)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [id] => Some((*id).clone()),
        [] => {
            diagnostics.push(
                link_error("semantic.symbol.not_found", reference.span())
                    .with_argument("kind", kind)
                    .with_argument("reference", reference.id()),
            );
            None
        }
        _ => {
            diagnostics.push(ambiguous_reference(
                "RSPDL-LINK-003",
                kind,
                reference,
                matches.iter().copied(),
            ));
            None
        }
    }
}

fn resolve_enum<'a>(
    definitions: impl IntoIterator<Item = &'a EnumDefinition>,
    reference: &SurfaceRef,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<&'a EnumDefinition> {
    if !validate_reference(reference, "RSPDL-LINK-003", diagnostics) {
        return None;
    }
    let matches = definitions
        .into_iter()
        .filter(|definition| top_level_reference_matches(&definition.id, reference))
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [definition] => Some(*definition),
        [] => {
            diagnostics.push(
                link_error("semantic.enum.not_found", reference.span())
                    .with_argument("reference", reference.id()),
            );
            None
        }
        _ => {
            diagnostics.push(ambiguous_reference(
                "RSPDL-LINK-003",
                "enum",
                reference,
                matches.iter().map(|definition| &definition.id),
            ));
            None
        }
    }
}

fn ambiguous_reference<'a>(
    rule_id: &str,
    kind: &str,
    reference: &SurfaceRef,
    ids: impl IntoIterator<Item = &'a CanonicalId>,
) -> Diagnostic {
    let candidates = ids
        .into_iter()
        .map(CanonicalId::as_str)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join(",");
    Diagnostic::error(rule_id, "semantic.reference.ambiguous", reference.span())
        .with_argument("kind", kind)
        .with_argument("reference", reference.id())
        .with_argument("candidates", candidates)
}

fn validate_reference(
    reference: &SurfaceRef,
    rule_id: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    match CanonicalId::new(reference.id()) {
        Ok(_) => true,
        Err(error) => {
            diagnostics.push(model_error(rule_id, error, reference.span()));
            false
        }
    }
}

fn top_level_reference_matches(id: &CanonicalId, reference: &SurfaceRef) -> bool {
    id.as_str() == reference.id()
        || (!reference.id().contains('.') && id.as_str().rsplit('.').next() == Some(reference.id()))
}

fn member_reference_matches(
    id: &CanonicalId,
    local_id: &CanonicalId,
    reference: &SurfaceRef,
) -> bool {
    id.as_str() == reference.id() || local_id.as_str() == reference.id()
}

fn canonical_required(
    declaration: &UnlinkedDeclaration,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<CanonicalId> {
    let Some(value) = declaration.id.as_ref() else {
        diagnostics.push(link_error(
            "semantic.declaration.stable_id_required",
            declaration.span,
        ));
        return None;
    };
    match CanonicalId::new(value) {
        Ok(id) => Some(id),
        Err(error) => {
            diagnostics.push(model_error("RSPDL-LINK-003", error, declaration.span));
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
            diagnostics.push(model_error("RSPDL-LINK-003", error, declaration.span));
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
        diagnostics
            .push(link_error("semantic.declaration.duplicate_id", span).with_argument("id", id));
    }
}

fn duplicate_name(
    kind: &str,
    declaration: &UnlinkedDeclaration,
    diagnostics: &mut Vec<Diagnostic>,
) {
    diagnostics.push(
        link_error("semantic.declaration.duplicate_name", declaration.span)
            .with_argument("kind", kind)
            .with_argument("name", &declaration.name),
    );
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

fn link_error(message_key: impl Into<String>, span: TextRange) -> Diagnostic {
    Diagnostic::error("RSPDL-LINK-003", message_key, span)
}

fn type_error(message_key: impl Into<String>, span: TextRange) -> Diagnostic {
    Diagnostic::error("RSPDL-TYPE-001", message_key, span)
}

fn data_diagnostic(
    rule_id: &str,
    severity: Severity,
    message_key: impl Into<String>,
    span: TextRange,
) -> Diagnostic {
    Diagnostic::new(rule_id, severity, message_key, span)
}

fn model_error(rule_id: &str, error: ModelError, span: TextRange) -> Diagnostic {
    match error {
        ModelError::InvalidCanonicalId { value } => {
            Diagnostic::error(rule_id, "model.invalid_canonical_id", span)
                .with_argument("value", value)
        }
        ModelError::EmptyEnum { type_id } => {
            Diagnostic::error(rule_id, "model.empty_enum", span).with_argument("type_id", type_id)
        }
        ModelError::DuplicateEnumVariant { type_id, variant } => {
            Diagnostic::error(rule_id, "model.duplicate_enum_variant", span)
                .with_argument("type_id", type_id)
                .with_argument("variant", variant)
        }
        ModelError::UnknownEnumVariant { type_id, variant } => {
            Diagnostic::error(rule_id, "model.unknown_enum_variant", span)
                .with_argument("type_id", type_id)
                .with_argument("variant", variant)
        }
        ModelError::InvalidRefinementBase {
            refinement,
            expected,
            actual,
        } => Diagnostic::error(rule_id, "model.invalid_refinement_base", span)
            .with_argument("refinement", refinement)
            .with_argument("expected", expected)
            .with_argument("actual", actual),
        ModelError::InvalidRefinedValue { value_type, value } => {
            Diagnostic::error(rule_id, "model.invalid_refined_value", span)
                .with_argument("value_type", value_type)
                .with_argument("value", value)
        }
        ModelError::RefinementMagnitudeExceeded {
            refinement,
            value,
            maximum,
        } => Diagnostic::error(rule_id, "model.refinement_magnitude_exceeded", span)
            .with_argument("refinement", refinement)
            .with_argument("value", value)
            .with_argument("maximum", maximum),
        ModelError::TypeMismatch {
            context,
            expected,
            actual,
        } => Diagnostic::error(rule_id, "model.type_mismatch", span)
            .with_argument("context", context)
            .with_argument("expected", expected)
            .with_argument("actual", actual),
        ModelError::EmptyOperands { operation } => {
            Diagnostic::error(rule_id, "model.empty_operands", span)
                .with_argument("operation", operation)
        }
        ModelError::ArityMismatch {
            predicate,
            expected,
            actual,
        } => Diagnostic::error(rule_id, "model.arity_mismatch", span)
            .with_argument("predicate", predicate)
            .with_argument("expected", expected)
            .with_argument("actual", actual),
        ModelError::InvalidInteger { value } => {
            Diagnostic::error(rule_id, "model.invalid_integer", span).with_argument("value", value)
        }
        ModelError::UnknownPredicate { predicate } => {
            Diagnostic::error(rule_id, "model.unknown_predicate", span)
                .with_argument("predicate", predicate)
        }
        ModelError::ConflictingPredicateSignature { predicate } => {
            Diagnostic::error(rule_id, "model.conflicting_predicate_signature", span)
                .with_argument("predicate", predicate)
        }
        ModelError::NonGroundFact {
            predicate,
            variable,
        } => Diagnostic::error(rule_id, "model.non_ground_fact", span)
            .with_argument("predicate", predicate)
            .with_argument("variable", variable),
    }
}
