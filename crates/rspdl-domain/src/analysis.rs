//! Locale-independent linking, type checking, and semantic analysis.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::frontend::ProductionTriggerKind;
use crate::{
    ActionDataMutationDefinition, ActionDataMutationProvenance, ActionDefinition,
    ActionInputDefinition, ActionInputKind, CanonicalId, CanonicalType, CanonicalValue,
    ConditionalProductionDefinition, ConstraintDefinition, ConstraintOperand,
    CreationBranchDefinition, CreationDecision, DataModelDefinition, DataMutationKind,
    DerivationDefinition, DerivationExpression, Diagnostic, EnumDefinition, EnumType,
    EnumVariantDefinition, EventDefinition, EventInputDefinition, EventInputKind, FieldDefinition,
    FieldIntentDefinition, FieldProducerCondition, FieldProducerDefinition, FieldProducerSource,
    ModelError, OutputRelationSlotDefinition, PolicyDefinition, PolicyEffect, ProducerPhase,
    ProductionCardinality, ProductionTriggerDefinition, RecalculationDefinition,
    RelationDefinition, RelationOperator, RelationProducerDefinition, RelationSlotCardinality,
    RelationalConstraintDefinition, RelationalConstraintKind, RoleDefinition, ScreenDefinition,
    ScreenOperationDefinition, ScreenOperationKind, SemanticModule, Severity, SourceId, SurfaceRef,
    TemplatePart, TextRange, UnlinkedActionDataMutation, UnlinkedActionInputKind,
    UnlinkedConstraint, UnlinkedCreationBranch, UnlinkedDataModel, UnlinkedDeclaration,
    UnlinkedEventInputKind, UnlinkedFieldIntent, UnlinkedFieldProducer,
    UnlinkedFieldProducerCondition, UnlinkedFieldProducerSource, UnlinkedLiteral, UnlinkedModule,
    UnlinkedOperand, UnlinkedPolicy, UnlinkedRecalculation, UnlinkedRelation,
    UnlinkedRelationProducer, UnlinkedRelationalConstraint, UnlinkedRelationalConstraintKind,
    UnlinkedScreen, UnlinkedSumDerivation, UnlinkedTemplatePart, UnlinkedTypeReference,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AnalysisOutput {
    pub module: Option<SemanticModule>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub action_data_mutation_provenance: Vec<ActionDataMutationProvenance>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Links and analyzes unresolved semantic intent from any conforming frontend.
pub fn analyze(module: UnlinkedModule) -> AnalysisOutput {
    analyze_with_source(module, SourceId::inline())
}

/// Links and analyzes unresolved intent while retaining its source identity.
pub fn analyze_with_source(module: UnlinkedModule, source_id: SourceId) -> AnalysisOutput {
    let mut diagnostics = Vec::new();
    let module_span = module.span;
    let Some(module_id) = canonical_required(&module.declaration, &mut diagnostics) else {
        return AnalysisOutput {
            module: None,
            action_data_mutation_provenance: Vec::new(),
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
                span: variant.span,
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
                span: value.span,
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
            span: value.span,
        });
    }

    let mut actions = Vec::new();
    let mut action_names = BTreeMap::new();
    let mut pending_action_inputs = BTreeMap::new();
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
            id: id.clone(),
            name: value.declaration.name,
            inputs: Vec::new(),
            span: value.span,
        });
        pending_action_inputs.insert(id, value.inputs);
    }
    let mut events = Vec::new();
    let mut event_names = BTreeMap::new();
    let mut pending_event_inputs = BTreeMap::new();
    for value in module.events {
        let Some(id) = canonical_member(&value.declaration, &module_id, &mut diagnostics) else {
            continue;
        };
        duplicate_id(
            &id,
            value.declaration.span,
            &mut top_level_ids,
            &mut diagnostics,
        );
        if event_names
            .insert(value.declaration.name.clone(), id.clone())
            .is_some()
        {
            duplicate_name("event", &value.declaration, &mut diagnostics);
        }
        events.push(EventDefinition {
            id: id.clone(),
            name: value.declaration.name,
            inputs: Vec::new(),
            span: value.span,
        });
        pending_event_inputs.insert(id, value.inputs);
    }

    let enum_by_name = enums
        .iter()
        .map(|definition| (definition.name.clone(), definition.clone()))
        .collect::<BTreeMap<_, _>>();
    let declared_model_ids = module
        .models
        .iter()
        .filter_map(|model| {
            model
                .declaration
                .id
                .as_ref()
                .and_then(|id| qualify_member_id(&module_id, id).ok())
        })
        .collect::<BTreeSet<_>>();
    let mut models = Vec::new();
    let mut model_names = BTreeSet::new();
    for value in module.models {
        lower_model(
            value,
            &module_id,
            &enum_by_name,
            &declared_model_ids,
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
    for action in &mut actions {
        let Some(inputs) = pending_action_inputs.remove(&action.id) else {
            continue;
        };
        let mut seen = BTreeSet::new();
        for input in inputs {
            let Some(local_id) = canonical_required(&input.declaration, &mut diagnostics) else {
                continue;
            };
            if !seen.insert(local_id.clone()) {
                diagnostics.push(
                    link_error("semantic.action_input.duplicate_id", input.declaration.span)
                        .with_argument("id", local_id.as_str()),
                );
                continue;
            }
            let id = match CanonicalId::new(format!("{}.{}", action.id, local_id)) {
                Ok(id) => id,
                Err(error) => {
                    diagnostics.push(model_error("RSPDL-LINK-003", error, input.declaration.span));
                    continue;
                }
            };
            let kind = match input.kind {
                UnlinkedActionInputKind::ExistingModel { model } => {
                    resolve_model(&models_by_name, &model, &mut diagnostics).map(|model| {
                        ActionInputKind::ExistingModel {
                            model_id: model.id.clone(),
                        }
                    })
                }
                UnlinkedActionInputKind::Value { value_type } => link_field_type(
                    value_type,
                    &enum_by_name,
                    &module_id,
                    &declared_model_ids,
                    input.declaration.span,
                    &mut diagnostics,
                )
                .map(|value_type| ActionInputKind::Value { value_type }),
            };
            if let Some(kind) = kind {
                action.inputs.push(ActionInputDefinition {
                    id,
                    local_id,
                    name: input.declaration.name,
                    kind,
                    span: input.span,
                });
            }
        }
        action.inputs.sort_by(|left, right| left.id.cmp(&right.id));
    }
    for event in &mut events {
        let Some(inputs) = pending_event_inputs.remove(&event.id) else {
            continue;
        };
        let mut seen = BTreeSet::new();
        for input in inputs {
            let Some(local_id) = canonical_required(&input.declaration, &mut diagnostics) else {
                continue;
            };
            if !seen.insert(local_id.clone()) {
                diagnostics.push(
                    link_error("semantic.event_input.duplicate_id", input.declaration.span)
                        .with_argument("id", local_id.as_str()),
                );
                continue;
            }
            let Ok(id) = CanonicalId::new(format!("{}.{}", event.id, local_id)) else {
                continue;
            };
            let kind = match input.kind {
                UnlinkedEventInputKind::ExistingModel { model } => {
                    resolve_model(&models_by_name, &model, &mut diagnostics).map(|model| {
                        EventInputKind::ExistingModel {
                            model_id: model.id.clone(),
                        }
                    })
                }
                UnlinkedEventInputKind::Value { value_type } => link_field_type(
                    value_type,
                    &enum_by_name,
                    &module_id,
                    &declared_model_ids,
                    input.declaration.span,
                    &mut diagnostics,
                )
                .map(|value_type| EventInputKind::Value { value_type }),
            };
            if let Some(kind) = kind {
                event.inputs.push(EventInputDefinition {
                    id,
                    local_id,
                    name: input.declaration.name,
                    kind,
                    span: input.span,
                });
            }
        }
        event.inputs.sort_by(|left, right| left.id.cmp(&right.id));
    }

    let mut relations = Vec::new();
    let mut relation_names = BTreeSet::new();
    for value in module.relations {
        link_relation(
            value,
            &module_id,
            &models_by_name,
            &mut top_level_ids,
            &mut relation_names,
            &mut relations,
            &mut diagnostics,
        );
    }

    let mut relational_constraints_with_spans = Vec::new();
    for value in module.relational_constraints {
        let span = value.span;
        if let Some(definition) = link_relational_constraint(
            value,
            &module_id,
            &models_by_name,
            &relations,
            &mut top_level_ids,
            &mut diagnostics,
        ) {
            relational_constraints_with_spans.push((definition, span));
        }
    }
    validate_relation_compatibility(&relational_constraints_with_spans, &mut diagnostics);
    let relational_constraints: Vec<RelationalConstraintDefinition> =
        relational_constraints_with_spans
            .into_iter()
            .map(|(definition, _)| definition)
            .collect();

    let conditional_productions = analyze_conditional_productions(
        module.creation_branches,
        module.field_producers,
        module.relation_producers,
        &module_id,
        &actions,
        &action_names,
        &events,
        &event_names,
        &models_by_name,
        &enums,
        &relations,
        &relational_constraints,
        &mut top_level_ids,
        &mut diagnostics,
    );

    let DataUsageAnalysis {
        screens,
        action_data_mutations,
        action_data_mutation_provenance,
        derivations,
        recalculations,
        field_intents,
    } = analyze_data_usage(
        module.screens,
        module.action_data_mutations,
        module.derivations,
        module.recalculations,
        module.field_intents,
        &module_id,
        &models,
        &action_names,
        &source_id,
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
            action_data_mutation_provenance: Vec::new(),
            diagnostics,
        };
    }

    AnalysisOutput {
        module: Some(SemanticModule {
            id: module_id,
            name: module.declaration.name,
            span: module_span,
            enums,
            models,
            relations,
            relational_constraints,
            screens,
            action_data_mutations,
            derivations,
            recalculations,
            field_intents,
            constraints,
            roles,
            actions,
            events,
            conditional_productions,
            policies,
        }),
        action_data_mutation_provenance,
        diagnostics,
    }
}

#[allow(clippy::too_many_arguments)]
fn link_relation(
    value: UnlinkedRelation,
    module_id: &CanonicalId,
    models: &BTreeMap<String, DataModelDefinition>,
    top_level_ids: &mut BTreeSet<CanonicalId>,
    relation_names: &mut BTreeSet<String>,
    relations: &mut Vec<RelationDefinition>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(id) = canonical_member(&value.declaration, module_id, diagnostics) else {
        return;
    };
    duplicate_id(&id, value.declaration.span, top_level_ids, diagnostics);
    if !relation_names.insert(value.declaration.name.clone()) {
        duplicate_name("relation", &value.declaration, diagnostics);
    }
    if !(1..=2).contains(&value.parameter_models.len()) {
        diagnostics.push(
            Diagnostic::error(
                "RSPDL-REL-001",
                "semantic.relation.arity_unsupported",
                value.span,
            )
            .with_argument("actual", value.parameter_models.len())
            .with_argument("supported", "1..2"),
        );
        return;
    }
    let parameter_model_ids = value
        .parameter_models
        .iter()
        .filter_map(|reference| resolve_model(models, reference, diagnostics))
        .map(|model| model.id.clone())
        .collect::<Vec<_>>();
    if parameter_model_ids.len() != value.parameter_models.len() {
        return;
    }
    relations.push(RelationDefinition {
        id,
        name: value.declaration.name,
        parameter_model_ids,
        span: value.span,
    });
}

fn link_relational_constraint(
    value: UnlinkedRelationalConstraint,
    module_id: &CanonicalId,
    models: &BTreeMap<String, DataModelDefinition>,
    relations: &[RelationDefinition],
    top_level_ids: &mut BTreeSet<CanonicalId>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<RelationalConstraintDefinition> {
    let constraint = match value.constraint {
        UnlinkedRelationalConstraintKind::NonEmpty { model } => {
            let model = resolve_model(models, &model, diagnostics)?;
            RelationalConstraintKind::NonEmpty {
                model_id: model.id.clone(),
            }
        }
        UnlinkedRelationalConstraintKind::Required { model, relation } => {
            let model = resolve_model(models, &model, diagnostics)?;
            let relation = resolve_relation(relations, &relation, diagnostics)?;
            require_binary_cardinality(relation, value.span, diagnostics)?;
            require_cardinality_anchor(model, relation, value.span, diagnostics)?;
            RelationalConstraintKind::Required {
                relation_id: relation.id.clone(),
            }
        }
        UnlinkedRelationalConstraintKind::Unique { model, relation } => {
            let model = resolve_model(models, &model, diagnostics)?;
            let relation = resolve_relation(relations, &relation, diagnostics)?;
            require_binary_cardinality(relation, value.span, diagnostics)?;
            require_cardinality_anchor(model, relation, value.span, diagnostics)?;
            RelationalConstraintKind::Unique {
                relation_id: relation.id.clone(),
            }
        }
        UnlinkedRelationalConstraintKind::Exclusive {
            relations: references,
        } => RelationalConstraintKind::Exclusive {
            relation_ids: resolve_relation_group(relations, &references, value.span, diagnostics)?,
        },
        UnlinkedRelationalConstraintKind::Exhaustive {
            relations: references,
        } => RelationalConstraintKind::Exhaustive {
            relation_ids: resolve_relation_group(relations, &references, value.span, diagnostics)?,
        },
        UnlinkedRelationalConstraintKind::Coexistent {
            relations: references,
        } => RelationalConstraintKind::Coexistent {
            relation_ids: resolve_relation_group(relations, &references, value.span, diagnostics)?,
        },
    };
    let generated = generated_relational_constraint_id(&constraint);
    let declaration = UnlinkedDeclaration {
        id: Some(generated),
        ..value.declaration
    };
    let id = canonical_member(&declaration, module_id, diagnostics)?;
    duplicate_id(&id, declaration.span, top_level_ids, diagnostics);
    Some(RelationalConstraintDefinition {
        id,
        constraint,
        span: value.span,
    })
}

fn require_binary_cardinality(
    relation: &RelationDefinition,
    span: TextRange,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<()> {
    if relation.parameter_model_ids.len() == 2 {
        Some(())
    } else {
        diagnostics.push(
            Diagnostic::error(
                "RSPDL-REL-002",
                "semantic.relation.cardinality_requires_binary",
                span,
            )
            .with_argument("relation_id", &relation.id),
        );
        None
    }
}

fn require_cardinality_anchor(
    model: &DataModelDefinition,
    relation: &RelationDefinition,
    span: TextRange,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<()> {
    let actual_model_id = relation.parameter_model_ids.first()?;
    if actual_model_id == &model.id {
        Some(())
    } else {
        diagnostics.push(
            Diagnostic::error(
                "RSPDL-REL-002",
                "semantic.relation.cardinality_anchor_mismatch",
                span,
            )
            .with_argument("relation_id", &relation.id)
            .with_argument("expected_model_id", actual_model_id)
            .with_argument("actual_model_id", &model.id),
        );
        None
    }
}

fn resolve_relation_group(
    definitions: &[RelationDefinition],
    references: &[SurfaceRef],
    span: TextRange,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Vec<CanonicalId>> {
    let mut relation_ids = references
        .iter()
        .filter_map(|reference| resolve_relation(definitions, reference, diagnostics))
        .map(|relation| relation.id.clone())
        .collect::<Vec<_>>();
    if relation_ids.len() != references.len() {
        return None;
    }
    relation_ids.sort();
    let original_len = relation_ids.len();
    relation_ids.dedup();
    if relation_ids.len() != original_len || relation_ids.len() < 2 {
        diagnostics.push(Diagnostic::error(
            "RSPDL-REL-003",
            "semantic.relation.group_requires_distinct_members",
            span,
        ));
        return None;
    }
    let signature = definitions
        .iter()
        .find(|relation| relation.id == relation_ids[0])?
        .parameter_model_ids
        .clone();
    if relation_ids.iter().any(|id| {
        definitions
            .iter()
            .find(|relation| &relation.id == id)
            .is_none_or(|relation| relation.parameter_model_ids != signature)
    }) {
        diagnostics.push(Diagnostic::error(
            "RSPDL-REL-003",
            "semantic.relation.group_signature_mismatch",
            span,
        ));
        return None;
    }
    Some(relation_ids)
}

fn validate_relation_compatibility(
    constraints: &[(RelationalConstraintDefinition, TextRange)],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut exclusive = constraints
        .iter()
        .filter_map(|(definition, span)| match &definition.constraint {
            RelationalConstraintKind::Exclusive { relation_ids } => {
                Some((relation_ids, definition, span))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    exclusive.sort_by(|(_, left, _), (_, right, _)| left.id.cmp(&right.id));
    for (definition, span) in constraints {
        let RelationalConstraintKind::Coexistent { relation_ids } = &definition.constraint else {
            continue;
        };
        let conflict = relation_ids.iter().enumerate().find_map(|(index, left)| {
            relation_ids.iter().skip(index + 1).find_map(|right| {
                exclusive
                    .iter()
                    .find(|(ids, _, _)| ids.contains(left) && ids.contains(right))
                    .map(|(_, rule, _)| (left, right, *rule))
            })
        });
        if let Some((left, right, exclusive)) = conflict {
            diagnostics.push(
                Diagnostic::error(
                    "RSPDL-REL-004",
                    "semantic.relation.compatibility_conflict",
                    *span,
                )
                .with_argument("exclusive_rule_id", &exclusive.id)
                .with_argument("coexistent_rule_id", &definition.id)
                .with_argument("relation_ids", format!("{left},{right}")),
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn lower_model(
    value: UnlinkedDataModel,
    module_id: &CanonicalId,
    enums: &BTreeMap<String, EnumDefinition>,
    declared_model_ids: &BTreeSet<CanonicalId>,
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
    if value.fields.is_empty() {
        diagnostics.push(Diagnostic::error(
            "RSPDL-DATA-007",
            "semantic.model.field_required",
            value.declaration.span,
        ));
        return;
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
        let value_type = link_field_type(
            field.value_type,
            enums,
            module_id,
            declared_model_ids,
            field.declaration.span,
            diagnostics,
        );
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
            span: field.span,
        });
    }
    models.push(DataModelDefinition {
        id,
        name: value.declaration.name,
        fields,
        span: value.span,
    });
}

fn link_field_type(
    value: UnlinkedTypeReference,
    enums: &BTreeMap<String, EnumDefinition>,
    module_id: &CanonicalId,
    declared_model_ids: &BTreeSet<CanonicalId>,
    span: TextRange,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<CanonicalType> {
    let value_type = match value {
        UnlinkedTypeReference::String => CanonicalType::String,
        UnlinkedTypeReference::Integer => CanonicalType::Integer,
        UnlinkedTypeReference::Boolean => CanonicalType::Boolean,
        UnlinkedTypeReference::Decimal => CanonicalType::Decimal,
        UnlinkedTypeReference::Date => CanonicalType::Date,
        UnlinkedTypeReference::Time => CanonicalType::Time,
        UnlinkedTypeReference::DateTime => CanonicalType::DateTime,
        UnlinkedTypeReference::Duration => CanonicalType::Duration,
        UnlinkedTypeReference::Latitude => CanonicalType::Latitude,
        UnlinkedTypeReference::Longitude => CanonicalType::Longitude,
        UnlinkedTypeReference::Money(currency) => match crate::CurrencyCode::new(currency) {
            Ok(currency) => CanonicalType::Money(currency),
            Err(error) => {
                diagnostics.push(model_error("RSPDL-TYPE-001", error, span));
                return None;
            }
        },
        UnlinkedTypeReference::Percentage => CanonicalType::Percentage,
        UnlinkedTypeReference::Quantity(unit) => {
            match CanonicalValue::quantity_from_str(format!("1 {unit}")) {
                Ok(value) => value.value_type().clone(),
                Err(error) => {
                    diagnostics.push(model_error("RSPDL-TYPE-001", error, span));
                    return None;
                }
            }
        }
        UnlinkedTypeReference::Coordinate => CanonicalType::Coordinate,
        UnlinkedTypeReference::LocalDateTime => CanonicalType::LocalDateTime,
        UnlinkedTypeReference::ZonedDateTime => CanonicalType::ZonedDateTime,
        UnlinkedTypeReference::CalendarDuration => CanonicalType::CalendarDuration,
        UnlinkedTypeReference::Uuid => CanonicalType::Uuid,
        UnlinkedTypeReference::Email => CanonicalType::Email,
        UnlinkedTypeReference::Url => CanonicalType::Url,
        UnlinkedTypeReference::PhoneNumber => CanonicalType::PhoneNumber,
        UnlinkedTypeReference::IpAddress => CanonicalType::IpAddress,
        UnlinkedTypeReference::Cidr => CanonicalType::Cidr,
        UnlinkedTypeReference::CountryCode => CanonicalType::CountryCode,
        UnlinkedTypeReference::LanguageCode => CanonicalType::LanguageCode,
        UnlinkedTypeReference::CurrencyCode => CanonicalType::CurrencyCode,
        UnlinkedTypeReference::List(element) => CanonicalType::List(Box::new(link_field_type(
            *element,
            enums,
            module_id,
            declared_model_ids,
            span,
            diagnostics,
        )?)),
        UnlinkedTypeReference::Set(element) => CanonicalType::Set(Box::new(link_field_type(
            *element,
            enums,
            module_id,
            declared_model_ids,
            span,
            diagnostics,
        )?)),
        UnlinkedTypeReference::Map { key, value } => CanonicalType::map(
            link_field_type(
                *key,
                enums,
                module_id,
                declared_model_ids,
                span,
                diagnostics,
            )?,
            link_field_type(
                *value,
                enums,
                module_id,
                declared_model_ids,
                span,
                diagnostics,
            )?,
        )
        .map_err(|error| diagnostics.push(model_error("RSPDL-TYPE-001", error, span)))
        .ok()?,
        UnlinkedTypeReference::Reference(reference) => {
            match qualify_member_id(module_id, reference.id()) {
                Ok(model) if declared_model_ids.contains(&model) => CanonicalType::Reference(model),
                Ok(model) => {
                    diagnostics.push(
                        link_error("semantic.reference.target_not_found", span)
                            .with_argument("reference", model),
                    );
                    return None;
                }
                Err(error) => {
                    diagnostics.push(model_error("RSPDL-TYPE-001", error, span));
                    return None;
                }
            }
        }
        UnlinkedTypeReference::Named(reference) => {
            resolve_enum(enums.values(), &reference, diagnostics)
                .map(|definition| CanonicalType::Enum(definition.enum_type.clone()))?
        }
    };
    Some(value_type)
}

fn qualify_member_id(module_id: &CanonicalId, value: &str) -> Result<CanonicalId, ModelError> {
    if value.contains('.') {
        CanonicalId::new(value)
    } else {
        CanonicalId::new(format!("{module_id}.{value}"))
    }
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
    ) && !left_type.as_ref().is_some_and(CanonicalType::is_ordered)
    {
        diagnostics.push(type_error(
            "semantic.constraint.order_requires_ordered_type",
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
        span: value.span,
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
        span: value.span,
    })
}

#[derive(Clone)]
struct ResolvedCreationBranch {
    id: CanonicalId,
    input_id: CanonicalId,
    variant_id: CanonicalId,
    decision: CreationDecision,
    span: TextRange,
}

/// Lowers the first conditional-production slice: one closed enum action input
/// partitions creation of exactly one output record. The production span is
/// retained from the canonically first branch ID, never source order or span.
#[allow(clippy::too_many_arguments)]
fn analyze_conditional_productions(
    values: Vec<UnlinkedCreationBranch>,
    field_producer_values: Vec<UnlinkedFieldProducer>,
    relation_producer_values: Vec<crate::UnlinkedRelationProducer>,
    module_id: &CanonicalId,
    actions: &[ActionDefinition],
    action_names: &BTreeMap<String, CanonicalId>,
    events: &[EventDefinition],
    event_names: &BTreeMap<String, CanonicalId>,
    models: &BTreeMap<String, DataModelDefinition>,
    enums: &[EnumDefinition],
    relations: &[RelationDefinition],
    relational_constraints: &[RelationalConstraintDefinition],
    top_level_ids: &mut BTreeSet<CanonicalId>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<ConditionalProductionDefinition> {
    let mut branches_by_production = BTreeMap::<
        (ProductionTriggerKind, CanonicalId, CanonicalId),
        Vec<ResolvedCreationBranch>,
    >::new();

    for value in values {
        let Some(branch_id) = canonical_member(&value.declaration, module_id, diagnostics) else {
            continue;
        };
        duplicate_id(
            &branch_id,
            value.declaration.span,
            top_level_ids,
            diagnostics,
        );
        let trigger_id = match value.trigger.kind {
            ProductionTriggerKind::Action => resolve_named_id(
                "action",
                action_names,
                &value.trigger.reference,
                diagnostics,
            ),
            ProductionTriggerKind::Event => {
                resolve_named_id("event", event_names, &value.trigger.reference, diagnostics)
            }
        };
        let Some(trigger_id) = trigger_id else {
            continue;
        };
        match (value.trigger.kind, value.action.as_ref()) {
            (ProductionTriggerKind::Event, Some(legacy_action)) => {
                diagnostics.push(with_trigger_arguments(
                    Diagnostic::error(
                        "RSPDL-PROD-007",
                        "semantic.creation_branch.legacy_action_incompatible",
                        value.span,
                    )
                    .with_argument("branch_id", &branch_id)
                    .with_argument("legacy_action_reference", legacy_action.id()),
                    ProductionTriggerKind::Event,
                    &trigger_id,
                ));
                continue;
            }
            (ProductionTriggerKind::Action, Some(legacy_action)) => {
                let Some(legacy_action_id) =
                    resolve_named_id("action", action_names, legacy_action, diagnostics)
                else {
                    continue;
                };
                if legacy_action_id != trigger_id {
                    diagnostics.push(with_trigger_arguments(
                        Diagnostic::error(
                            "RSPDL-PROD-007",
                            "semantic.creation_branch.legacy_action_incompatible",
                            value.span,
                        )
                        .with_argument("branch_id", &branch_id)
                        .with_argument("legacy_action_id", legacy_action_id),
                        ProductionTriggerKind::Action,
                        &trigger_id,
                    ));
                    continue;
                }
            }
            (_, None) => {}
        }
        let Some(output_model) = resolve_model(models, &value.output_model, diagnostics) else {
            continue;
        };
        let input_and_enum = if value.trigger.kind == ProductionTriggerKind::Action {
            actions
                .iter()
                .find(|action| action.id == trigger_id)
                .and_then(|action| {
                    resolve_creation_decision_input(
                        action,
                        &value.input,
                        value.trigger.kind,
                        &trigger_id,
                        &output_model.id,
                        value.span,
                        diagnostics,
                    )
                    .map(|(input, enum_type)| (input.id.clone(), enum_type.id().clone()))
                })
        } else {
            events
                .iter()
                .find(|event| event.id == trigger_id)
                .and_then(|event| {
                    resolve_event_creation_decision_input(
                        event,
                        &value.input,
                        &trigger_id,
                        &output_model.id,
                        value.span,
                        diagnostics,
                    )
                    .map(|(input, enum_type)| (input.id.clone(), enum_type.id().clone()))
                })
        };
        let Some((input_id, enum_id)) = input_and_enum else {
            continue;
        };
        let Some(variant_id) = resolve_creation_variant(
            enums,
            enums
                .iter()
                .find(|definition| definition.id == enum_id)
                .map(|definition| &definition.enum_type)
                .expect("event/action enum was linked"),
            &value.variant,
            value.trigger.kind,
            &trigger_id,
            &output_model.id,
            &input_id,
            diagnostics,
        ) else {
            continue;
        };

        branches_by_production
            .entry((value.trigger.kind, trigger_id, output_model.id.clone()))
            .or_default()
            .push(ResolvedCreationBranch {
                id: branch_id,
                input_id,
                variant_id,
                decision: value.decision,
                span: value.span,
            });
    }

    let mut productions = Vec::new();
    for ((trigger_kind, trigger_id, output_model_id), mut branches) in branches_by_production {
        branches.sort_by(|left, right| {
            (&left.id, &left.variant_id, &left.decision).cmp(&(
                &right.id,
                &right.variant_id,
                &right.decision,
            ))
        });
        let representative_span = branches
            .first()
            .expect("a production group always has a branch")
            .span;
        let production_id =
            generated_production_id(module_id, trigger_kind, &trigger_id, &output_model_id)
                .expect("generated production IDs are always canonical");
        duplicate_id(
            &production_id,
            representative_span,
            top_level_ids,
            diagnostics,
        );

        let decision_input_ids = branches
            .iter()
            .map(|branch| branch.input_id.clone())
            .collect::<BTreeSet<_>>();
        if decision_input_ids.len() != 1 {
            diagnostics.push(with_trigger_arguments(
                Diagnostic::error(
                    "RSPDL-PROD-007",
                    "semantic.creation_production.mixed_decision_inputs",
                    representative_span,
                )
                .with_argument("production_id", &production_id)
                .with_argument("output_model_id", &output_model_id)
                .with_argument("input_ids", joined_ids_set(&decision_input_ids)),
                trigger_kind,
                &trigger_id,
            ));
            continue;
        }
        let decision_input_id = decision_input_ids
            .into_iter()
            .next()
            .expect("one decision input was checked above");
        let enum_type = if trigger_kind == ProductionTriggerKind::Action {
            actions
                .iter()
                .find(|action| action.id == trigger_id)
                .and_then(|action| {
                    action
                        .inputs
                        .iter()
                        .find(|input| input.id == decision_input_id)
                        .and_then(enum_input_type)
                })
        } else {
            events
                .iter()
                .find(|event| event.id == trigger_id)
                .and_then(|event| {
                    event
                        .inputs
                        .iter()
                        .find(|input| input.id == decision_input_id)
                        .and_then(event_input_type)
                })
        }
        .expect("only enum decision inputs reach a production");

        let branch_ids_by_variant = branches.iter().fold(
            BTreeMap::<CanonicalId, Vec<CanonicalId>>::new(),
            |mut grouped, branch| {
                grouped
                    .entry(branch.variant_id.clone())
                    .or_default()
                    .push(branch.id.clone());
                grouped
            },
        );
        for (variant_id, branch_ids) in &branch_ids_by_variant {
            if branch_ids.len() > 1 {
                let conflict_span = branches
                    .iter()
                    .find(|branch| branch.id == branch_ids[0])
                    .expect("a grouped branch ID came from this production")
                    .span;
                diagnostics.push(with_trigger_arguments(
                    Diagnostic::error(
                        "RSPDL-POLICY-007",
                        "semantic.creation_production.variant_conflict",
                        conflict_span,
                    )
                    .with_argument("production_id", &production_id)
                    .with_argument("output_model_id", &output_model_id)
                    .with_argument("input_id", &decision_input_id)
                    .with_argument("variant_id", variant_id)
                    .with_argument("branch_ids", joined_ids_csv(branch_ids)),
                    trigger_kind,
                    &trigger_id,
                ));
            }
        }
        let covered_variant_ids = branch_ids_by_variant
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        let missing_variant_ids = enum_type
            .variants()
            .difference(&covered_variant_ids)
            .cloned()
            .collect::<Vec<_>>();
        if !missing_variant_ids.is_empty() {
            diagnostics.push(with_trigger_arguments(
                Diagnostic::error(
                    "RSPDL-POLICY-008",
                    "semantic.creation_production.variant_coverage_missing",
                    representative_span,
                )
                .with_argument("production_id", &production_id)
                .with_argument("output_model_id", &output_model_id)
                .with_argument("input_id", &decision_input_id)
                .with_argument("missing_variant_ids", joined_ids_csv(&missing_variant_ids)),
                trigger_kind,
                &trigger_id,
            ));
        }

        productions.push(ConditionalProductionDefinition {
            id: production_id,
            action_id: (trigger_kind == ProductionTriggerKind::Action)
                .then_some(trigger_id.clone()),
            trigger: match trigger_kind {
                ProductionTriggerKind::Action => {
                    ProductionTriggerDefinition::Action(trigger_id.clone())
                }
                ProductionTriggerKind::Event => {
                    ProductionTriggerDefinition::Event(trigger_id.clone())
                }
            },
            output_model_id: output_model_id.clone(),
            instance_cardinality: ProductionCardinality::ExactlyOne,
            decision_input_id,
            branches: branches
                .into_iter()
                .map(|branch| CreationBranchDefinition {
                    id: branch.id,
                    variant_id: branch.variant_id,
                    decision: branch.decision,
                    span: branch.span,
                })
                .collect(),
            field_producers: Vec::new(),
            field_evaluation_order: Vec::new(),
            relation_slots: output_relation_slots(
                &output_model_id,
                relations,
                relational_constraints,
            ),
            relation_producers: Vec::new(),
            span: representative_span,
        });
    }
    productions.sort_by(|left, right| {
        (&left.trigger, &left.output_model_id, &left.id).cmp(&(
            &right.trigger,
            &right.output_model_id,
            &right.id,
        ))
    });
    analyze_field_producers(
        field_producer_values,
        module_id,
        actions,
        action_names,
        events,
        event_names,
        models,
        enums,
        top_level_ids,
        diagnostics,
        &mut productions,
    );
    analyze_relation_producers(
        relation_producer_values,
        module_id,
        actions,
        action_names,
        events,
        event_names,
        models,
        relations,
        top_level_ids,
        diagnostics,
        &mut productions,
    );
    productions
}

struct ResolvedFieldProducerSource {
    source: FieldProducerSource,
    value_type: CanonicalType,
    evidence: String,
}

/// `action` is a legacy Action-only projection on unlinked producers. The
/// tagged trigger owns semantics; reject a contradictory legacy projection so
/// a frontend cannot smuggle an Action owner into an Event producer.
fn producer_legacy_action_is_compatible(
    legacy_action: Option<&SurfaceRef>,
    trigger_kind: ProductionTriggerKind,
    trigger_id: &CanonicalId,
    action_names: &BTreeMap<String, CanonicalId>,
    producer_id: &CanonicalId,
    span: TextRange,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    match (trigger_kind, legacy_action) {
        (ProductionTriggerKind::Event, Some(legacy_action)) => {
            diagnostics.push(with_trigger_arguments(
                Diagnostic::error(
                    "RSPDL-PROD-007",
                    "semantic.producer.legacy_action_incompatible",
                    span,
                )
                .with_argument("producer_id", producer_id)
                .with_argument("legacy_action_reference", legacy_action.id()),
                trigger_kind,
                trigger_id,
            ));
            false
        }
        (ProductionTriggerKind::Action, Some(legacy_action)) => {
            let Some(legacy_id) =
                resolve_named_id("action", action_names, legacy_action, diagnostics)
            else {
                return false;
            };
            if legacy_id == *trigger_id {
                true
            } else {
                diagnostics.push(with_trigger_arguments(
                    Diagnostic::error(
                        "RSPDL-PROD-007",
                        "semantic.producer.legacy_action_incompatible",
                        span,
                    )
                    .with_argument("producer_id", producer_id)
                    .with_argument("legacy_action_id", legacy_id),
                    trigger_kind,
                    trigger_id,
                ));
                false
            }
        }
        (_, None) => true,
    }
}

/// Links field producers. They do not create a production: they only enrich a
/// previously linked action/output decision. A conditional producer is scoped
/// to exactly one variant of that production's direct enum decision input.
#[allow(clippy::too_many_arguments)]
fn analyze_field_producers(
    values: Vec<UnlinkedFieldProducer>,
    module_id: &CanonicalId,
    actions: &[ActionDefinition],
    action_names: &BTreeMap<String, CanonicalId>,
    events: &[EventDefinition],
    event_names: &BTreeMap<String, CanonicalId>,
    models: &BTreeMap<String, DataModelDefinition>,
    enums: &[EnumDefinition],
    top_level_ids: &mut BTreeSet<CanonicalId>,
    diagnostics: &mut Vec<Diagnostic>,
    productions: &mut [ConditionalProductionDefinition],
) {
    let production_indexes = productions
        .iter()
        .enumerate()
        .map(|(index, production)| match &production.trigger {
            ProductionTriggerDefinition::Action(trigger_id) => (
                (
                    ProductionTriggerKind::Action,
                    trigger_id.clone(),
                    production.output_model_id.clone(),
                ),
                index,
            ),
            ProductionTriggerDefinition::Event(trigger_id) => (
                (
                    ProductionTriggerKind::Event,
                    trigger_id.clone(),
                    production.output_model_id.clone(),
                ),
                index,
            ),
        })
        .collect::<BTreeMap<_, _>>();

    for value in values {
        let Some(producer_id) = canonical_member(&value.declaration, module_id, diagnostics) else {
            continue;
        };
        duplicate_id(
            &producer_id,
            value.declaration.span,
            top_level_ids,
            diagnostics,
        );
        let trigger_id = match value.trigger.kind {
            ProductionTriggerKind::Action => resolve_named_id(
                "action",
                action_names,
                &value.trigger.reference,
                diagnostics,
            ),
            ProductionTriggerKind::Event => {
                resolve_named_id("event", event_names, &value.trigger.reference, diagnostics)
            }
        };
        let Some(trigger_id) = trigger_id else {
            continue;
        };
        if !producer_legacy_action_is_compatible(
            value.action.as_ref(),
            value.trigger.kind,
            &trigger_id,
            action_names,
            &producer_id,
            value.span,
            diagnostics,
        ) {
            continue;
        }
        let Some(output_model) = resolve_model(models, &value.output_model, diagnostics) else {
            continue;
        };
        let Some(&production_index) = production_indexes.get(&(
            value.trigger.kind,
            trigger_id.clone(),
            output_model.id.clone(),
        )) else {
            diagnostics.push(with_trigger_arguments(
                Diagnostic::error(
                    "RSPDL-PROD-007",
                    "semantic.creation_production.field_producer_without_creation_decision",
                    value.span,
                )
                .with_argument("producer_id", &producer_id)
                .with_argument("output_model_id", &output_model.id),
                value.trigger.kind,
                &trigger_id,
            ));
            continue;
        };
        let Some(output_field) = resolve_field(output_model, &value.output_field, diagnostics)
        else {
            continue;
        };
        let (condition, source, phase) = match value.trigger.kind {
            ProductionTriggerKind::Action => {
                let action = actions
                    .iter()
                    .find(|action| action.id == trigger_id)
                    .expect("a production action was linked from this action table");
                let Some(condition) = resolve_field_producer_condition(
                    value.condition.as_ref(),
                    action,
                    enums,
                    &producer_id,
                    &productions[production_index],
                    value.span,
                    diagnostics,
                ) else {
                    continue;
                };
                let Some(source) = resolve_field_producer_source(
                    &value.source,
                    &producer_id,
                    action,
                    &trigger_id,
                    &output_model.id,
                    &output_field.id,
                    &output_field.value_type,
                    models,
                    enums,
                    output_model,
                    value.span,
                    diagnostics,
                ) else {
                    continue;
                };
                (condition, source, ProducerPhase::PreMutation)
            }
            ProductionTriggerKind::Event => {
                if value.condition.is_some() {
                    diagnostics.push(with_trigger_arguments(
                        Diagnostic::error(
                            "RSPDL-PROD-007",
                            "semantic.event_field_producer.conditional_unsupported",
                            value.span,
                        )
                        .with_argument("producer_id", &producer_id)
                        .with_argument("output_model_id", &output_model.id),
                        ProductionTriggerKind::Event,
                        &trigger_id,
                    ));
                    continue;
                }
                let event = events
                    .iter()
                    .find(|event| event.id == trigger_id)
                    .expect("a production event was linked from this event table");
                let Some(source) = resolve_event_field_producer_source(
                    &value.source,
                    &producer_id,
                    event,
                    &trigger_id,
                    &output_model.id,
                    &output_field.id,
                    &output_field.value_type,
                    models,
                    output_model,
                    value.span,
                    diagnostics,
                ) else {
                    continue;
                };
                (None, source, ProducerPhase::TriggerPayload)
            }
        };
        if source.value_type != output_field.value_type {
            let diagnostic = match value.trigger.kind {
                ProductionTriggerKind::Action => field_producer_type_error(
                    value.span,
                    &producer_id,
                    &trigger_id,
                    &output_model.id,
                    &output_field.id,
                    &source.evidence,
                    &output_field.value_type,
                ),
                ProductionTriggerKind::Event => with_trigger_arguments(
                    Diagnostic::error(
                        "RSPDL-PROD-002",
                        "semantic.field_producer.type_mismatch",
                        value.span,
                    )
                    .with_argument("producer_id", &producer_id)
                    .with_argument("output_model_id", &output_model.id)
                    .with_argument("output_field_id", &output_field.id)
                    .with_argument("source", &source.evidence)
                    .with_argument("output_type", &output_field.value_type),
                    ProductionTriggerKind::Event,
                    &trigger_id,
                ),
            };
            diagnostics.push(diagnostic.with_argument("source_type", &source.value_type));
            continue;
        }
        productions[production_index]
            .field_producers
            .push(FieldProducerDefinition {
                id: producer_id,
                output_field_id: output_field.id.clone(),
                source: source.source,
                condition,
                phase,
                span: value.span,
            });
    }

    for production in productions {
        production
            .field_producers
            .sort_by(|left, right| left.id.cmp(&right.id));
        production.field_evaluation_order =
            canonical_field_evaluation_order(&production.field_producers);
        let create_branches_by_variant = production.branches.iter().fold(
            BTreeMap::<CanonicalId, Vec<&CreationBranchDefinition>>::new(),
            |mut grouped, branch| {
                if branch.decision == CreationDecision::Create {
                    grouped
                        .entry(branch.variant_id.clone())
                        .or_default()
                        .push(branch);
                }
                grouped
            },
        );
        if create_branches_by_variant.is_empty() {
            continue;
        }
        let output_model = models
            .values()
            .find(|model| model.id == production.output_model_id)
            .expect("production output was linked above");
        for (variant_id, create_branches) in &create_branches_by_variant {
            let create_branch_ids = create_branches
                .iter()
                .map(|branch| branch.id.clone())
                .collect::<Vec<_>>();
            let witness_span = create_branches[0].span;
            for field in &output_model.fields {
                let producers = production
                    .field_producers
                    .iter()
                    .filter(|producer| {
                        producer.output_field_id == field.id
                            && producer_applies_to_variant(
                                producer,
                                &production.decision_input_id,
                                variant_id,
                            )
                    })
                    .collect::<Vec<_>>();
                if producers.is_empty() && field.required {
                    diagnostics.push(with_production_trigger(
                        Diagnostic::error(
                            "RSPDL-PROD-003",
                            "semantic.creation_production.required_field_producer_missing",
                            witness_span,
                        )
                        .with_argument("production_id", &production.id)
                        .with_argument("output_model_id", &production.output_model_id)
                        .with_argument("field_id", &field.id)
                        .with_argument("variant_id", variant_id)
                        .with_argument("create_branch_ids", joined_ids_csv(&create_branch_ids)),
                        production,
                    ));
                }
                if producers.len() > 1 {
                    diagnostics.push(with_production_trigger(
                        Diagnostic::error(
                            "RSPDL-PROD-004",
                            "semantic.creation_production.field_producer_conflict",
                            producers[0].span,
                        )
                        .with_argument("production_id", &production.id)
                        .with_argument("output_model_id", &production.output_model_id)
                        .with_argument("field_id", &field.id)
                        .with_argument(
                            "producer_ids",
                            joined_ids_csv(
                                &producers
                                    .iter()
                                    .map(|producer| producer.id.clone())
                                    .collect::<Vec<_>>(),
                            ),
                        )
                        .with_argument("variant_id", variant_id)
                        .with_argument("create_branch_ids", joined_ids_csv(&create_branch_ids)),
                        production,
                    ));
                }
            }
        }
        analyze_template_dependencies(
            production,
            output_model,
            &create_branches_by_variant,
            diagnostics,
        );
    }
}

fn canonical_field_evaluation_order(producers: &[FieldProducerDefinition]) -> Vec<CanonicalId> {
    let mut remaining = producers
        .iter()
        .map(|producer| (producer.output_field_id.clone(), BTreeSet::new()))
        .collect::<BTreeMap<_, BTreeSet<_>>>();
    for producer in producers {
        if let FieldProducerSource::Template { parts } = &producer.source {
            for part in parts {
                if let TemplatePart::OutputField { field_id } = part {
                    remaining
                        .entry(producer.output_field_id.clone())
                        .or_default()
                        .insert(field_id.clone());
                }
            }
        }
    }
    let mut order = Vec::new();
    while !remaining.is_empty() {
        let ready = remaining
            .iter()
            .filter(|(_, dependencies)| dependencies.is_empty())
            .map(|(field, _)| field.clone())
            .collect::<Vec<_>>();
        if ready.is_empty() {
            order.extend(remaining.keys().cloned());
            break;
        }
        for field in ready {
            remaining.remove(&field);
            for dependencies in remaining.values_mut() {
                dependencies.remove(&field);
            }
            order.push(field);
        }
    }
    order
}

/// Every production diagnostic identifies its tagged trigger.  `action_id`
/// remains an Action-only compatibility field; Event findings intentionally do
/// not serialize an action-shaped value.
fn with_trigger_arguments(
    diagnostic: Diagnostic,
    trigger_kind: ProductionTriggerKind,
    trigger_id: &CanonicalId,
) -> Diagnostic {
    let kind = match trigger_kind {
        ProductionTriggerKind::Action => "action",
        ProductionTriggerKind::Event => "event",
    };
    let diagnostic = diagnostic
        .with_argument("trigger_kind", kind)
        .with_argument("trigger_id", trigger_id);
    if trigger_kind == ProductionTriggerKind::Action {
        diagnostic.with_argument("action_id", trigger_id)
    } else {
        diagnostic
    }
}

fn with_production_trigger(
    diagnostic: Diagnostic,
    production: &ConditionalProductionDefinition,
) -> Diagnostic {
    match &production.trigger {
        ProductionTriggerDefinition::Action(trigger_id) => {
            with_trigger_arguments(diagnostic, ProductionTriggerKind::Action, trigger_id)
        }
        ProductionTriggerDefinition::Event(trigger_id) => {
            with_trigger_arguments(diagnostic, ProductionTriggerKind::Event, trigger_id)
        }
    }
}

/// Templates are producers themselves, but their placeholders additionally
/// depend on other output fields.  Check the graph per effective Create
/// variant so optional fields cannot be silently absent when interpolated.
fn analyze_template_dependencies(
    production: &ConditionalProductionDefinition,
    output_model: &DataModelDefinition,
    create_branches_by_variant: &BTreeMap<CanonicalId, Vec<&CreationBranchDefinition>>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for (variant_id, branches) in create_branches_by_variant {
        let branch_ids = branches
            .iter()
            .map(|branch| branch.id.clone())
            .collect::<Vec<_>>();
        let mut graph = BTreeMap::<CanonicalId, BTreeSet<CanonicalId>>::new();
        for producer in &production.field_producers {
            if !producer_applies_to_variant(producer, &production.decision_input_id, variant_id) {
                continue;
            }
            let FieldProducerSource::Template { parts } = &producer.source else {
                continue;
            };
            let dependencies = parts
                .iter()
                .filter_map(|part| match part {
                    TemplatePart::OutputField { field_id } => Some(field_id.clone()),
                    TemplatePart::Text { .. } => None,
                })
                .collect::<BTreeSet<_>>();
            for field_id in dependencies {
                let dependency_producers = production
                    .field_producers
                    .iter()
                    .filter(|candidate| {
                        candidate.output_field_id == field_id
                            && producer_applies_to_variant(
                                candidate,
                                &production.decision_input_id,
                                variant_id,
                            )
                    })
                    .collect::<Vec<_>>();
                if dependency_producers.is_empty() {
                    diagnostics.push(with_production_trigger(
                        Diagnostic::error(
                            "RSPDL-PROD-003",
                            "semantic.template.dependency_producer_missing",
                            producer.span,
                        )
                        .with_argument("production_id", &production.id)
                        .with_argument("output_model_id", &production.output_model_id)
                        .with_argument("target_field_id", &producer.output_field_id)
                        .with_argument("dependency_field_id", &field_id)
                        .with_argument("variant_id", variant_id)
                        .with_argument("create_branch_ids", joined_ids_csv(&branch_ids)),
                        production,
                    ));
                }
                if dependency_producers.len() > 1 {
                    diagnostics.push(with_production_trigger(
                        Diagnostic::error(
                            "RSPDL-PROD-004",
                            "semantic.template.dependency_producer_conflict",
                            producer.span,
                        )
                        .with_argument("production_id", &production.id)
                        .with_argument("output_model_id", &production.output_model_id)
                        .with_argument("target_field_id", &producer.output_field_id)
                        .with_argument("dependency_field_id", &field_id)
                        .with_argument(
                            "producer_ids",
                            joined_ids_csv(
                                &dependency_producers
                                    .iter()
                                    .map(|candidate| candidate.id.clone())
                                    .collect::<Vec<_>>(),
                            ),
                        )
                        .with_argument("variant_id", variant_id)
                        .with_argument("create_branch_ids", joined_ids_csv(&branch_ids)),
                        production,
                    ));
                }
                graph
                    .entry(producer.output_field_id.clone())
                    .or_default()
                    .insert(field_id);
            }
        }
        if let Some(cycle) = canonical_template_cycle(&graph) {
            diagnostics.push(with_production_trigger(
                Diagnostic::error(
                    "RSPDL-PROD-008",
                    "semantic.template.dependency_cycle",
                    branches[0].span,
                )
                .with_argument("production_id", &production.id)
                .with_argument("output_model_id", &production.output_model_id)
                .with_argument("variant_id", variant_id)
                .with_argument("create_branch_ids", joined_ids_csv(&branch_ids))
                .with_argument("cycle_field_ids", joined_ids_csv(&cycle)),
                production,
            ));
        }
    }
    let _ = output_model; // documents the same-model contract at the call site.
}

fn canonical_template_cycle(
    graph: &BTreeMap<CanonicalId, BTreeSet<CanonicalId>>,
) -> Option<Vec<CanonicalId>> {
    fn visit(
        node: &CanonicalId,
        graph: &BTreeMap<CanonicalId, BTreeSet<CanonicalId>>,
        visited: &mut BTreeSet<CanonicalId>,
        stack: &mut Vec<CanonicalId>,
    ) -> Option<Vec<CanonicalId>> {
        if let Some(index) = stack.iter().position(|value| value == node) {
            let mut cycle = stack[index..].to_vec();
            cycle.sort();
            cycle.dedup();
            return Some(cycle);
        }
        if !visited.insert(node.clone()) {
            return None;
        }
        stack.push(node.clone());
        if let Some(next) = graph.get(node) {
            for dependency in next {
                if let Some(cycle) = visit(dependency, graph, visited, stack) {
                    return Some(cycle);
                }
            }
        }
        stack.pop();
        None
    }
    let mut visited = BTreeSet::new();
    for node in graph.keys() {
        if let Some(cycle) = visit(node, graph, &mut visited, &mut Vec::new()) {
            return Some(cycle);
        }
    }
    None
}

fn producer_applies_to_variant(
    producer: &FieldProducerDefinition,
    decision_input_id: &CanonicalId,
    variant_id: &CanonicalId,
) -> bool {
    match &producer.condition {
        None => true,
        Some(FieldProducerCondition::EnumVariant {
            input_id,
            variant_id: producer_variant,
        }) => input_id == decision_input_id && producer_variant == variant_id,
    }
}

fn output_relation_slots(
    output_model_id: &CanonicalId,
    relations: &[RelationDefinition],
    constraints: &[RelationalConstraintDefinition],
) -> Vec<OutputRelationSlotDefinition> {
    let required = constraints
        .iter()
        .filter_map(|constraint| match &constraint.constraint {
            RelationalConstraintKind::Required { relation_id } => Some(relation_id),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let unique = constraints
        .iter()
        .filter_map(|constraint| match &constraint.constraint {
            RelationalConstraintKind::Unique { relation_id } => Some(relation_id),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let mut slots = relations
        .iter()
        .filter(|relation| {
            relation.parameter_model_ids.len() == 2
                && relation.parameter_model_ids[0] == *output_model_id
                && required.contains(&relation.id)
                && unique.contains(&relation.id)
        })
        .map(|relation| OutputRelationSlotDefinition {
            relation_id: relation.id.clone(),
            output_model_id: output_model_id.clone(),
            endpoint_model_id: relation.parameter_model_ids[1].clone(),
            cardinality: RelationSlotCardinality::ExactlyOne,
            span: relation.span,
        })
        .collect::<Vec<_>>();
    slots.sort_by(|left, right| left.relation_id.cmp(&right.relation_id));
    slots
}

#[allow(clippy::too_many_arguments)]
fn analyze_relation_producers(
    values: Vec<UnlinkedRelationProducer>,
    module_id: &CanonicalId,
    actions: &[ActionDefinition],
    action_names: &BTreeMap<String, CanonicalId>,
    events: &[EventDefinition],
    event_names: &BTreeMap<String, CanonicalId>,
    models: &BTreeMap<String, DataModelDefinition>,
    relations: &[RelationDefinition],
    top_level_ids: &mut BTreeSet<CanonicalId>,
    diagnostics: &mut Vec<Diagnostic>,
    productions: &mut [ConditionalProductionDefinition],
) {
    let indexes = productions
        .iter()
        .enumerate()
        .map(|(index, production)| match &production.trigger {
            ProductionTriggerDefinition::Action(trigger_id) => (
                (
                    ProductionTriggerKind::Action,
                    trigger_id.clone(),
                    production.output_model_id.clone(),
                ),
                index,
            ),
            ProductionTriggerDefinition::Event(trigger_id) => (
                (
                    ProductionTriggerKind::Event,
                    trigger_id.clone(),
                    production.output_model_id.clone(),
                ),
                index,
            ),
        })
        .collect::<BTreeMap<_, _>>();
    for value in values {
        let Some(producer_id) = canonical_member(&value.declaration, module_id, diagnostics) else {
            continue;
        };
        duplicate_id(
            &producer_id,
            value.declaration.span,
            top_level_ids,
            diagnostics,
        );
        let trigger_id = match value.trigger.kind {
            ProductionTriggerKind::Action => resolve_named_id(
                "action",
                action_names,
                &value.trigger.reference,
                diagnostics,
            ),
            ProductionTriggerKind::Event => {
                resolve_named_id("event", event_names, &value.trigger.reference, diagnostics)
            }
        };
        let Some(trigger_id) = trigger_id else {
            continue;
        };
        if !producer_legacy_action_is_compatible(
            value.action.as_ref(),
            value.trigger.kind,
            &trigger_id,
            action_names,
            &producer_id,
            value.span,
            diagnostics,
        ) {
            continue;
        }
        let Some(output_model) = resolve_model(models, &value.output_model, diagnostics) else {
            continue;
        };
        let Some(&production_index) = indexes.get(&(
            value.trigger.kind,
            trigger_id.clone(),
            output_model.id.clone(),
        )) else {
            diagnostics.push(with_trigger_arguments(
                Diagnostic::error(
                    "RSPDL-PROD-007",
                    "semantic.creation_production.relation_producer_without_creation_decision",
                    value.span,
                )
                .with_argument("producer_id", &producer_id)
                .with_argument("output_model_id", &output_model.id),
                value.trigger.kind,
                &trigger_id,
            ));
            continue;
        };
        let Some(relation) = resolve_relation(relations, &value.relation, diagnostics) else {
            continue;
        };
        let production = &productions[production_index];
        let Some(slot) = production
            .relation_slots
            .iter()
            .find(|slot| slot.relation_id == relation.id)
        else {
            diagnostics.push(with_trigger_arguments(
                Diagnostic::error(
                    "RSPDL-PROD-007",
                    "semantic.creation_production.relation_producer_not_exactly_one_slot",
                    value.span,
                )
                .with_argument("producer_id", &producer_id)
                .with_argument("production_id", &production.id)
                .with_argument("output_model_id", &output_model.id)
                .with_argument("relation_id", &relation.id),
                value.trigger.kind,
                &trigger_id,
            ));
            continue;
        };
        let input = match value.trigger.kind {
            ProductionTriggerKind::Action => actions
                .iter()
                .find(|action| action.id == trigger_id)
                .and_then(|action| {
                    action.inputs.iter().find(|input| {
                        member_reference_matches(&input.id, &input.local_id, &value.input)
                    })
                })
                .map(|input| {
                    (
                        input.id.clone(),
                        match &input.kind {
                            ActionInputKind::ExistingModel { model_id } => Some(model_id.clone()),
                            ActionInputKind::Value { .. } => None,
                        },
                    )
                }),
            ProductionTriggerKind::Event => events
                .iter()
                .find(|event| event.id == trigger_id)
                .and_then(|event| {
                    event.inputs.iter().find(|input| {
                        member_reference_matches(&input.id, &input.local_id, &value.input)
                    })
                })
                .map(|input| {
                    (
                        input.id.clone(),
                        match &input.kind {
                            EventInputKind::ExistingModel { model_id } => Some(model_id.clone()),
                            EventInputKind::Value { .. } => None,
                        },
                    )
                }),
        };
        let Some((input_id, source_model_id)) = input else {
            diagnostics.push(with_trigger_arguments(
                Diagnostic::error(
                    "RSPDL-PROD-002",
                    "semantic.relation_producer.source_input_invalid",
                    value.input.span(),
                )
                .with_argument("producer_id", &producer_id)
                .with_argument("relation_id", &relation.id)
                .with_argument("source", value.input.id()),
                value.trigger.kind,
                &trigger_id,
            ));
            continue;
        };
        let matches_endpoint = source_model_id.as_ref() == Some(&slot.endpoint_model_id);
        if !matches_endpoint {
            diagnostics.push(with_trigger_arguments(
                Diagnostic::error(
                    "RSPDL-PROD-002",
                    "semantic.relation_producer.source_endpoint_mismatch",
                    value.span,
                )
                .with_argument("producer_id", &producer_id)
                .with_argument("output_model_id", &output_model.id)
                .with_argument("relation_id", &relation.id)
                .with_argument("input_id", &input_id)
                .with_argument("endpoint_model_id", &slot.endpoint_model_id),
                value.trigger.kind,
                &trigger_id,
            ));
            continue;
        }
        productions[production_index]
            .relation_producers
            .push(RelationProducerDefinition {
                id: producer_id,
                relation_id: relation.id.clone(),
                input_id,
                phase: match value.trigger.kind {
                    ProductionTriggerKind::Action => ProducerPhase::PreMutation,
                    ProductionTriggerKind::Event => ProducerPhase::TriggerPayload,
                },
                span: value.span,
            });
    }
    for production in productions {
        production
            .relation_producers
            .sort_by(|left, right| left.id.cmp(&right.id));
        let create_branches = production
            .branches
            .iter()
            .filter(|branch| branch.decision == CreationDecision::Create)
            .fold(
                BTreeMap::<CanonicalId, Vec<&CreationBranchDefinition>>::new(),
                |mut groups, branch| {
                    groups
                        .entry(branch.variant_id.clone())
                        .or_default()
                        .push(branch);
                    groups
                },
            );
        for (variant_id, branches) in create_branches {
            let branch_ids = branches
                .iter()
                .map(|branch| branch.id.clone())
                .collect::<Vec<_>>();
            for slot in &production.relation_slots {
                let producers = production
                    .relation_producers
                    .iter()
                    .filter(|producer| producer.relation_id == slot.relation_id)
                    .collect::<Vec<_>>();
                if producers.is_empty() {
                    diagnostics.push(with_production_trigger(
                        Diagnostic::error(
                            "RSPDL-PROD-003",
                            "semantic.creation_production.required_relation_producer_missing",
                            branches[0].span,
                        )
                        .with_argument("production_id", &production.id)
                        .with_argument("output_model_id", &production.output_model_id)
                        .with_argument("relation_id", &slot.relation_id)
                        .with_argument("variant_id", &variant_id)
                        .with_argument("create_branch_ids", joined_ids_csv(&branch_ids)),
                        production,
                    ));
                }
                if producers.len() > 1 {
                    diagnostics.push(with_production_trigger(
                        Diagnostic::error(
                            "RSPDL-PROD-004",
                            "semantic.creation_production.relation_producer_conflict",
                            producers[0].span,
                        )
                        .with_argument("production_id", &production.id)
                        .with_argument("output_model_id", &production.output_model_id)
                        .with_argument("relation_id", &slot.relation_id)
                        .with_argument("variant_id", &variant_id)
                        .with_argument("create_branch_ids", joined_ids_csv(&branch_ids))
                        .with_argument(
                            "producer_ids",
                            joined_ids_csv(
                                &producers
                                    .iter()
                                    .map(|producer| producer.id.clone())
                                    .collect::<Vec<_>>(),
                            ),
                        ),
                        production,
                    ));
                }
            }
        }
    }
}

/// Conditions deliberately have a smaller domain than generic predicates.
/// Keeping every invalid shape under PROD-007 avoids accepting a second
/// decision axis by accident and gives frontends one stable contract.
fn resolve_field_producer_condition(
    condition: Option<&UnlinkedFieldProducerCondition>,
    action: &ActionDefinition,
    enums: &[EnumDefinition],
    producer_id: &CanonicalId,
    production: &ConditionalProductionDefinition,
    span: TextRange,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Option<FieldProducerCondition>> {
    let Some(UnlinkedFieldProducerCondition::EnumVariant { input, variant }) = condition else {
        return Some(None);
    };
    let invalid = |diagnostics: &mut Vec<Diagnostic>, input_id: &str, variant_id: &str| {
        diagnostics.push(with_production_trigger(
            Diagnostic::error(
                "RSPDL-PROD-007",
                "semantic.field_producer.condition_not_creation_decision_variant",
                span,
            )
            .with_argument("production_id", &production.id)
            .with_argument("producer_id", producer_id)
            .with_argument("output_model_id", &production.output_model_id)
            .with_argument("decision_input_id", &production.decision_input_id)
            .with_argument("input_id", input_id)
            .with_argument("variant_id", variant_id),
            production,
        ));
    };
    if !validate_reference(input, "RSPDL-PROD-007", diagnostics)
        || !validate_reference(variant, "RSPDL-PROD-007", diagnostics)
    {
        return None;
    }
    let Some(input_definition) = action
        .inputs
        .iter()
        .find(|candidate| member_reference_matches(&candidate.id, &candidate.local_id, input))
    else {
        invalid(diagnostics, input.id(), variant.id());
        return None;
    };
    let ActionInputKind::Value {
        value_type: CanonicalType::Enum(enum_type),
    } = &input_definition.kind
    else {
        invalid(diagnostics, &input_definition.id.to_string(), variant.id());
        return None;
    };
    let variant_definition = enums
        .iter()
        .find(|definition| definition.id == *enum_type.id())
        .and_then(|definition| {
            definition.variants.iter().find(|candidate| {
                member_reference_matches(&candidate.id, &candidate.local_id, variant)
            })
        });
    let Some(variant_definition) = variant_definition else {
        invalid(diagnostics, &input_definition.id.to_string(), variant.id());
        return None;
    };
    if input_definition.id != production.decision_input_id {
        invalid(
            diagnostics,
            &input_definition.id.to_string(),
            &variant_definition.id.to_string(),
        );
        return None;
    }
    Some(Some(FieldProducerCondition::EnumVariant {
        input_id: input_definition.id.clone(),
        variant_id: variant_definition.id.clone(),
    }))
}

#[allow(clippy::too_many_arguments)]
fn resolve_field_producer_source(
    source: &UnlinkedFieldProducerSource,
    producer_id: &CanonicalId,
    action: &ActionDefinition,
    action_id: &CanonicalId,
    output_model_id: &CanonicalId,
    output_field_id: &CanonicalId,
    output_type: &CanonicalType,
    models: &BTreeMap<String, DataModelDefinition>,
    enums: &[EnumDefinition],
    output_model: &DataModelDefinition,
    source_span: TextRange,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<ResolvedFieldProducerSource> {
    match source {
        UnlinkedFieldProducerSource::ActionInput { input } => {
            let input = resolve_field_producer_input(
                action,
                input,
                producer_id,
                action_id,
                output_model_id,
                output_field_id,
                diagnostics,
            )?;
            match &input.kind {
                ActionInputKind::Value { value_type } => Some(ResolvedFieldProducerSource {
                    source: FieldProducerSource::ActionInput {
                        input_id: input.id.clone(),
                    },
                    value_type: value_type.clone(),
                    evidence: input.id.to_string(),
                }),
                ActionInputKind::ExistingModel { .. } => {
                    diagnostics.push(field_producer_type_error(
                        input.span,
                        producer_id,
                        action_id,
                        output_model_id,
                        output_field_id,
                        &input.id.to_string(),
                        output_type,
                    ));
                    None
                }
            }
        }
        UnlinkedFieldProducerSource::InputField { input, field } => {
            let input = resolve_field_producer_input(
                action,
                input,
                producer_id,
                action_id,
                output_model_id,
                output_field_id,
                diagnostics,
            )?;
            let ActionInputKind::ExistingModel { model_id } = &input.kind else {
                diagnostics.push(field_producer_type_error(
                    input.span,
                    producer_id,
                    action_id,
                    output_model_id,
                    output_field_id,
                    &input.id.to_string(),
                    output_type,
                ));
                return None;
            };
            let model = models
                .values()
                .find(|model| model.id == *model_id)
                .expect("existing-model action inputs were linked from these models");
            let field = resolve_field(model, field, diagnostics)?;
            Some(ResolvedFieldProducerSource {
                source: FieldProducerSource::InputField {
                    input_id: input.id.clone(),
                    field_id: field.id.clone(),
                },
                value_type: field.value_type.clone(),
                evidence: format!("{}:{}", input.id, field.id),
            })
        }
        UnlinkedFieldProducerSource::Constant { literal } => {
            let value = resolve_field_producer_constant(
                literal,
                output_type,
                enums,
                producer_id,
                action_id,
                output_model_id,
                output_field_id,
                diagnostics,
            )?;
            Some(ResolvedFieldProducerSource {
                value_type: value.value_type().clone(),
                source: FieldProducerSource::Constant { value },
                evidence: "constant".into(),
            })
        }
        UnlinkedFieldProducerSource::Template { parts } => {
            if *output_type != CanonicalType::String {
                diagnostics.push(field_producer_type_error(
                    source_span,
                    producer_id,
                    action_id,
                    output_model_id,
                    output_field_id,
                    "template",
                    output_type,
                ));
                return None;
            }
            let mut resolved = Vec::new();
            for part in parts {
                match part {
                    UnlinkedTemplatePart::Text { value } => resolved.push(TemplatePart::Text {
                        value: value.clone(),
                    }),
                    UnlinkedTemplatePart::OutputField { field } => {
                        let field = resolve_field(output_model, field, diagnostics)?;
                        if field.value_type != CanonicalType::String {
                            diagnostics.push(
                                Diagnostic::error(
                                    "RSPDL-PROD-002",
                                    "semantic.template.placeholder_not_string",
                                    source_span,
                                )
                                .with_argument("producer_id", producer_id)
                                .with_argument("action_id", action_id)
                                .with_argument("output_model_id", output_model_id)
                                .with_argument("output_field_id", output_field_id)
                                .with_argument("dependency_field_id", &field.id)
                                .with_argument("dependency_type", &field.value_type),
                            );
                            return None;
                        }
                        resolved.push(TemplatePart::OutputField {
                            field_id: field.id.clone(),
                        });
                    }
                }
            }
            Some(ResolvedFieldProducerSource {
                source: FieldProducerSource::Template { parts: resolved },
                value_type: CanonicalType::String,
                evidence: "template".into(),
            })
        }
        UnlinkedFieldProducerSource::EventInput { .. }
        | UnlinkedFieldProducerSource::EventInputField { .. } => {
            diagnostics.push(
                Diagnostic::error(
                    "RSPDL-PROD-001",
                    "semantic.field_producer.source_trigger_owner_mismatch",
                    source_span,
                )
                .with_argument("producer_id", producer_id)
                .with_argument("action_id", action_id)
                .with_argument("output_model_id", output_model_id)
                .with_argument("output_field_id", output_field_id),
            );
            None
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn resolve_event_field_producer_source(
    source: &UnlinkedFieldProducerSource,
    producer_id: &CanonicalId,
    event: &EventDefinition,
    event_id: &CanonicalId,
    output_model_id: &CanonicalId,
    output_field_id: &CanonicalId,
    output_type: &CanonicalType,
    models: &BTreeMap<String, DataModelDefinition>,
    output_model: &DataModelDefinition,
    source_span: TextRange,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<ResolvedFieldProducerSource> {
    let event_diagnostic = |rule_id, message_key, span| {
        with_trigger_arguments(
            Diagnostic::error(rule_id, message_key, span)
                .with_argument("producer_id", producer_id)
                .with_argument("output_model_id", output_model_id)
                .with_argument("output_field_id", output_field_id),
            ProductionTriggerKind::Event,
            event_id,
        )
    };
    let resolve_input = |reference: &SurfaceRef, diagnostics: &mut Vec<Diagnostic>| {
        if !validate_reference(reference, "RSPDL-PROD-001", diagnostics) {
            return None;
        }
        event
            .inputs
            .iter()
            .find(|input| member_reference_matches(&input.id, &input.local_id, reference))
            .or_else(|| {
                diagnostics.push(
                    event_diagnostic(
                        "RSPDL-PROD-001",
                        "semantic.field_producer.source_input_not_found",
                        reference.span(),
                    )
                    .with_argument("source", reference.id()),
                );
                None
            })
    };
    match source {
        UnlinkedFieldProducerSource::EventInput { input } => {
            let input = resolve_input(input, diagnostics)?;
            match &input.kind {
                EventInputKind::Value { value_type } => Some(ResolvedFieldProducerSource {
                    source: FieldProducerSource::EventInput {
                        input_id: input.id.clone(),
                    },
                    value_type: value_type.clone(),
                    evidence: input.id.to_string(),
                }),
                EventInputKind::ExistingModel { .. } => {
                    diagnostics.push(
                        event_diagnostic(
                            "RSPDL-PROD-002",
                            "semantic.field_producer.type_mismatch",
                            input.span,
                        )
                        .with_argument("source", &input.id)
                        .with_argument("output_type", output_type),
                    );
                    None
                }
            }
        }
        UnlinkedFieldProducerSource::EventInputField { input, field } => {
            let input = resolve_input(input, diagnostics)?;
            let EventInputKind::ExistingModel { model_id } = &input.kind else {
                diagnostics.push(
                    event_diagnostic(
                        "RSPDL-PROD-002",
                        "semantic.field_producer.type_mismatch",
                        input.span,
                    )
                    .with_argument("source", &input.id)
                    .with_argument("output_type", output_type),
                );
                return None;
            };
            let model = models
                .values()
                .find(|model| model.id == *model_id)
                .expect("existing-model event inputs were linked from these models");
            if let Err(error) = CanonicalId::new(field.id()) {
                diagnostics.push(with_trigger_arguments(
                    model_error("RSPDL-LINK-003", error, field.span()),
                    ProductionTriggerKind::Event,
                    event_id,
                ));
                return None;
            }
            let Some(field) = model.fields.iter().find(|candidate| {
                member_reference_matches(&candidate.id, &candidate.local_id, field)
            }) else {
                diagnostics.push(
                    event_diagnostic("RSPDL-LINK-003", "semantic.field.not_found", field.span())
                        .with_argument("model_id", &model.id)
                        .with_argument("reference", field.id()),
                );
                return None;
            };
            Some(ResolvedFieldProducerSource {
                source: FieldProducerSource::EventInputField {
                    input_id: input.id.clone(),
                    field_id: field.id.clone(),
                },
                value_type: field.value_type.clone(),
                evidence: format!("{}:{}", input.id, field.id),
            })
        }
        UnlinkedFieldProducerSource::Template { parts } => {
            if *output_type != CanonicalType::String {
                diagnostics.push(
                    event_diagnostic(
                        "RSPDL-PROD-002",
                        "semantic.field_producer.type_mismatch",
                        source_span,
                    )
                    .with_argument("source", "template")
                    .with_argument("output_type", output_type),
                );
                return None;
            }
            let mut resolved = Vec::new();
            for part in parts {
                match part {
                    UnlinkedTemplatePart::Text { value } => {
                        resolved.push(TemplatePart::Text {
                            value: value.clone(),
                        });
                    }
                    UnlinkedTemplatePart::OutputField { field } => {
                        let field = resolve_field(output_model, field, diagnostics)?;
                        if field.value_type != CanonicalType::String {
                            diagnostics.push(
                                event_diagnostic(
                                    "RSPDL-PROD-002",
                                    "semantic.template.placeholder_not_string",
                                    source_span,
                                )
                                .with_argument("dependency_field_id", &field.id)
                                .with_argument("dependency_type", &field.value_type),
                            );
                            return None;
                        }
                        resolved.push(TemplatePart::OutputField {
                            field_id: field.id.clone(),
                        });
                    }
                }
            }
            Some(ResolvedFieldProducerSource {
                source: FieldProducerSource::Template { parts: resolved },
                value_type: CanonicalType::String,
                evidence: "template".into(),
            })
        }
        UnlinkedFieldProducerSource::Constant { .. } => {
            diagnostics.push(event_diagnostic(
                "RSPDL-PROD-007",
                "semantic.event_field_producer.constant_unsupported",
                source_span,
            ));
            None
        }
        UnlinkedFieldProducerSource::ActionInput { .. }
        | UnlinkedFieldProducerSource::InputField { .. } => {
            diagnostics.push(event_diagnostic(
                "RSPDL-PROD-001",
                "semantic.field_producer.source_trigger_owner_mismatch",
                source_span,
            ));
            None
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn resolve_field_producer_input<'a>(
    action: &'a ActionDefinition,
    reference: &SurfaceRef,
    producer_id: &CanonicalId,
    action_id: &CanonicalId,
    output_model_id: &CanonicalId,
    output_field_id: &CanonicalId,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<&'a ActionInputDefinition> {
    if !validate_reference(reference, "RSPDL-PROD-001", diagnostics) {
        return None;
    }
    action
        .inputs
        .iter()
        .find(|input| member_reference_matches(&input.id, &input.local_id, reference))
        .or_else(|| {
            diagnostics.push(
                Diagnostic::error(
                    "RSPDL-PROD-001",
                    "semantic.field_producer.source_input_not_found",
                    reference.span(),
                )
                .with_argument("producer_id", producer_id)
                .with_argument("action_id", action_id)
                .with_argument("output_model_id", output_model_id)
                .with_argument("output_field_id", output_field_id)
                .with_argument("source", reference.id()),
            );
            None
        })
}

#[allow(clippy::too_many_arguments)]
fn resolve_field_producer_constant(
    literal: &UnlinkedLiteral,
    output_type: &CanonicalType,
    enums: &[EnumDefinition],
    producer_id: &CanonicalId,
    action_id: &CanonicalId,
    output_model_id: &CanonicalId,
    output_field_id: &CanonicalId,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<CanonicalValue> {
    let value = match (literal, output_type) {
        (UnlinkedLiteral::String { value, .. }, CanonicalType::String) => {
            Some(CanonicalValue::string(value))
        }
        (UnlinkedLiteral::Integer { value, .. }, CanonicalType::Integer) => {
            CanonicalValue::integer_from_decimal(value).ok()
        }
        (UnlinkedLiteral::Boolean { value, .. }, CanonicalType::Boolean) => {
            Some(CanonicalValue::boolean(*value))
        }
        (UnlinkedLiteral::Named(reference), CanonicalType::Enum(enum_type)) => enums
            .iter()
            .find(|definition| definition.id == *enum_type.id())
            .and_then(|definition| {
                definition.variants.iter().find(|variant| {
                    member_reference_matches(&variant.id, &variant.local_id, reference)
                })
            })
            .and_then(|variant| {
                CanonicalValue::enum_variant(enum_type.clone(), variant.id.clone()).ok()
            }),
        _ => None,
    };
    value.or_else(|| {
        diagnostics.push(field_producer_type_error(
            literal_span(literal),
            producer_id,
            action_id,
            output_model_id,
            output_field_id,
            "constant",
            output_type,
        ));
        None
    })
}

fn field_producer_type_error(
    span: TextRange,
    producer_id: &CanonicalId,
    action_id: &CanonicalId,
    output_model_id: &CanonicalId,
    output_field_id: &CanonicalId,
    source: &str,
    output_type: &CanonicalType,
) -> Diagnostic {
    Diagnostic::error(
        "RSPDL-PROD-002",
        "semantic.field_producer.type_mismatch",
        span,
    )
    .with_argument("producer_id", producer_id)
    .with_argument("action_id", action_id)
    .with_argument("output_model_id", output_model_id)
    .with_argument("output_field_id", output_field_id)
    .with_argument("source", source)
    .with_argument("output_type", output_type)
}

fn resolve_creation_decision_input<'a>(
    action: &'a ActionDefinition,
    reference: &SurfaceRef,
    trigger_kind: ProductionTriggerKind,
    trigger_id: &CanonicalId,
    output_model_id: &CanonicalId,
    span: TextRange,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<(&'a ActionInputDefinition, &'a EnumType)> {
    if !validate_reference(reference, "RSPDL-PROD-002", diagnostics) {
        return None;
    }
    let Some(input) = action
        .inputs
        .iter()
        .find(|input| member_reference_matches(&input.id, &input.local_id, reference))
    else {
        diagnostics.push(with_trigger_arguments(
            Diagnostic::error(
                "RSPDL-PROD-002",
                "semantic.creation_branch.decision_input_not_found",
                reference.span(),
            )
            .with_argument("output_model_id", output_model_id)
            .with_argument("reference", reference.id()),
            trigger_kind,
            trigger_id,
        ));
        return None;
    };
    match &input.kind {
        ActionInputKind::Value {
            value_type: CanonicalType::Enum(enum_type),
        } => Some((input, enum_type)),
        ActionInputKind::ExistingModel { .. } | ActionInputKind::Value { .. } => {
            diagnostics.push(with_trigger_arguments(
                Diagnostic::error(
                    "RSPDL-PROD-002",
                    "semantic.creation_branch.decision_input_requires_enum",
                    span,
                )
                .with_argument("output_model_id", output_model_id)
                .with_argument("input_id", &input.id),
                trigger_kind,
                trigger_id,
            ));
            None
        }
    }
}

fn enum_input_type(input: &ActionInputDefinition) -> Option<&EnumType> {
    match &input.kind {
        ActionInputKind::Value {
            value_type: CanonicalType::Enum(enum_type),
        } => Some(enum_type),
        _ => None,
    }
}

fn event_input_type(input: &EventInputDefinition) -> Option<&EnumType> {
    match &input.kind {
        EventInputKind::Value {
            value_type: CanonicalType::Enum(enum_type),
        } => Some(enum_type),
        _ => None,
    }
}

fn resolve_event_creation_decision_input<'a>(
    event: &'a EventDefinition,
    reference: &SurfaceRef,
    trigger_id: &CanonicalId,
    output_model_id: &CanonicalId,
    span: TextRange,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<(&'a EventInputDefinition, &'a EnumType)> {
    if !validate_reference(reference, "RSPDL-PROD-002", diagnostics) {
        return None;
    }
    let Some(input) = event
        .inputs
        .iter()
        .find(|input| member_reference_matches(&input.id, &input.local_id, reference))
    else {
        diagnostics.push(
            Diagnostic::error(
                "RSPDL-PROD-002",
                "semantic.creation_branch.decision_input_not_found",
                span,
            )
            .with_argument("trigger_kind", "event")
            .with_argument("trigger_id", trigger_id)
            .with_argument("output_model_id", output_model_id)
            .with_argument("reference", reference.id()),
        );
        return None;
    };
    let Some(enum_type) = event_input_type(input) else {
        diagnostics.push(
            Diagnostic::error(
                "RSPDL-PROD-002",
                "semantic.creation_branch.decision_input_not_enum",
                span,
            )
            .with_argument("trigger_kind", "event")
            .with_argument("trigger_id", trigger_id)
            .with_argument("output_model_id", output_model_id)
            .with_argument("input_id", &input.id),
        );
        return None;
    };
    Some((input, enum_type))
}

#[allow(clippy::too_many_arguments)]
fn resolve_creation_variant(
    enums: &[EnumDefinition],
    enum_type: &EnumType,
    reference: &SurfaceRef,
    trigger_kind: ProductionTriggerKind,
    trigger_id: &CanonicalId,
    output_model_id: &CanonicalId,
    input_id: &CanonicalId,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<CanonicalId> {
    if !validate_reference(reference, "RSPDL-PROD-002", diagnostics) {
        return None;
    }
    let variant = enums
        .iter()
        .find(|definition| definition.id == *enum_type.id())
        .and_then(|definition| {
            definition
                .variants
                .iter()
                .find(|variant| member_reference_matches(&variant.id, &variant.local_id, reference))
        });
    match variant {
        Some(variant) => Some(variant.id.clone()),
        None => {
            diagnostics.push(with_trigger_arguments(
                Diagnostic::error(
                    "RSPDL-PROD-002",
                    "semantic.creation_branch.variant_not_in_decision_enum",
                    reference.span(),
                )
                .with_argument("output_model_id", output_model_id)
                .with_argument("input_id", input_id)
                .with_argument("enum_id", enum_type.id())
                .with_argument("reference", reference.id()),
                trigger_kind,
                trigger_id,
            ));
            None
        }
    }
}

struct ResolvedDerivation {
    target_field_id: CanonicalId,
    source_field_id: CanonicalId,
    span: TextRange,
}

#[derive(Default)]
struct DataUsageAnalysis {
    screens: Vec<ScreenDefinition>,
    action_data_mutations: Vec<ActionDataMutationDefinition>,
    action_data_mutation_provenance: Vec<ActionDataMutationProvenance>,
    derivations: Vec<DerivationDefinition>,
    recalculations: Vec<RecalculationDefinition>,
    field_intents: Vec<FieldIntentDefinition>,
}

#[allow(clippy::too_many_arguments)]
fn analyze_data_usage(
    screens: Vec<UnlinkedScreen>,
    action_data_mutations: Vec<UnlinkedActionDataMutation>,
    derivations: Vec<UnlinkedSumDerivation>,
    recalculations: Vec<UnlinkedRecalculation>,
    field_intents: Vec<UnlinkedFieldIntent>,
    module_id: &CanonicalId,
    models: &[DataModelDefinition],
    actions: &BTreeMap<String, CanonicalId>,
    source_id: &SourceId,
    top_level_ids: &mut BTreeSet<CanonicalId>,
    diagnostics: &mut Vec<Diagnostic>,
) -> DataUsageAnalysis {
    if screens.is_empty()
        && action_data_mutations.is_empty()
        && derivations.is_empty()
        && recalculations.is_empty()
        && field_intents.is_empty()
    {
        return DataUsageAnalysis::default();
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
                    span: screen.span,
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
            span: screen.span,
        };
        let definition = screen_map
            .get_mut(&screen_id)
            .expect("screen was inserted above");
        if definition.operations.iter().any(|existing| {
            existing.kind == operation.kind
                && existing.model_id == operation.model_id
                && existing.field_ids == operation.field_ids
        }) {
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

    let mut action_data_mutation_definitions = Vec::new();
    let mut action_data_mutation_provenance = Vec::new();
    let mut mutations_by_action_and_model =
        BTreeMap::<(CanonicalId, CanonicalId), BTreeMap<DataMutationKind, TextRange>>::new();
    for value in action_data_mutations {
        let Some(action_id) = resolve_named_id("action", actions, &value.action, diagnostics)
        else {
            continue;
        };
        let Some(model) = find_model(models, &value.model, diagnostics) else {
            continue;
        };
        let key = (action_id.clone(), model.id.clone());
        let declared = mutations_by_action_and_model.entry(key).or_default();
        if declared.contains_key(&value.mutation) {
            diagnostics.push(
                data_diagnostic(
                    "RSPDL-DATA-004",
                    Severity::Error,
                    "semantic.action_data_mutation.duplicate",
                    value.span,
                )
                .with_argument("action_id", &action_id)
                .with_argument("model_id", &model.id)
                .with_argument("mutation", data_mutation_name(value.mutation)),
            );
            continue;
        }
        if !declared.is_empty() {
            let mutations = declared
                .keys()
                .copied()
                .chain(std::iter::once(value.mutation))
                .map(data_mutation_name)
                .collect::<Vec<_>>()
                .join(",");
            diagnostics.push(
                data_diagnostic(
                    "RSPDL-DATA-004",
                    Severity::Error,
                    "semantic.action_data_mutation.conflict",
                    value.span,
                )
                .with_argument("action_id", &action_id)
                .with_argument("model_id", &model.id)
                .with_argument("mutations", mutations),
            );
        }
        declared.insert(value.mutation, value.span);
        match value.mutation {
            DataMutationKind::Create => {
                model_creators.insert(model.id.clone());
            }
            DataMutationKind::Update | DataMutationKind::Delete => {
                model_uses.push((model.id.clone(), value.span));
            }
        }
        action_data_mutation_definitions.push(ActionDataMutationDefinition {
            action_id: action_id.clone(),
            model_id: model.id.clone(),
            mutation: value.mutation,
            span: value.span,
        });
        action_data_mutation_provenance.push(ActionDataMutationProvenance {
            action_id,
            model_id: model.id.clone(),
            mutation: value.mutation,
            source_id: source_id.clone(),
            span: value.span,
        });
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
    let mut recalculation_definitions = Vec::new();
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
        recalculation_definitions.push(RecalculationDefinition {
            source_field_id: source_field.id.clone(),
            target_field_id: target_field.id.clone(),
            span: recalculation.span,
        });
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
            span: derivation.span,
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
            span: intent.span,
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
        screen.operations.sort_by(|left, right| {
            (&left.kind, &left.model_id, &left.field_ids).cmp(&(
                &right.kind,
                &right.model_id,
                &right.field_ids,
            ))
        });
    }
    derivation_definitions.sort_by(|left, right| left.target_field_id.cmp(&right.target_field_id));
    recalculation_definitions.sort_by(|left, right| {
        (&left.target_field_id, &left.source_field_id)
            .cmp(&(&right.target_field_id, &right.source_field_id))
    });
    action_data_mutation_definitions.sort_by(|left, right| {
        (&left.action_id, &left.model_id, &left.mutation).cmp(&(
            &right.action_id,
            &right.model_id,
            &right.mutation,
        ))
    });
    action_data_mutation_provenance.sort();
    intents.sort_by(|left, right| {
        (&left.field_id, &left.intent).cmp(&(&right.field_id, &right.intent))
    });
    DataUsageAnalysis {
        screens: screen_definitions,
        action_data_mutations: action_data_mutation_definitions,
        action_data_mutation_provenance,
        derivations: derivation_definitions,
        recalculations: recalculation_definitions,
        field_intents: intents,
    }
}

const fn data_mutation_name(mutation: DataMutationKind) -> &'static str {
    match mutation {
        DataMutationKind::Create => "create",
        DataMutationKind::Update => "update",
        DataMutationKind::Delete => "delete",
    }
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
        (UnlinkedLiteral::String { value, .. }, CanonicalType::Decimal) => {
            Some(CanonicalValue::decimal_from_str(value))
        }
        (UnlinkedLiteral::String { value, .. }, CanonicalType::Date) => {
            Some(CanonicalValue::date_from_iso(value))
        }
        (UnlinkedLiteral::String { value, .. }, CanonicalType::Time) => {
            Some(CanonicalValue::time_from_iso(value))
        }
        (UnlinkedLiteral::String { value, .. }, CanonicalType::DateTime) => {
            Some(CanonicalValue::date_time_from_rfc3339(value))
        }
        (UnlinkedLiteral::String { value, .. }, CanonicalType::Duration) => {
            Some(CanonicalValue::duration_from_iso(value))
        }
        (UnlinkedLiteral::String { value, .. }, CanonicalType::Latitude) => {
            Some(CanonicalValue::latitude_from_decimal(value))
        }
        (UnlinkedLiteral::String { value, .. }, CanonicalType::Longitude) => {
            Some(CanonicalValue::longitude_from_decimal(value))
        }
        (UnlinkedLiteral::String { value, .. }, CanonicalType::Money(currency)) => {
            Some(CanonicalValue::money_from_str(value).and_then(|bound| {
                if bound.value_type() == expected {
                    Ok(bound)
                } else {
                    Err(ModelError::TypeMismatch {
                        context: "money literal",
                        expected: CanonicalType::Money(currency.clone()),
                        actual: bound.value_type().clone(),
                    })
                }
            }))
        }
        (UnlinkedLiteral::String { value, .. }, CanonicalType::Percentage) => {
            Some(CanonicalValue::percentage_from_str(value))
        }
        (UnlinkedLiteral::String { value, .. }, CanonicalType::Quantity(_)) => {
            Some(CanonicalValue::quantity_from_str(value).and_then(|bound| {
                if bound.value_type() == expected {
                    Ok(bound)
                } else {
                    Err(ModelError::TypeMismatch {
                        context: "quantity literal",
                        expected: expected.clone(),
                        actual: bound.value_type().clone(),
                    })
                }
            }))
        }
        (UnlinkedLiteral::String { value, .. }, CanonicalType::Coordinate) => {
            Some(CanonicalValue::coordinate_from_str(value))
        }
        (
            UnlinkedLiteral::String { value, .. },
            CanonicalType::Uuid
            | CanonicalType::Email
            | CanonicalType::Url
            | CanonicalType::PhoneNumber
            | CanonicalType::IpAddress
            | CanonicalType::Cidr
            | CanonicalType::CountryCode
            | CanonicalType::LanguageCode
            | CanonicalType::CurrencyCode,
        ) => Some(CanonicalValue::refinement_from_str(expected.clone(), value)),
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

fn resolve_relation<'a>(
    definitions: &'a [RelationDefinition],
    reference: &SurfaceRef,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<&'a RelationDefinition> {
    if !validate_reference(reference, "RSPDL-REL-001", diagnostics) {
        return None;
    }
    let matches = definitions
        .iter()
        .filter(|relation| top_level_reference_matches(&relation.id, reference))
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [relation] => Some(*relation),
        [] => {
            diagnostics.push(
                Diagnostic::error(
                    "RSPDL-REL-001",
                    "semantic.relation.not_found",
                    reference.span(),
                )
                .with_argument("reference", reference.id()),
            );
            None
        }
        _ => {
            diagnostics.push(ambiguous_reference(
                "RSPDL-REL-001",
                "relation",
                reference,
                matches.iter().map(|relation| &relation.id),
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

fn generated_production_id(
    module_id: &CanonicalId,
    trigger_kind: ProductionTriggerKind,
    trigger_id: &CanonicalId,
    output_model_id: &CanonicalId,
) -> Result<CanonicalId, ModelError> {
    CanonicalId::new(format!(
        "{module_id}.{}",
        generated_id(
            "production",
            &match trigger_kind {
                ProductionTriggerKind::Action => format!("{trigger_id}\0{output_model_id}"),
                ProductionTriggerKind::Event => format!("event\0{trigger_id}\0{output_model_id}"),
            }
        )
    ))
}

fn generated_relational_constraint_id(constraint: &RelationalConstraintKind) -> String {
    let identity = match constraint {
        RelationalConstraintKind::NonEmpty { model_id } => format!("nonempty\0{model_id}"),
        RelationalConstraintKind::Required { relation_id } => format!("required\0{relation_id}"),
        RelationalConstraintKind::Unique { relation_id } => format!("unique\0{relation_id}"),
        RelationalConstraintKind::Exclusive { relation_ids } => {
            format!("exclusive\0{}", joined_ids(relation_ids))
        }
        RelationalConstraintKind::Exhaustive { relation_ids } => {
            format!("exhaustive\0{}", joined_ids(relation_ids))
        }
        RelationalConstraintKind::Coexistent { relation_ids } => {
            format!("coexistent\0{}", joined_ids(relation_ids))
        }
    };
    generated_id("relation_rule", &identity)
}

fn joined_ids(ids: &[CanonicalId]) -> String {
    ids.iter()
        .map(CanonicalId::as_str)
        .collect::<Vec<_>>()
        .join("\0")
}

fn joined_ids_set(ids: &BTreeSet<CanonicalId>) -> String {
    joined_ids_csv(&ids.iter().cloned().collect::<Vec<_>>())
}

fn joined_ids_csv(ids: &[CanonicalId]) -> String {
    ids.iter()
        .map(CanonicalId::as_str)
        .collect::<Vec<_>>()
        .join(",")
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
            } else if let Some(value) = value.as_decimal() {
                format!("decimal:{value}")
            } else if let Some(value) = value.as_boolean() {
                format!("boolean:{value}")
            } else if let Some(value) = value.as_date() {
                format!("date:{value}")
            } else if let Some(value) = value.as_time() {
                format!("time:{value}")
            } else if let Some(value) = value.as_date_time() {
                format!("date_time:{value}")
            } else if let Some(value) = value.as_duration() {
                format!("duration:{value}")
            } else if let Some(value) = value.as_latitude() {
                format!("latitude:{value}")
            } else if let Some(value) = value.as_longitude() {
                format!("longitude:{value}")
            } else if let Some(value) = value.as_enum_variant() {
                format!("enum:{value}")
            } else {
                format!("{}:{}", value.value_type(), value.canonical_text())
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
        ModelError::InvalidCurrencyCode { value } => {
            Diagnostic::error(rule_id, "model.invalid_currency_code", span)
                .with_argument("value", value)
        }
        ModelError::UnsupportedMapKeyType { value_type } => {
            Diagnostic::error(rule_id, "model.unsupported_map_key_type", span)
                .with_argument("value_type", value_type)
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
        ModelError::InvalidDecimal { value } => {
            Diagnostic::error(rule_id, "model.invalid_decimal", span).with_argument("value", value)
        }
        ModelError::InvalidDate { value } => {
            Diagnostic::error(rule_id, "model.invalid_date", span).with_argument("value", value)
        }
        ModelError::InvalidTime { value } => {
            Diagnostic::error(rule_id, "model.invalid_time", span).with_argument("value", value)
        }
        ModelError::InvalidDateTime { value } => {
            Diagnostic::error(rule_id, "model.invalid_date_time", span)
                .with_argument("value", value)
        }
        ModelError::InvalidDuration { value } => {
            Diagnostic::error(rule_id, "model.invalid_duration", span).with_argument("value", value)
        }
        ModelError::InvalidLatitude { value } => {
            Diagnostic::error(rule_id, "model.invalid_latitude", span).with_argument("value", value)
        }
        ModelError::InvalidLongitude { value } => {
            Diagnostic::error(rule_id, "model.invalid_longitude", span)
                .with_argument("value", value)
        }
        ModelError::InvalidMoney { value }
        | ModelError::InvalidPercentage { value }
        | ModelError::InvalidQuantity { value }
        | ModelError::InvalidCoordinate { value } => {
            Diagnostic::error(rule_id, "model.invalid_extended_scalar", span)
                .with_argument("value", value)
        }
        ModelError::InvalidRefinementText { value_type, value } => {
            Diagnostic::error(rule_id, "model.invalid_refinement_text", span)
                .with_argument("value_type", value_type)
                .with_argument("value", value)
        }
        ModelError::InvalidLocalDateTime { value }
        | ModelError::InvalidZonedDateTime { value }
        | ModelError::InvalidCalendarDuration { value } => {
            Diagnostic::error(rule_id, "model.invalid_temporal_value", span)
                .with_argument("value", value)
        }
        ModelError::CalendarDateOverflow => {
            Diagnostic::error(rule_id, "model.calendar_date_overflow", span)
        }
        ModelError::DuplicateSetElement => {
            Diagnostic::error(rule_id, "model.duplicate_set_element", span)
        }
        ModelError::DuplicateMapKey => Diagnostic::error(rule_id, "model.duplicate_map_key", span),
        ModelError::InvalidReferenceRecordId => {
            Diagnostic::error(rule_id, "model.invalid_reference_record_id", span)
        }
        ModelError::InvalidRadius => Diagnostic::error(rule_id, "model.invalid_radius", span),
        ModelError::UnsupportedOperation {
            operation,
            value_type,
        } => Diagnostic::error(rule_id, "model.unsupported_operation", span)
            .with_argument("operation", operation)
            .with_argument("value_type", value_type),
    }
}
