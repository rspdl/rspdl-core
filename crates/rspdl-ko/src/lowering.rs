use std::collections::{BTreeMap, BTreeSet};

use rspdl_domain::{
    CreationDecision, DataMutationKind, Diagnostic, FieldIntentKind, Frontend, FrontendOutput,
    PolicyEffect, ProductionTriggerKind, RelationOperator, ScreenOperationKind, SurfaceRef,
    UnlinkedAction, UnlinkedActionDataMutation, UnlinkedActionInput, UnlinkedActionInputKind,
    UnlinkedConstraint, UnlinkedCreationBranch, UnlinkedDataModel, UnlinkedDeclaration,
    UnlinkedEnum, UnlinkedEnumVariant, UnlinkedEvent, UnlinkedEventInput, UnlinkedEventInputKind,
    UnlinkedField, UnlinkedFieldIntent, UnlinkedFieldProducer, UnlinkedFieldProducerCondition,
    UnlinkedFieldProducerSource, UnlinkedLiteral, UnlinkedModule, UnlinkedOperand, UnlinkedPolicy,
    UnlinkedProductionTrigger, UnlinkedRecalculation, UnlinkedRelation, UnlinkedRelationProducer,
    UnlinkedRelationalConstraint, UnlinkedRelationalConstraintKind, UnlinkedRole, UnlinkedScreen,
    UnlinkedSumDerivation, UnlinkedTemplatePart, UnlinkedTypeReference,
};
use rspdl_domain::{
    ScreenLayoutKind, UnlinkedActionOutcome, UnlinkedActionOutcomes, UnlinkedCategory,
    UnlinkedHandlerKind, UnlinkedLayoutElement, UnlinkedLookupResult, UnlinkedOutcomeData,
    UnlinkedOutcomeDataSource, UnlinkedOutcomeKind, UnlinkedRecovery, UnlinkedRecoveryKind,
    UnlinkedSameScreenHandler, UnlinkedScreenLayout, UnlinkedScreenPath, UnlinkedScreenPermission,
    UnlinkedWorkflow, UnlinkedWorkflowAcquisition, UnlinkedWorkflowCompletion,
    UnlinkedWorkflowData,
};

use crate::ast::*;
use crate::{Span, parse};

pub type LowerOutput = FrontendOutput;

#[derive(Clone, Debug)]
struct Symbol {
    name: String,
    id: String,
}

impl From<&NamedIdAst> for Symbol {
    fn from(value: &NamedIdAst) -> Self {
        Self {
            name: value.name.clone(),
            id: value.id.clone(),
        }
    }
}

#[derive(Clone, Debug)]
struct EnumSymbols {
    symbol: Symbol,
    variants: Vec<Symbol>,
}

#[derive(Clone, Debug)]
struct FieldSymbol {
    symbol: Symbol,
    value_type: TypeReferenceAst,
}

#[derive(Clone, Debug)]
struct ModelSymbols {
    symbol: Symbol,
    fields: Vec<FieldSymbol>,
}

#[derive(Clone, Debug)]
struct ActionInputSymbol {
    action_id: String,
    symbol: Symbol,
    enum_type_name: Option<String>,
    existing_model_name: Option<String>,
}

#[derive(Clone, Debug, Default)]
struct StableIdIndex {
    enums: Vec<EnumSymbols>,
    models: Vec<ModelSymbols>,
    relations: Vec<Symbol>,
    roles: Vec<Symbol>,
    actions: Vec<Symbol>,
    events: Vec<Symbol>,
    action_inputs: Vec<ActionInputSymbol>,
    event_inputs: Vec<ActionInputSymbol>,
    /// Screens are declared across several sentences, so the same screen appears
    /// more than once here. Resolution dedupes by ID, so that is harmless.
    screens: Vec<Symbol>,
}

impl StableIdIndex {
    fn new(document: &DocumentAst) -> Self {
        let mut index = Self::default();
        for declaration in &document.declarations {
            match declaration {
                DeclarationAst::Enum(value) => index.enums.push(EnumSymbols {
                    symbol: Symbol::from(&value.declaration),
                    variants: value
                        .values
                        .iter()
                        .map(|variant| Symbol::from(&variant.declaration))
                        .collect(),
                }),
                DeclarationAst::DataModel(value) => index.models.push(ModelSymbols {
                    symbol: Symbol::from(&value.declaration),
                    fields: value
                        .fields
                        .iter()
                        .map(|field| FieldSymbol {
                            symbol: Symbol::from(&field.declaration),
                            value_type: field.value_type.clone(),
                        })
                        .collect(),
                }),
                DeclarationAst::Relation(value) => {
                    index.relations.push(Symbol::from(&value.declaration));
                }
                DeclarationAst::Role(value) => {
                    index.roles.push(Symbol::from(&value.declaration));
                }
                DeclarationAst::Action(value) => {
                    index.actions.push(Symbol::from(&value.declaration));
                }
                DeclarationAst::Event(value) => {
                    index.events.push(Symbol::from(&value.declaration));
                }
                DeclarationAst::Screen(value) => {
                    index.screens.push(Symbol::from(&value.declaration));
                }
                _ => {}
            }
        }
        for declaration in &document.declarations {
            let DeclarationAst::EventInput(value) = declaration else {
                continue;
            };
            let Some(event_id) = unique_symbol_id(index.events.iter(), &value.event) else {
                continue;
            };
            let enum_type_name = match &value.kind {
                ActionInputKindAst::Value {
                    value_type: TypeReferenceAst::Named(name),
                } => Some(name.clone()),
                _ => None,
            };
            let existing_model_name = match &value.kind {
                ActionInputKindAst::ExistingModel { model } => Some(model.clone()),
                _ => None,
            };
            index.event_inputs.push(ActionInputSymbol {
                action_id: event_id.to_owned(),
                symbol: Symbol::from(&value.declaration),
                enum_type_name,
                existing_model_name,
            });
        }
        for declaration in &document.declarations {
            let DeclarationAst::ActionInput(value) = declaration else {
                continue;
            };
            let Some(action_id) = unique_symbol_id(index.actions.iter(), &value.action) else {
                continue;
            };
            let enum_type_name = match &value.kind {
                ActionInputKindAst::Value {
                    value_type: TypeReferenceAst::Named(name),
                } => Some(name.clone()),
                ActionInputKindAst::ExistingModel { .. } | ActionInputKindAst::Value { .. } => None,
            };
            let existing_model_name = match &value.kind {
                ActionInputKindAst::ExistingModel { model } => Some(model.clone()),
                ActionInputKindAst::Value { .. } => None,
            };
            index.action_inputs.push(ActionInputSymbol {
                action_id: action_id.to_owned(),
                symbol: Symbol::from(&value.declaration),
                enum_type_name,
                existing_model_name,
            });
        }
        index
    }

    fn enum_reference(
        &self,
        value: &str,
        span: Span,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<SurfaceRef> {
        resolve_symbols(
            self.enums.iter().map(|value| &value.symbol),
            value,
            "enum",
            span,
            diagnostics,
        )
    }

    fn model_reference(
        &self,
        value: &str,
        span: Span,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<SurfaceRef> {
        resolve_symbols(
            self.models.iter().map(|value| &value.symbol),
            value,
            "model",
            span,
            diagnostics,
        )
    }

    fn role_reference(
        &self,
        value: &str,
        span: Span,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<SurfaceRef> {
        resolve_symbols(self.roles.iter(), value, "role", span, diagnostics)
    }

    fn relation_reference(
        &self,
        value: &str,
        span: Span,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<SurfaceRef> {
        resolve_symbols(self.relations.iter(), value, "relation", span, diagnostics)
    }

    fn action_reference(
        &self,
        value: &str,
        span: Span,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<SurfaceRef> {
        resolve_symbols(self.actions.iter(), value, "action", span, diagnostics)
    }

    fn event_reference(
        &self,
        value: &str,
        span: Span,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<SurfaceRef> {
        resolve_symbols(self.events.iter(), value, "event", span, diagnostics)
    }

    fn trigger_reference(
        &self,
        value: &str,
        span: Span,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<(ProductionTriggerKind, SurfaceRef)> {
        let action = matching_symbol_ids(self.actions.iter(), value);
        let event = matching_symbol_ids(self.events.iter(), value);
        match (action.as_slice(), event.as_slice()) {
            ([id], []) => Some((
                ProductionTriggerKind::Action,
                SurfaceRef::stable_id(*id, span),
            )),
            ([], [id]) => Some((
                ProductionTriggerKind::Event,
                SurfaceRef::stable_id(*id, span),
            )),
            ([], []) => {
                diagnostics.push(
                    Diagnostic::error("RSPDL-KO-REF-001", "ko.reference.not_found", span)
                        .with_argument("kind", "trigger")
                        .with_argument("reference", value),
                );
                None
            }
            _ => {
                diagnostics.push(
                    Diagnostic::error("RSPDL-KO-REF-002", "ko.reference.ambiguous", span)
                        .with_argument("kind", "trigger")
                        .with_argument("reference", value),
                );
                None
            }
        }
    }

    fn action_input_reference(
        &self,
        action: Option<&SurfaceRef>,
        value: &str,
        span: Span,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<SurfaceRef> {
        let action_id = action?.id();
        resolve_symbols(
            self.action_inputs
                .iter()
                .filter(|input| input.action_id == action_id)
                .map(|input| &input.symbol),
            value,
            "action_input",
            span,
            diagnostics,
        )
    }

    fn event_input_reference(
        &self,
        event: Option<&SurfaceRef>,
        value: &str,
        span: Span,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<SurfaceRef> {
        let event_id = event?.id();
        resolve_symbols(
            self.event_inputs
                .iter()
                .filter(|input| input.action_id == event_id)
                .map(|input| &input.symbol),
            value,
            "event_input",
            span,
            diagnostics,
        )
    }

    fn event_input_enum_reference(
        &self,
        event: Option<&SurfaceRef>,
        input: Option<&SurfaceRef>,
        span: Span,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<SurfaceRef> {
        let event_id = event?.id();
        let input_id = input?.id();
        let name = self
            .event_inputs
            .iter()
            .find(|candidate| candidate.action_id == event_id && candidate.symbol.id == input_id)?
            .enum_type_name
            .as_deref()?;
        self.enum_reference(name, span, diagnostics)
    }

    /// A scalar or existing-model input intentionally has no enum here. Its
    /// variant remains a raw surface reference so the common analyzer owns the
    /// required `RSPDL-PROD-002` decision-input diagnostic.
    fn action_input_enum_reference(
        &self,
        action: Option<&SurfaceRef>,
        input: Option<&SurfaceRef>,
        span: Span,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<SurfaceRef> {
        let action_id = action?.id();
        let input_id = input?.id();
        let enum_type_name = self
            .action_inputs
            .iter()
            .find(|candidate| candidate.action_id == action_id && candidate.symbol.id == input_id)?
            .enum_type_name
            .as_deref()?;
        self.enum_reference(enum_type_name, span, diagnostics)
    }

    /// Conditional producer diagnostics belong to the common analyzer. When
    /// the input is not an enum (or the variant belongs to another enum), keep
    /// a stable-ID candidate rather than turning a Korean display token into a
    /// frontend-only canonical-ID failure.
    fn enum_variant_reference_any(
        &self,
        value: &str,
        span: Span,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<SurfaceRef> {
        resolve_symbols(
            self.enums
                .iter()
                .flat_map(|enum_symbols| enum_symbols.variants.iter()),
            value,
            "enum_variant",
            span,
            diagnostics,
        )
    }

    fn action_input_model_reference(
        &self,
        action: Option<&SurfaceRef>,
        input: Option<&SurfaceRef>,
        span: Span,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<SurfaceRef> {
        let candidate = self.action_inputs.iter().find(|candidate| {
            Some(candidate.action_id.as_str()) == action.map(SurfaceRef::id)
                && Some(candidate.symbol.id.as_str()) == input.map(SurfaceRef::id)
        })?;
        self.model_reference(candidate.existing_model_name.as_deref()?, span, diagnostics)
    }

    fn event_input_model_reference(
        &self,
        event: Option<&SurfaceRef>,
        input: Option<&SurfaceRef>,
        span: Span,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<SurfaceRef> {
        let candidate = self.event_inputs.iter().find(|candidate| {
            Some(candidate.action_id.as_str()) == event.map(SurfaceRef::id)
                && Some(candidate.symbol.id.as_str()) == input.map(SurfaceRef::id)
        })?;
        self.model_reference(candidate.existing_model_name.as_deref()?, span, diagnostics)
    }

    /// Resolves a screen named the way a sentence names one.
    fn screen_reference(
        &self,
        value: &str,
        span: Span,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<SurfaceRef> {
        resolve_symbols(self.screens.iter(), value, "screen", span, diagnostics)
    }

    /// Resolves a field that is named without saying which model owns it.
    ///
    /// The frontmatter writes `- 입력: 수량` because the screen already says which
    /// model it touches, but that is a cross-declaration fact and a frontend may
    /// not reason about it. So the name is matched against every declared field
    /// and an ambiguity is reported as one, which is the honest answer.
    fn unscoped_field_reference(
        &self,
        value: &str,
        span: Span,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<SurfaceRef> {
        resolve_symbols(
            self.models
                .iter()
                .flat_map(|model| model.fields.iter().map(|field| &field.symbol)),
            value,
            "field",
            span,
            diagnostics,
        )
    }

    fn field_reference(
        &self,
        model: Option<&SurfaceRef>,
        value: &str,
        span: Span,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<SurfaceRef> {
        let model_id = model?.id();
        resolve_symbols(
            self.models
                .iter()
                .filter(|model| model.symbol.id == model_id)
                .flat_map(|model| model.fields.iter().map(|field| &field.symbol)),
            value,
            "field",
            span,
            diagnostics,
        )
    }

    fn enum_variant_reference(
        &self,
        enum_id: Option<&str>,
        value: &str,
        span: Span,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<SurfaceRef> {
        let enum_id = enum_id?;
        resolve_symbols(
            self.enums
                .iter()
                .filter(|definition| definition.symbol.id == enum_id)
                .flat_map(|definition| definition.variants.iter()),
            value,
            "enum_variant",
            span,
            diagnostics,
        )
    }

    fn field_enum_id(
        &self,
        model: Option<&SurfaceRef>,
        field: Option<&SurfaceRef>,
    ) -> Option<&str> {
        let model_id = model?.id();
        let field_id = field?.id();
        let enum_name = self
            .models
            .iter()
            .find(|model| model.symbol.id == model_id)?
            .fields
            .iter()
            .find(|field| field.symbol.id == field_id)?
            .value_type
            .clone();
        let TypeReferenceAst::Named(enum_name) = enum_name else {
            return None;
        };
        unique_symbol_id(
            self.enums.iter().map(|definition| &definition.symbol),
            &enum_name,
        )
    }
}

fn resolve_symbols<'a>(
    symbols: impl IntoIterator<Item = &'a Symbol>,
    value: &str,
    kind: &str,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<SurfaceRef> {
    let ids = matching_symbol_ids(symbols, value);
    match ids.len() {
        1 => Some(SurfaceRef::stable_id(
            ids.into_iter().next().expect("one ID must exist"),
            span,
        )),
        0 => {
            diagnostics.push(
                Diagnostic::error("RSPDL-KO-REF-001", "ko.reference.not_found", span)
                    .with_argument("kind", kind)
                    .with_argument("reference", value),
            );
            None
        }
        _ => {
            diagnostics.push(
                Diagnostic::error("RSPDL-KO-REF-002", "ko.reference.ambiguous", span)
                    .with_argument("kind", kind)
                    .with_argument("reference", value),
            );
            None
        }
    }
}

fn unique_symbol_id<'a>(
    symbols: impl IntoIterator<Item = &'a Symbol>,
    value: &str,
) -> Option<&'a str> {
    let ids = matching_symbol_ids(symbols, value);
    (ids.len() == 1).then(|| *ids.first().expect("one ID must exist"))
}

fn matching_symbol_ids<'a>(
    symbols: impl IntoIterator<Item = &'a Symbol>,
    value: &str,
) -> Vec<&'a str> {
    symbols
        .into_iter()
        .filter(|symbol| symbol.name == value || symbol.id == value)
        .map(|symbol| symbol.id.as_str())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn required_reference(reference: Option<SurfaceRef>, span: Span) -> SurfaceRef {
    reference.unwrap_or_else(|| SurfaceRef::stable_id("_invalid", span))
}

/// The controlled-Korean frontend implementation.
#[derive(Clone, Copy, Debug, Default)]
pub struct KoreanFrontend;

impl Frontend for KoreanFrontend {
    fn language_id(&self) -> &'static str {
        "ko-KR"
    }

    fn lower_source(&self, source: &str) -> FrontendOutput {
        let parsed = parse(source);
        let mut diagnostics = parsed.diagnostics;
        let module = if diagnostics.iter().any(|diagnostic| diagnostic.is_error()) {
            None
        } else {
            parsed.document.as_ref().and_then(|document| {
                let lowered = lower(document);
                diagnostics.extend(lowered.diagnostics);
                lowered.module
            })
        };
        FrontendOutput {
            module,
            diagnostics,
        }
    }
}

/// Desugars a Korean AST into locale-neutral, unresolved semantic intent.
///
/// Locale display names are mapped to declaration stable IDs here. Validation,
/// type checking, and semantic analysis still belong to the shared analyzer.
pub fn lower(document: &DocumentAst) -> LowerOutput {
    let index = StableIdIndex::new(document);
    let mut diagnostics = Vec::new();
    let mut action_inputs = lower_action_inputs(document, &index, &mut diagnostics);
    let mut event_inputs = lower_event_inputs(document, &index, &mut diagnostics);
    let mut module = UnlinkedModule {
        declaration: declaration(&document.module.declaration, true),
        span: document.module.span,
        enums: Vec::new(),
        models: Vec::new(),
        relations: Vec::new(),
        relational_constraints: Vec::new(),
        screens: Vec::new(),
        action_data_mutations: Vec::new(),
        derivations: Vec::new(),
        recalculations: Vec::new(),
        field_intents: Vec::new(),
        constraints: Vec::new(),
        roles: Vec::new(),
        actions: Vec::new(),
        events: Vec::new(),
        creation_branches: Vec::new(),
        field_producers: Vec::new(),
        relation_producers: Vec::new(),
        policies: Vec::new(),
        information_architecture: Vec::new(),
        screen_layouts: Vec::new(),
        screen_paths: Vec::new(),
        action_outcomes: Vec::new(),
        lookup_results: Vec::new(),
        workflows: Vec::new(),
    };

    if let Some(frontmatter) = &document.frontmatter {
        for category in &frontmatter.information_architecture {
            flatten_category(
                category,
                None,
                &index,
                &mut module.information_architecture,
                &mut diagnostics,
            );
        }
        module.screen_layouts = frontmatter
            .screens
            .iter()
            .filter_map(|value| screen_layout(value, &index, &mut diagnostics))
            .collect();
        module.screen_paths = frontmatter
            .paths
            .iter()
            .filter_map(|value| screen_path(value, &index, &mut diagnostics))
            .collect();
        module.action_outcomes = frontmatter
            .action_outcomes
            .iter()
            .filter_map(|value| action_outcomes(value, &index, &mut diagnostics))
            .collect();
        module.lookup_results = frontmatter
            .lookup_results
            .iter()
            .filter_map(|value| lookup_result(value, &index, &mut diagnostics))
            .collect();
        module.workflows = frontmatter
            .workflows
            .iter()
            .filter_map(|value| workflow(value, &index, &mut diagnostics))
            .collect();
    }

    for value in &document.declarations {
        match value {
            DeclarationAst::Enum(value) => module.enums.push(UnlinkedEnum {
                declaration: declaration(&value.declaration, true),
                variants: value
                    .values
                    .iter()
                    .map(|variant| UnlinkedEnumVariant {
                        declaration: declaration(&variant.declaration, true),
                        span: variant.span,
                    })
                    .collect(),
                span: value.span,
            }),
            DeclarationAst::DataModel(value) => module.models.push(UnlinkedDataModel {
                declaration: declaration(&value.declaration, true),
                fields: value
                    .fields
                    .iter()
                    .map(|field| UnlinkedField {
                        declaration: declaration(&field.declaration, true),
                        required: field.required,
                        value_type: type_reference(
                            &field.value_type,
                            field.declaration.span,
                            &index,
                            &mut diagnostics,
                        ),
                        span: field.span,
                    })
                    .collect(),
                span: value.span,
            }),
            DeclarationAst::Relation(value) => {
                let parameter_models = value
                    .parameter_models
                    .iter()
                    .map(|model| {
                        required_reference(
                            index.model_reference(model, value.span, &mut diagnostics),
                            value.span,
                        )
                    })
                    .collect();
                module.relations.push(UnlinkedRelation {
                    declaration: declaration(&value.declaration, true),
                    parameter_models,
                    span: value.span,
                });
            }
            DeclarationAst::RelationalConstraint(value) => {
                let constraint = match &value.constraint {
                    RelationalConstraintKindAst::NonEmpty { model } => {
                        UnlinkedRelationalConstraintKind::NonEmpty {
                            model: required_reference(
                                index.model_reference(model, value.span, &mut diagnostics),
                                value.span,
                            ),
                        }
                    }
                    RelationalConstraintKindAst::Required { model, relation } => {
                        UnlinkedRelationalConstraintKind::Required {
                            model: required_reference(
                                index.model_reference(model, value.span, &mut diagnostics),
                                value.span,
                            ),
                            relation: required_reference(
                                index.relation_reference(relation, value.span, &mut diagnostics),
                                value.span,
                            ),
                        }
                    }
                    RelationalConstraintKindAst::Unique { model, relation } => {
                        UnlinkedRelationalConstraintKind::Unique {
                            model: required_reference(
                                index.model_reference(model, value.span, &mut diagnostics),
                                value.span,
                            ),
                            relation: required_reference(
                                index.relation_reference(relation, value.span, &mut diagnostics),
                                value.span,
                            ),
                        }
                    }
                    RelationalConstraintKindAst::Exclusive { relations } => {
                        UnlinkedRelationalConstraintKind::Exclusive {
                            relations: relation_references(
                                relations,
                                value.span,
                                &index,
                                &mut diagnostics,
                            ),
                        }
                    }
                    RelationalConstraintKindAst::Exhaustive { relations } => {
                        UnlinkedRelationalConstraintKind::Exhaustive {
                            relations: relation_references(
                                relations,
                                value.span,
                                &index,
                                &mut diagnostics,
                            ),
                        }
                    }
                    RelationalConstraintKindAst::Coexistent { relations } => {
                        UnlinkedRelationalConstraintKind::Coexistent {
                            relations: relation_references(
                                relations,
                                value.span,
                                &index,
                                &mut diagnostics,
                            ),
                        }
                    }
                };
                module
                    .relational_constraints
                    .push(UnlinkedRelationalConstraint {
                        declaration: UnlinkedDeclaration {
                            name: String::new(),
                            id: None,
                            span: value.span,
                        },
                        constraint,
                        span: value.span,
                    });
            }
            DeclarationAst::Screen(value) => {
                let model = index.model_reference(&value.model, value.span, &mut diagnostics);
                let fields = value
                    .fields
                    .iter()
                    .map(|field| {
                        required_reference(
                            index.field_reference(
                                model.as_ref(),
                                field,
                                value.span,
                                &mut diagnostics,
                            ),
                            value.span,
                        )
                    })
                    .collect();
                module.screens.push(UnlinkedScreen {
                    declaration: declaration(&value.declaration, true),
                    model: required_reference(model, value.span),
                    fields,
                    operation: screen_operation(value.operation),
                    span: value.span,
                });
            }
            DeclarationAst::ActionDataMutation(value) => {
                let action = index.action_reference(&value.action, value.span, &mut diagnostics);
                let model = index.model_reference(&value.model, value.span, &mut diagnostics);
                module
                    .action_data_mutations
                    .push(UnlinkedActionDataMutation {
                        action: required_reference(action, value.span),
                        model: required_reference(model, value.span),
                        mutation: match value.mutation {
                            DataMutationKindAst::Create => DataMutationKind::Create,
                            DataMutationKindAst::Update => DataMutationKind::Update,
                            DataMutationKindAst::Delete => DataMutationKind::Delete,
                        },
                        span: value.span,
                    });
            }
            DeclarationAst::SumDerivation(value) => {
                let target_model =
                    index.model_reference(&value.target_model, value.span, &mut diagnostics);
                let target_field = index.field_reference(
                    target_model.as_ref(),
                    &value.target_field,
                    value.span,
                    &mut diagnostics,
                );
                let source_model =
                    index.model_reference(&value.source_model, value.span, &mut diagnostics);
                let source_field = index.field_reference(
                    source_model.as_ref(),
                    &value.source_field,
                    value.span,
                    &mut diagnostics,
                );
                module.derivations.push(UnlinkedSumDerivation {
                    target_model: required_reference(target_model, value.span),
                    target_field: required_reference(target_field, value.span),
                    source_model: required_reference(source_model, value.span),
                    source_field: required_reference(source_field, value.span),
                    span: value.span,
                });
            }
            DeclarationAst::Recalculation(value) => {
                let source_model =
                    index.model_reference(&value.source_model, value.span, &mut diagnostics);
                let source_field = index.field_reference(
                    source_model.as_ref(),
                    &value.source_field,
                    value.span,
                    &mut diagnostics,
                );
                let target_model =
                    index.model_reference(&value.target_model, value.span, &mut diagnostics);
                let target_field = index.field_reference(
                    target_model.as_ref(),
                    &value.target_field,
                    value.span,
                    &mut diagnostics,
                );
                module.recalculations.push(UnlinkedRecalculation {
                    source_model: required_reference(source_model, value.span),
                    source_field: required_reference(source_field, value.span),
                    target_model: required_reference(target_model, value.span),
                    target_field: required_reference(target_field, value.span),
                    span: value.span,
                });
            }
            DeclarationAst::FieldIntent(value) => {
                let model = index.model_reference(&value.model, value.span, &mut diagnostics);
                let field = index.field_reference(
                    model.as_ref(),
                    &value.field,
                    value.span,
                    &mut diagnostics,
                );
                module.field_intents.push(UnlinkedFieldIntent {
                    model: required_reference(model, value.span),
                    field: required_reference(field, value.span),
                    intent: match value.intent {
                        FieldIntentKindAst::Internal => FieldIntentKind::Internal,
                        FieldIntentKindAst::Hidden => FieldIntentKind::Hidden,
                    },
                    span: value.span,
                });
            }
            DeclarationAst::Constraint(value) => {
                let model = index.model_reference(
                    &value.expression.model,
                    value.expression.span,
                    &mut diagnostics,
                );
                let left_field = operand_field_reference(
                    &value.expression.left,
                    model.as_ref(),
                    value.expression.span,
                    &index,
                    &mut diagnostics,
                );
                let right_field = operand_field_reference(
                    &value.expression.right,
                    model.as_ref(),
                    value.expression.span,
                    &index,
                    &mut diagnostics,
                );
                // A literal takes its expected enum type from the field operand
                // on the opposite side of the comparison.
                let left_expected_enum = index.field_enum_id(model.as_ref(), right_field.as_ref());
                let right_expected_enum = index.field_enum_id(model.as_ref(), left_field.as_ref());
                module.constraints.push(UnlinkedConstraint {
                    // Anonymous semantic IDs are generated by the shared linker
                    // from the stable IDs supplied by this frontend.
                    declaration: declaration(&value.declaration, false),
                    model: required_reference(model, value.expression.span),
                    left: operand(
                        &value.expression.left,
                        left_field,
                        left_expected_enum,
                        value.expression.span,
                        &index,
                        &mut diagnostics,
                    ),
                    operator: relation(value.expression.operator),
                    right: operand(
                        &value.expression.right,
                        right_field,
                        right_expected_enum,
                        value.expression.span,
                        &index,
                        &mut diagnostics,
                    ),
                    span: value.expression.span,
                });
            }
            DeclarationAst::Role(value) => module.roles.push(UnlinkedRole {
                declaration: declaration(&value.declaration, true),
                span: value.span,
            }),
            DeclarationAst::Action(value) => module.actions.push(UnlinkedAction {
                declaration: declaration(&value.declaration, true),
                inputs: action_inputs
                    .remove(&value.declaration.id)
                    .unwrap_or_default(),
                span: value.span,
            }),
            DeclarationAst::Event(value) => module.events.push(UnlinkedEvent {
                declaration: declaration(&value.declaration, true),
                inputs: event_inputs
                    .remove(&value.declaration.id)
                    .unwrap_or_default(),
                span: value.span,
            }),
            DeclarationAst::ActionInput(_) => {}
            DeclarationAst::EventInput(_) => {}
            DeclarationAst::CreationBranch(value) => {
                let Some((trigger_kind, trigger_ref)) =
                    index.trigger_reference(&value.action, value.span, &mut diagnostics)
                else {
                    continue;
                };
                let action =
                    (trigger_kind == ProductionTriggerKind::Action).then_some(trigger_ref.clone());
                let input = if trigger_kind == ProductionTriggerKind::Action {
                    index.action_input_reference(
                        action.as_ref(),
                        &value.input,
                        value.span,
                        &mut diagnostics,
                    )
                } else {
                    index.event_input_reference(
                        Some(&trigger_ref),
                        &value.input,
                        value.span,
                        &mut diagnostics,
                    )
                };
                let enum_type = if trigger_kind == ProductionTriggerKind::Action {
                    index.action_input_enum_reference(
                        action.as_ref(),
                        input.as_ref(),
                        value.span,
                        &mut diagnostics,
                    )
                } else {
                    index.event_input_enum_reference(
                        Some(&trigger_ref),
                        input.as_ref(),
                        value.span,
                        &mut diagnostics,
                    )
                };
                let Some(input) = input else {
                    continue;
                };
                let variant = if let Some(enum_type) = enum_type.as_ref() {
                    let Some(variant) = index.enum_variant_reference(
                        Some(enum_type.id()),
                        &value.variant,
                        value.span,
                        &mut diagnostics,
                    ) else {
                        continue;
                    };
                    variant
                } else {
                    SurfaceRef::stable_id(&value.variant, value.span)
                };
                let Some(output_model) =
                    index.model_reference(&value.output_model, value.span, &mut diagnostics)
                else {
                    continue;
                };
                module.creation_branches.push(UnlinkedCreationBranch {
                    declaration: declaration(&value.declaration, true),
                    action: action.clone(),
                    trigger: UnlinkedProductionTrigger {
                        kind: trigger_kind,
                        reference: trigger_ref,
                    },
                    input,
                    variant,
                    output_model,
                    decision: match value.decision {
                        CreationDecisionAst::Create => CreationDecision::Create,
                        CreationDecisionAst::Skip => CreationDecision::Skip,
                    },
                    span: value.span,
                });
            }
            DeclarationAst::FieldProducer(value) => {
                let trigger_kind = match value.trigger.kind {
                    ProducerTriggerKindAst::Action => ProductionTriggerKind::Action,
                    ProducerTriggerKindAst::Event => ProductionTriggerKind::Event,
                };
                let trigger = match trigger_kind {
                    ProductionTriggerKind::Action => {
                        index.action_reference(&value.trigger.name, value.span, &mut diagnostics)
                    }
                    ProductionTriggerKind::Event => {
                        index.event_reference(&value.trigger.name, value.span, &mut diagnostics)
                    }
                };
                let action = (trigger_kind == ProductionTriggerKind::Action)
                    .then(|| trigger.clone())
                    .flatten();
                let output_model =
                    index.model_reference(&value.output_model, value.span, &mut diagnostics);
                let output_field = index.field_reference(
                    output_model.as_ref(),
                    &value.output_field,
                    value.span,
                    &mut diagnostics,
                );
                let source = match &value.source {
                    FieldProducerSourceAst::ActionInput { input } => {
                        let input = if trigger_kind == ProductionTriggerKind::Action {
                            index.action_input_reference(
                                action.as_ref(),
                                input,
                                value.span,
                                &mut diagnostics,
                            )
                        } else {
                            index.event_input_reference(
                                trigger.as_ref(),
                                input,
                                value.span,
                                &mut diagnostics,
                            )
                        };
                        match trigger_kind {
                            ProductionTriggerKind::Action => {
                                UnlinkedFieldProducerSource::ActionInput {
                                    input: required_reference(input, value.span),
                                }
                            }
                            ProductionTriggerKind::Event => {
                                UnlinkedFieldProducerSource::EventInput {
                                    input: required_reference(input, value.span),
                                }
                            }
                        }
                    }
                    FieldProducerSourceAst::InputField { input, field } => {
                        let input = if trigger_kind == ProductionTriggerKind::Action {
                            index.action_input_reference(
                                action.as_ref(),
                                input,
                                value.span,
                                &mut diagnostics,
                            )
                        } else {
                            index.event_input_reference(
                                trigger.as_ref(),
                                input,
                                value.span,
                                &mut diagnostics,
                            )
                        };
                        let source_model = if trigger_kind == ProductionTriggerKind::Action {
                            index.action_input_model_reference(
                                action.as_ref(),
                                input.as_ref(),
                                value.span,
                                &mut diagnostics,
                            )
                        } else {
                            index.event_input_model_reference(
                                trigger.as_ref(),
                                input.as_ref(),
                                value.span,
                                &mut diagnostics,
                            )
                        };
                        let field = required_reference(
                            index.field_reference(
                                source_model.as_ref(),
                                field,
                                value.span,
                                &mut diagnostics,
                            ),
                            value.span,
                        );
                        match trigger_kind {
                            ProductionTriggerKind::Action => {
                                UnlinkedFieldProducerSource::InputField {
                                    input: required_reference(input, value.span),
                                    field,
                                }
                            }
                            ProductionTriggerKind::Event => {
                                UnlinkedFieldProducerSource::EventInputField {
                                    input: required_reference(input, value.span),
                                    field,
                                }
                            }
                        }
                    }
                    FieldProducerSourceAst::Constant { literal } => {
                        let enum_id =
                            index.field_enum_id(output_model.as_ref(), output_field.as_ref());
                        let literal = match literal {
                            LiteralAst::String(literal_value) => UnlinkedLiteral::String {
                                value: literal_value.clone(),
                                span: value.span,
                            },
                            LiteralAst::Integer(literal_value) => UnlinkedLiteral::Integer {
                                value: literal_value.clone(),
                                span: value.span,
                            },
                            LiteralAst::Boolean(literal_value) => UnlinkedLiteral::Boolean {
                                value: *literal_value,
                                span: value.span,
                            },
                            LiteralAst::Named(literal_value) => {
                                UnlinkedLiteral::Named(required_reference(
                                    index.enum_variant_reference(
                                        enum_id,
                                        literal_value,
                                        value.span,
                                        &mut diagnostics,
                                    ),
                                    value.span,
                                ))
                            }
                        };
                        UnlinkedFieldProducerSource::Constant { literal }
                    }
                    FieldProducerSourceAst::Template { value: template } => {
                        let mut parts = Vec::new();
                        let mut chars = template.chars().peekable();
                        let mut text = String::new();
                        while let Some(ch) = chars.next() {
                            match ch {
                                '{' if chars.peek() == Some(&'{') => {
                                    chars.next();
                                    text.push('{');
                                }
                                '}' if chars.peek() == Some(&'}') => {
                                    chars.next();
                                    text.push('}');
                                }
                                '{' => {
                                    if !text.is_empty() {
                                        parts.push(UnlinkedTemplatePart::Text {
                                            value: std::mem::take(&mut text),
                                        });
                                    }
                                    let mut name = String::new();
                                    for next in chars.by_ref() {
                                        if next == '}' {
                                            break;
                                        }
                                        name.push(next);
                                    }
                                    parts.push(UnlinkedTemplatePart::OutputField {
                                        field: required_reference(
                                            index.field_reference(
                                                output_model.as_ref(),
                                                &name,
                                                value.span,
                                                &mut diagnostics,
                                            ),
                                            value.span,
                                        ),
                                    });
                                }
                                other => text.push(other),
                            }
                        }
                        if !text.is_empty() {
                            parts.push(UnlinkedTemplatePart::Text { value: text });
                        }
                        UnlinkedFieldProducerSource::Template { parts }
                    }
                };
                module.field_producers.push(UnlinkedFieldProducer {
                    declaration: declaration(&value.declaration, true),
                    action: action.clone(),
                    trigger: UnlinkedProductionTrigger {
                        kind: trigger_kind,
                        reference: required_reference(trigger.clone(), value.span),
                    },
                    output_model: required_reference(output_model, value.span),
                    output_field: required_reference(output_field, value.span),
                    source,
                    condition: value.condition.as_ref().map(|condition| {
                        let action = action.as_ref();
                        let input = index.action_input_reference(
                            action,
                            &condition.input,
                            value.span,
                            &mut diagnostics,
                        );
                        let enum_type = index.action_input_enum_reference(
                            action,
                            input.as_ref(),
                            value.span,
                            &mut diagnostics,
                        );
                        let variant = enum_type
                            .as_ref()
                            .and_then(|enum_type| {
                                index.enum_variant_reference(
                                    Some(enum_type.id()),
                                    &condition.variant,
                                    value.span,
                                    &mut diagnostics,
                                )
                            })
                            .or_else(|| {
                                index.enum_variant_reference_any(
                                    &condition.variant,
                                    value.span,
                                    &mut diagnostics,
                                )
                            });
                        UnlinkedFieldProducerCondition::EnumVariant {
                            input: required_reference(input, value.span),
                            variant: required_reference(variant, value.span),
                        }
                    }),
                    span: value.span,
                });
            }
            DeclarationAst::RelationProducer(value) => {
                let trigger_kind = match value.trigger.kind {
                    ProducerTriggerKindAst::Action => ProductionTriggerKind::Action,
                    ProducerTriggerKindAst::Event => ProductionTriggerKind::Event,
                };
                let trigger = match trigger_kind {
                    ProductionTriggerKind::Action => {
                        index.action_reference(&value.trigger.name, value.span, &mut diagnostics)
                    }
                    ProductionTriggerKind::Event => {
                        index.event_reference(&value.trigger.name, value.span, &mut diagnostics)
                    }
                };
                let action = (trigger_kind == ProductionTriggerKind::Action)
                    .then(|| trigger.clone())
                    .flatten();
                let input = if trigger_kind == ProductionTriggerKind::Action {
                    index.action_input_reference(
                        action.as_ref(),
                        &value.input,
                        value.span,
                        &mut diagnostics,
                    )
                } else {
                    index.event_input_reference(
                        trigger.as_ref(),
                        &value.input,
                        value.span,
                        &mut diagnostics,
                    )
                };
                let output_model =
                    index.model_reference(&value.output_model, value.span, &mut diagnostics);
                let relation =
                    index.relation_reference(&value.relation, value.span, &mut diagnostics);
                module.relation_producers.push(UnlinkedRelationProducer {
                    declaration: declaration(&value.declaration, true),
                    action,
                    trigger: UnlinkedProductionTrigger {
                        kind: trigger_kind,
                        reference: required_reference(trigger, value.span),
                    },
                    input: required_reference(input, value.span),
                    output_model: required_reference(output_model, value.span),
                    relation: required_reference(relation, value.span),
                    span: value.span,
                });
            }
            DeclarationAst::Policy(value) => {
                let role = index.role_reference(&value.role, value.span, &mut diagnostics);
                let model = index.model_reference(&value.model, value.span, &mut diagnostics);
                let field = index.field_reference(
                    model.as_ref(),
                    &value.field,
                    value.span,
                    &mut diagnostics,
                );
                let action = index.action_reference(&value.action, value.span, &mut diagnostics);
                module.policies.push(UnlinkedPolicy {
                    // See the constraint note above. Locale display text never
                    // participates in the canonical generated ID.
                    declaration: declaration(&value.declaration, false),
                    role: required_reference(role, value.span),
                    model: required_reference(model, value.span),
                    field: required_reference(field, value.span),
                    action: required_reference(action, value.span),
                    effect: match value.effect {
                        PolicyEffectAst::Allow => PolicyEffect::Allow,
                        PolicyEffectAst::Deny => PolicyEffect::Deny,
                    },
                    span: value.span,
                });
            }
        }
    }

    let module = (!diagnostics.iter().any(Diagnostic::is_error)).then_some(module);
    FrontendOutput {
        module,
        diagnostics,
    }
}

fn workflow(
    value: &WorkflowAst,
    index: &StableIdIndex,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<UnlinkedWorkflow> {
    let start_screen = index.screen_reference(
        &value.start_screen.text,
        value.start_screen.span,
        diagnostics,
    );
    let initial_data = value
        .initial_data
        .iter()
        .filter_map(|item| workflow_data(item, index, diagnostics))
        .collect();
    let acquisitions = value
        .acquisitions
        .iter()
        .filter_map(|acquisition| {
            Some(UnlinkedWorkflowAcquisition {
                source_screen: index.screen_reference(
                    &acquisition.source_screen.text,
                    acquisition.source_screen.span,
                    diagnostics,
                )?,
                source_element: SurfaceRef::stable_id(
                    acquisition.source_element.text.clone(),
                    acquisition.source_element.span,
                ),
                data: acquisition
                    .data
                    .iter()
                    .filter_map(|item| workflow_data(item, index, diagnostics))
                    .collect(),
                span: acquisition.span,
            })
        })
        .collect();
    let completions = value
        .completions
        .iter()
        .filter_map(|completion| {
            let required_data = completion
                .required_data
                .iter()
                .filter_map(|item| workflow_data(item, index, diagnostics))
                .collect();
            Some(UnlinkedWorkflowCompletion {
                screen: index.screen_reference(
                    &completion.screen.text,
                    completion.screen.span,
                    diagnostics,
                )?,
                required_data,
                span: completion.span,
            })
        })
        .collect();
    Some(UnlinkedWorkflow {
        declaration: declaration(&value.declaration, true),
        start_screen: start_screen?,
        initial_data,
        acquisitions,
        completions,
        span: value.span,
    })
}

fn workflow_data(
    value: &WorkflowDataAst,
    index: &StableIdIndex,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<UnlinkedWorkflowData> {
    let model = index.model_reference(&value.model.text, value.model.span, diagnostics)?;
    let field = index.field_reference(
        Some(&model),
        &value.field.text,
        value.field.span,
        diagnostics,
    )?;
    Some(UnlinkedWorkflowData {
        model,
        field,
        span: value.span,
    })
}

fn lower_action_inputs(
    document: &DocumentAst,
    index: &StableIdIndex,
    diagnostics: &mut Vec<Diagnostic>,
) -> BTreeMap<String, Vec<UnlinkedActionInput>> {
    let mut inputs = BTreeMap::<String, Vec<UnlinkedActionInput>>::new();
    for item in &document.declarations {
        let DeclarationAst::ActionInput(value) = item else {
            continue;
        };
        let action = required_reference(
            index.action_reference(&value.action, value.span, diagnostics),
            value.span,
        );
        let kind = match &value.kind {
            ActionInputKindAst::ExistingModel { model } => UnlinkedActionInputKind::ExistingModel {
                model: required_reference(
                    index.model_reference(model, value.span, diagnostics),
                    value.span,
                ),
            },
            ActionInputKindAst::Value { value_type } => UnlinkedActionInputKind::Value {
                value_type: type_reference(value_type, value.span, index, diagnostics),
            },
        };
        inputs
            .entry(action.id().to_owned())
            .or_default()
            .push(UnlinkedActionInput {
                declaration: declaration(&value.declaration, true),
                kind,
                span: value.span,
            });
    }
    inputs
}

fn lower_event_inputs(
    document: &DocumentAst,
    index: &StableIdIndex,
    diagnostics: &mut Vec<Diagnostic>,
) -> BTreeMap<String, Vec<UnlinkedEventInput>> {
    let mut inputs = BTreeMap::<String, Vec<UnlinkedEventInput>>::new();
    for item in &document.declarations {
        let DeclarationAst::EventInput(value) = item else {
            continue;
        };
        let event = required_reference(
            index.event_reference(&value.event, value.span, diagnostics),
            value.span,
        );
        let kind = match &value.kind {
            ActionInputKindAst::ExistingModel { model } => UnlinkedEventInputKind::ExistingModel {
                model: required_reference(
                    index.model_reference(model, value.span, diagnostics),
                    value.span,
                ),
            },
            ActionInputKindAst::Value { value_type } => UnlinkedEventInputKind::Value {
                value_type: type_reference(value_type, value.span, index, diagnostics),
            },
        };
        inputs
            .entry(event.id().to_owned())
            .or_default()
            .push(UnlinkedEventInput {
                declaration: declaration(&value.declaration, true),
                kind,
                span: value.span,
            });
    }
    inputs
}

fn relation_references(
    values: &[String],
    span: Span,
    index: &StableIdIndex,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<SurfaceRef> {
    values
        .iter()
        .map(|value| required_reference(index.relation_reference(value, span, diagnostics), span))
        .collect()
}

/// Flattens the declared category tree into a pre-order list carrying parent
/// references.
///
/// The parent is a plain reference like every other one here: the frontend does
/// not resolve it, so a parent naming something that does not exist survives to
/// the analyzer, which is the only layer allowed to say so.
fn flatten_category(
    value: &CategoryAst,
    parent: Option<SurfaceRef>,
    index: &StableIdIndex,
    output: &mut Vec<UnlinkedCategory>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let id = value.declaration.id.clone();
    output.push(UnlinkedCategory {
        declaration: declaration(&value.declaration, true),
        parent,
        // A screen that does not resolve is reported once, here, and then left
        // out. Carrying a placeholder forward would make the analyzer say the
        // same thing a second time in different words.
        screens: value
            .screens
            .iter()
            .filter_map(|screen| index.screen_reference(&screen.text, screen.span, diagnostics))
            .collect(),
        span: value.span,
    });

    for child in &value.children {
        // Children point back at the id as written; the span is the parent's own
        // declaration so a diagnostic lands on the category, not on the child.
        flatten_category(
            child,
            Some(SurfaceRef::stable_id(id.clone(), value.declaration.span)),
            index,
            output,
            diagnostics,
        );
    }
}

/// Lowers one screen's layout, or drops it when the screen it belongs to does
/// not resolve — there is nothing to attach a layout to.
fn screen_layout(
    value: &ScreenLayoutAst,
    index: &StableIdIndex,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<UnlinkedScreenLayout> {
    let screen = index.screen_reference(&value.screen.text, value.screen.span, diagnostics)?;
    Some(UnlinkedScreenLayout {
        screen,
        roles: value
            .roles
            .iter()
            .filter_map(|v| index.role_reference(&v.text, v.span, diagnostics))
            .collect(),
        permissions: value
            .permissions
            .iter()
            .filter_map(|p| {
                let role = index.role_reference(&p.role.text, p.role.span, diagnostics)?;
                let action = index.action_reference(&p.action.text, p.action.span, diagnostics)?;
                let model = index.model_reference(&p.model.text, p.model.span, diagnostics)?;
                let field = p.field.as_ref().and_then(|f| {
                    index.field_reference(Some(&model), &f.text, f.span, diagnostics)
                });
                Some(UnlinkedScreenPermission {
                    role,
                    action,
                    model,
                    field,
                    span: p.span,
                })
            })
            .collect(),
        kind: value.kind.map(screen_layout_kind),
        elements: value
            .elements
            .iter()
            .filter_map(|element| layout_element(element, index, diagnostics))
            .collect(),
        span: value.span,
    })
}

const fn screen_layout_kind(value: ScreenLayoutKindAst) -> ScreenLayoutKind {
    match value {
        ScreenLayoutKindAst::Page => ScreenLayoutKind::Page,
        ScreenLayoutKindAst::Popup => ScreenLayoutKind::Popup,
        ScreenLayoutKindAst::Tab => ScreenLayoutKind::Tab,
        ScreenLayoutKindAst::Link => ScreenLayoutKind::Link,
    }
}

/// Lowers one layout element.
///
/// An element whose subject does not resolve is dropped rather than carried with
/// a placeholder: an input with no field and a list with no model do not mean
/// anything, and the reader has already been told why.
fn layout_element(
    value: &LayoutElementAst,
    index: &StableIdIndex,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<UnlinkedLayoutElement> {
    let lowered = match value {
        LayoutElementAst::Header { id, children, span } => UnlinkedLayoutElement::Header {
            id: id.clone(),
            children: children
                .iter()
                .filter_map(|child| layout_element(child, index, diagnostics))
                .collect(),
            span: *span,
        },
        LayoutElementAst::Section { id, children, span } => UnlinkedLayoutElement::Section {
            id: id.clone(),
            children: children
                .iter()
                .filter_map(|child| layout_element(child, index, diagnostics))
                .collect(),
            span: *span,
        },
        LayoutElementAst::Heading { id, text, span } => UnlinkedLayoutElement::Heading {
            id: id.clone(),
            text: text.clone(),
            span: *span,
        },
        LayoutElementAst::Form { id, inputs, span } => UnlinkedLayoutElement::Form {
            id: id.clone(),
            inputs: inputs
                .iter()
                .filter_map(|input| layout_element(input, index, diagnostics))
                .collect(),
            span: *span,
        },
        LayoutElementAst::Input { id, field, span } => UnlinkedLayoutElement::Input {
            id: id.clone(),
            field: index.unscoped_field_reference(&field.text, field.span, diagnostics)?,
            span: *span,
        },
        LayoutElementAst::List {
            id,
            model,
            fields,
            span,
        } => {
            let model_reference = index.model_reference(&model.text, model.span, diagnostics)?;
            UnlinkedLayoutElement::List {
                id: id.clone(),
                fields: fields
                    .iter()
                    .filter_map(|field| {
                        index.field_reference(
                            Some(&model_reference),
                            &field.text,
                            field.span,
                            diagnostics,
                        )
                    })
                    .collect(),
                model: model_reference,
                span: *span,
            }
        }
        LayoutElementAst::Button {
            id,
            name,
            action,
            span,
        } => UnlinkedLayoutElement::Button {
            id: id.clone(),
            name: name.clone(),
            // A button whose action does not resolve keeps its place. The button
            // is still there for a path to leave from; only the unreadable part
            // is dropped, and it was already reported.
            action: action
                .as_ref()
                .and_then(|value| index.action_reference(&value.text, value.span, diagnostics)),
            span: *span,
        },
        LayoutElementAst::Placeholder { id, text, span } => UnlinkedLayoutElement::Placeholder {
            id: id.clone(),
            text: text.clone(),
            span: *span,
        },
    };
    Some(lowered)
}

/// Lowers one path, or drops it when either endpoint screen does not resolve.
/// The element half is coined by the layout, so it stays as written and the
/// analyzer checks it against that screen's elements.
fn screen_path(
    value: &ScreenPathAst,
    index: &StableIdIndex,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<UnlinkedScreenPath> {
    let source_screen = index.screen_reference(
        &value.source_screen.text,
        value.source_screen.span,
        diagnostics,
    )?;
    let target_screen = value
        .target_screen
        .as_ref()
        .and_then(|v| index.screen_reference(&v.text, v.span, diagnostics));
    Some(UnlinkedScreenPath {
        id: value.id.clone(),
        source_screen,
        source_element: SurfaceRef::stable_id(
            value.source_element.text.clone(),
            value.source_element.span,
        ),
        target_screen,
        outcome: value.outcome.as_ref().map(|v| v.text.clone()),
        handler: value.handler.as_ref().map(|h| UnlinkedSameScreenHandler {
            kind: match h.kind {
                HandlerKindAst::State => UnlinkedHandlerKind::State,
                HandlerKindAst::Message => UnlinkedHandlerKind::Message,
                HandlerKindAst::Popup => UnlinkedHandlerKind::Popup,
                HandlerKindAst::Loading => UnlinkedHandlerKind::Loading,
            },
            id: h.id.clone(),
            content: h.content.clone(),
            span: h.span,
        }),
        label: value.label.clone(),
        span: value.span,
    })
}

fn action_outcomes(
    value: &ActionOutcomesAst,
    index: &StableIdIndex,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<UnlinkedActionOutcomes> {
    let action = index.action_reference(&value.action.text, value.action.span, diagnostics)?;
    let outcomes = value
        .outcomes
        .iter()
        .map(|o| UnlinkedActionOutcome {
            id: o.id.clone(),
            kind: match o.kind {
                OutcomeKindAst::Success => UnlinkedOutcomeKind::Success,
                OutcomeKindAst::Failure => UnlinkedOutcomeKind::Failure,
                OutcomeKindAst::Cancel => UnlinkedOutcomeKind::Cancel,
                OutcomeKindAst::Timeout => UnlinkedOutcomeKind::Timeout,
            },
            provided_data: o
                .provided_data
                .iter()
                .filter_map(|d| {
                    let model = index.model_reference(&d.model.text, d.model.span, diagnostics)?;
                    let field = index.field_reference(
                        Some(&model),
                        &d.field.text,
                        d.field.span,
                        diagnostics,
                    )?;
                    let source = match &d.source {
                        OutcomeDataSourceAst::Lookup { result } => {
                            UnlinkedOutcomeDataSource::Lookup {
                                result_id: result.text.clone(),
                            }
                        }
                        OutcomeDataSourceAst::Derivation { target } => {
                            UnlinkedOutcomeDataSource::Derivation {
                                target_field: index.unscoped_field_reference(
                                    &target.text,
                                    target.span,
                                    diagnostics,
                                )?,
                            }
                        }
                        OutcomeDataSourceAst::Producer { producer } => {
                            UnlinkedOutcomeDataSource::Producer {
                                producer_id: producer.text.clone(),
                            }
                        }
                    };
                    Some(UnlinkedOutcomeData {
                        model,
                        field,
                        source,
                        span: d.span,
                    })
                })
                .collect(),
            recovery: o.recovery.as_ref().map(|r| UnlinkedRecovery {
                kind: match r.kind {
                    RecoveryKindAst::Retry => UnlinkedRecoveryKind::Retry,
                    RecoveryKindAst::Return => UnlinkedRecoveryKind::Return,
                    RecoveryKindAst::Release => UnlinkedRecoveryKind::Release,
                },
                screen: r
                    .screen
                    .as_ref()
                    .and_then(|v| index.screen_reference(&v.text, v.span, diagnostics)),
                element: r
                    .element
                    .as_ref()
                    .map(|v| SurfaceRef::stable_id(v.text.clone(), v.span)),
                action: r
                    .action
                    .as_ref()
                    .and_then(|v| index.action_reference(&v.text, v.span, diagnostics)),
                path: r
                    .path
                    .as_ref()
                    .map(|v| SurfaceRef::stable_id(v.text.clone(), v.span)),
                span: r.span,
            }),
            span: o.span,
        })
        .collect();
    Some(UnlinkedActionOutcomes {
        action,
        outcomes,
        span: value.span,
    })
}

fn lookup_result(
    value: &LookupResultAst,
    index: &StableIdIndex,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<UnlinkedLookupResult> {
    let action = index.action_reference(&value.action.text, value.action.span, diagnostics)?;
    let input = index.action_input_reference(
        Some(&action),
        &value.input.text,
        value.input.span,
        diagnostics,
    )?;
    let model = index.model_reference(&value.model.text, value.model.span, diagnostics)?;
    let fields = value
        .fields
        .iter()
        .filter_map(|f| index.field_reference(Some(&model), &f.text, f.span, diagnostics))
        .collect();
    Some(UnlinkedLookupResult {
        id: value.id.clone(),
        action,
        input,
        model,
        fields,
        span: value.span,
    })
}

fn declaration(value: &NamedIdAst, keep_id: bool) -> UnlinkedDeclaration {
    UnlinkedDeclaration {
        name: value.name.clone(),
        id: keep_id.then(|| value.id.clone()),
        span: value.span,
    }
}

fn type_reference(
    value: &TypeReferenceAst,
    span: Span,
    index: &StableIdIndex,
    diagnostics: &mut Vec<Diagnostic>,
) -> UnlinkedTypeReference {
    match value {
        TypeReferenceAst::String => UnlinkedTypeReference::String,
        TypeReferenceAst::Integer => UnlinkedTypeReference::Integer,
        TypeReferenceAst::Boolean => UnlinkedTypeReference::Boolean,
        TypeReferenceAst::Decimal => UnlinkedTypeReference::Decimal,
        TypeReferenceAst::Date => UnlinkedTypeReference::Date,
        TypeReferenceAst::Time => UnlinkedTypeReference::Time,
        TypeReferenceAst::DateTime => UnlinkedTypeReference::DateTime,
        TypeReferenceAst::Duration => UnlinkedTypeReference::Duration,
        TypeReferenceAst::Latitude => UnlinkedTypeReference::Latitude,
        TypeReferenceAst::Longitude => UnlinkedTypeReference::Longitude,
        TypeReferenceAst::Money(currency) => UnlinkedTypeReference::Money(currency.clone()),
        TypeReferenceAst::Percentage => UnlinkedTypeReference::Percentage,
        TypeReferenceAst::Quantity(unit) => UnlinkedTypeReference::Quantity(unit.clone()),
        TypeReferenceAst::Coordinate => UnlinkedTypeReference::Coordinate,
        TypeReferenceAst::LocalDateTime => UnlinkedTypeReference::LocalDateTime,
        TypeReferenceAst::ZonedDateTime => UnlinkedTypeReference::ZonedDateTime,
        TypeReferenceAst::CalendarDuration => UnlinkedTypeReference::CalendarDuration,
        TypeReferenceAst::Uuid => UnlinkedTypeReference::Uuid,
        TypeReferenceAst::Email => UnlinkedTypeReference::Email,
        TypeReferenceAst::Url => UnlinkedTypeReference::Url,
        TypeReferenceAst::PhoneNumber => UnlinkedTypeReference::PhoneNumber,
        TypeReferenceAst::IpAddress => UnlinkedTypeReference::IpAddress,
        TypeReferenceAst::Cidr => UnlinkedTypeReference::Cidr,
        TypeReferenceAst::CountryCode => UnlinkedTypeReference::CountryCode,
        TypeReferenceAst::LanguageCode => UnlinkedTypeReference::LanguageCode,
        TypeReferenceAst::CurrencyCode => UnlinkedTypeReference::CurrencyCode,
        TypeReferenceAst::List(element) => {
            UnlinkedTypeReference::List(Box::new(type_reference(element, span, index, diagnostics)))
        }
        TypeReferenceAst::Set(element) => {
            UnlinkedTypeReference::Set(Box::new(type_reference(element, span, index, diagnostics)))
        }
        TypeReferenceAst::Map(key, value) => UnlinkedTypeReference::Map {
            key: Box::new(type_reference(key, span, index, diagnostics)),
            value: Box::new(type_reference(value, span, index, diagnostics)),
        },
        TypeReferenceAst::Reference(model) => UnlinkedTypeReference::Reference(required_reference(
            index.model_reference(model, span, diagnostics),
            span,
        )),
        TypeReferenceAst::Named(value) => UnlinkedTypeReference::Named(required_reference(
            index.enum_reference(value, span, diagnostics),
            span,
        )),
    }
}

fn screen_operation(value: ScreenOperationKindAst) -> ScreenOperationKind {
    match value {
        ScreenOperationKindAst::Create => ScreenOperationKind::Create,
        ScreenOperationKindAst::Read => ScreenOperationKind::Read,
        ScreenOperationKindAst::Input => ScreenOperationKind::Input,
        ScreenOperationKindAst::Update => ScreenOperationKind::Update,
        ScreenOperationKindAst::Delete => ScreenOperationKind::Delete,
    }
}

fn relation(value: RelationOperatorAst) -> RelationOperator {
    match value {
        RelationOperatorAst::Equal => RelationOperator::Equal,
        RelationOperatorAst::NotEqual => RelationOperator::NotEqual,
        RelationOperatorAst::LessThan => RelationOperator::LessThan,
        RelationOperatorAst::LessThanOrEqual => RelationOperator::LessThanOrEqual,
        RelationOperatorAst::GreaterThan => RelationOperator::GreaterThan,
        RelationOperatorAst::GreaterThanOrEqual => RelationOperator::GreaterThanOrEqual,
    }
}

fn operand_field_reference(
    value: &OperandAst,
    model: Option<&SurfaceRef>,
    span: Span,
    index: &StableIdIndex,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<SurfaceRef> {
    match value {
        OperandAst::Field(value) => index.field_reference(model, value, span, diagnostics),
        OperandAst::Literal(_) => None,
    }
}

fn operand(
    value: &OperandAst,
    field: Option<SurfaceRef>,
    expected_enum_id: Option<&str>,
    span: Span,
    index: &StableIdIndex,
    diagnostics: &mut Vec<Diagnostic>,
) -> UnlinkedOperand {
    match value {
        OperandAst::Field(_) => UnlinkedOperand::Field(required_reference(field, span)),
        OperandAst::Literal(value) => UnlinkedOperand::Literal(match value {
            LiteralAst::String(value) => UnlinkedLiteral::String {
                value: value.clone(),
                span,
            },
            LiteralAst::Integer(value) => UnlinkedLiteral::Integer {
                value: value.clone(),
                span,
            },
            LiteralAst::Boolean(value) => UnlinkedLiteral::Boolean {
                value: *value,
                span,
            },
            LiteralAst::Named(value) => UnlinkedLiteral::Named(required_reference(
                index.enum_variant_reference(expected_enum_id, value, span, diagnostics),
                span,
            )),
        }),
    }
}

#[cfg(test)]
mod tests {
    use rspdl_domain::{Frontend, UnlinkedOperand};

    use super::*;

    const FRONTMATTER_SOURCE: &str = r#"---
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
              - 입력: quantity
              - 입력: amount
          - 목록: { 모델: item, 필드: [quantity] }
          - 버튼: { id: submit, 이름: "담기", 행동: add_item }
          - 자리: "배송지 지도"
  cart_detail:
    레이아웃:
      - 제목: "장바구니"

흐름:
  - 출발: create_item.submit
    도착: cart_detail
    설명: "담기 성공"
---

장바구니 항목(item)은 다음 필드들로 구성되어 있다.
    수량(quantity): 필수 정수
    금액(amount): 필수 정수

장바구니 항목 입력 화면(create_item)에서는 장바구니 항목을 생성할 수 있다.
장바구니 항목 입력 화면(create_item)에서는 장바구니 항목의 수량, 금액을 입력할 수 있다.
장바구니 작성 화면(create_cart)에서는 장바구니 항목을 생성할 수 있다.
장바구니 상세 화면(cart_detail)에서는 장바구니 항목의 수량을 조회할 수 있다.
담기(add_item)는 행동이다.
"#;

    fn lowered_frontmatter() -> UnlinkedModule {
        let parsed = parse(FRONTMATTER_SOURCE);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let output = lower(&parsed.document.unwrap());
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
        output.module.unwrap()
    }

    #[test]
    fn flattens_the_category_tree_in_declaration_order() {
        let module = lowered_frontmatter();
        let ids: Vec<_> = module
            .information_architecture
            .iter()
            .map(|category| category.declaration.id.as_deref().unwrap())
            .collect();
        // 선언 순서 그대로의 pre-order 다. 정렬하지 않는다.
        assert_eq!(ids, vec!["order", "payment", "inquiry"]);

        let parents: Vec<_> = module
            .information_architecture
            .iter()
            .map(|category| category.parent.as_ref().map(SurfaceRef::id))
            .collect();
        assert_eq!(parents, vec![None, Some("order"), Some("order")]);

        let payment = &module.information_architecture[1];
        let screens: Vec<_> = payment.screens.iter().map(SurfaceRef::id).collect();
        assert_eq!(screens, vec!["create_cart", "create_item"]);
    }

    #[test]
    fn frontmatter_names_resolve_the_way_sentence_names_do() {
        let module = lowered_frontmatter();
        // 머리말에 적은 이름도 문장의 이름과 같은 해석기를 지난다. 그래서 여기 실린 것은
        // 사람이 쓴 이름이 아니라 이미 풀린 stable ID 다.
        let payment = &module.information_architecture[1];
        let screens: Vec<_> = payment.screens.iter().map(SurfaceRef::id).collect();
        assert_eq!(screens, vec!["create_cart", "create_item"]);
    }

    #[test]
    fn an_unknown_frontmatter_name_is_reported_once_at_the_locale_boundary() {
        let source = FRONTMATTER_SOURCE.replace("도착: cart_detail", "도착: 없는 화면");
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let output = lower(&parsed.document.unwrap());
        let keys: Vec<_> = output
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.message_key.as_str())
            .collect();
        // 원인 하나에 진단 하나. 풀리지 않은 경로는 여기서 빠지므로 분석기가 같은 말을
        // 다른 표현으로 한 번 더 하지 않는다.
        assert_eq!(keys, vec!["ko.reference.not_found"]);
        // 오류가 하나라도 있으면 frontend 는 모듈을 아예 내보내지 않는다. 분석기가 이
        // 문서를 볼 일이 없으므로 연쇄가 생길 자리도 없다.
        assert!(output.module.is_none());
    }

    #[test]
    fn lowers_the_layout_vocabulary_preserving_order() {
        let module = lowered_frontmatter();
        assert_eq!(module.screen_layouts.len(), 2);

        let layout = &module.screen_layouts[0];
        assert_eq!(layout.screen.id(), "create_item");
        assert_eq!(layout.kind, Some(ScreenLayoutKind::Page));

        let UnlinkedLayoutElement::Header { children, .. } = &layout.elements[0] else {
            panic!("첫 요소는 머리말이다: {:?}", layout.elements[0]);
        };
        assert!(matches!(
            &children[0],
            UnlinkedLayoutElement::Heading { text, .. } if text == "항목 추가"
        ));

        let UnlinkedLayoutElement::Section { children, .. } = &layout.elements[1] else {
            panic!("둘째 요소는 구역이다: {:?}", layout.elements[1]);
        };
        let UnlinkedLayoutElement::Form { inputs, .. } = &children[0] else {
            panic!("구역의 첫 자식은 폼이다: {:?}", children[0]);
        };
        let fields: Vec<_> = inputs
            .iter()
            .map(|input| match input {
                UnlinkedLayoutElement::Input { field, .. } => field.id(),
                other => panic!("폼 안에는 입력만 온다: {other:?}"),
            })
            .collect();
        assert_eq!(fields, vec!["quantity", "amount"]);

        assert!(matches!(
            &children[1],
            UnlinkedLayoutElement::List { model, fields, .. }
                if model.id() == "item" && fields.len() == 1 && fields[0].id() == "quantity"
        ));
        assert!(matches!(
            &children[2],
            UnlinkedLayoutElement::Button { id, name, action, .. }
                if id == "submit"
                    && name == "담기"
                    && action.as_ref().map(SurfaceRef::id) == Some("add_item")
        ));
        assert!(matches!(
            &children[3],
            UnlinkedLayoutElement::Placeholder { text, .. } if text == "배송지 지도"
        ));
    }

    #[test]
    fn an_unstated_screen_kind_stays_unstated() {
        let module = lowered_frontmatter();
        let layout = &module.screen_layouts[1];
        assert_eq!(layout.screen.id(), "cart_detail");
        // page 로 채우지 않는다. 선언되지 않은 의도를 추측하지 않는다.
        assert_eq!(layout.kind, None);
    }

    #[test]
    fn a_path_leaves_from_an_element_inside_a_screen() {
        let module = lowered_frontmatter();
        assert_eq!(module.screen_paths.len(), 1);
        let path = &module.screen_paths[0];
        assert_eq!(path.source_screen.id(), "create_item");
        assert_eq!(path.source_element.id(), "submit");
        assert_eq!(path.target_screen.as_ref().unwrap().id(), "cart_detail");
        // 설명은 조건식이 아니라 사람이 읽는 문자열이라 그대로 남는다.
        assert_eq!(path.label.as_deref(), Some("담기 성공"));
    }

    #[test]
    fn a_document_without_frontmatter_lowers_to_empty_structure_lists() {
        let source = r#"@모듈 승인(expense)
신청(request)은 다음 필드들로 구성되어 있다.
    금액(amount): 필수 정수
"#;
        let parsed = parse(source);
        let module = lower(&parsed.document.unwrap()).module.unwrap();
        assert!(module.information_architecture.is_empty());
        assert!(module.screen_layouts.is_empty());
        assert!(module.screen_paths.is_empty());
    }

    #[test]
    fn lowers_surface_names_to_stable_id_references() {
        let source = r#"@모듈 승인(expense)
신청(request)은 다음 필드들로 구성되어 있다.
    금액(amount): 필수 정수
신청의 금액은 0보다 커야 한다.
관리자(manager)는 역할이다.
변경(change)은 행동이다.
변경이 실행되면 신청을 수정한다.
관리자는 신청의 금액을 변경할 수 있다.
"#;
        let parsed = parse(source);
        assert!(parsed.diagnostics.is_empty());
        let output = lower(&parsed.document.unwrap());
        assert!(output.diagnostics.is_empty());
        let module = output.module.unwrap();

        assert_eq!(module.declaration.id.as_deref(), Some("expense"));
        assert_eq!(module.constraints[0].model.id(), "request");
        assert!(module.constraints[0].declaration.id.is_none());
        assert!(matches!(
            &module.constraints[0].left,
            UnlinkedOperand::Field(reference) if reference.id() == "amount"
        ));
        assert_eq!(module.policies[0].role.id(), "manager");
        assert!(module.policies[0].declaration.id.is_none());
        assert_eq!(module.action_data_mutations[0].action.id(), "change");
        assert_eq!(module.action_data_mutations[0].model.id(), "request");
        assert_eq!(
            module.action_data_mutations[0].mutation,
            DataMutationKind::Update
        );
    }

    #[test]
    fn unresolved_surface_names_stop_at_the_locale_boundary() {
        let source = r#"@모듈 승인(expense)
신청(request)은 다음 필드들로 구성되어 있다.
    금액(amount): 필수 정수
미등록자는 신청의 금액을 삭제할 수 있다.
"#;
        let output = KoreanFrontend.lower_source(source);

        assert!(output.module.is_none());
        assert!(
            output
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.rule_id == "RSPDL-KO-REF-001"
                    && diagnostic.message_key == "ko.reference.not_found"
                    && diagnostic.argument("kind") == Some("role")
                    && diagnostic.argument("reference") == Some("미등록자")),
            "{:?}",
            output.diagnostics
        );
    }

    #[test]
    fn ambiguous_surface_names_stop_at_the_locale_boundary() {
        let source = r#"@모듈 승인(expense)
신청(request)은 다음 필드들로 구성되어 있다.
    금액(amount): 필수 정수
request(other)은 다음 필드들로 구성되어 있다.
    값(value): 필수 정수
request의 금액은 0보다 커야 한다.
"#;
        let output = KoreanFrontend.lower_source(source);

        assert!(output.module.is_none());
        assert!(output.diagnostics.iter().any(|diagnostic| {
            diagnostic.rule_id == "RSPDL-KO-REF-002"
                && diagnostic.message_key == "ko.reference.ambiguous"
                && diagnostic.argument("kind") == Some("model")
                && diagnostic.argument("reference") == Some("request")
        }));
    }

    #[test]
    fn creation_trigger_reference_distinguishes_not_found_from_cross_kind_ambiguity() {
        let ambiguous = r#"@모듈 알림(notifications)
상태(status)는 다음 값 중 하나다.
    접수됨(received)
점검 전달 알림(notice)은 다음 필드들로 구성되어 있다.
    내용(content): 선택 문자열
같은 이름(same_action)은 행동이다.
같은 이름은 상태를 요청 상태(request_status)로 입력받는다.
같은 이름(same_event)은 사건이다.
같은 이름은 상태를 요청 상태(request_status)로 담는다.
알림 생성(create_notice)은 같은 이름의 요청 상태가 접수됨이면 점검 전달 알림을 하나 생성한다.
"#;
        let output = KoreanFrontend.lower_source(ambiguous);
        assert!(output.diagnostics.iter().any(|diagnostic| {
            diagnostic.rule_id == "RSPDL-KO-REF-002"
                && diagnostic.argument("kind") == Some("trigger")
                && diagnostic.argument("reference") == Some("같은 이름")
        }));

        let missing = ambiguous.replace("같은 이름의 요청 상태", "없는 사건의 요청 상태");
        let output = KoreanFrontend.lower_source(&missing);
        assert!(output.diagnostics.iter().any(|diagnostic| {
            diagnostic.rule_id == "RSPDL-KO-REF-001"
                && diagnostic.argument("kind") == Some("trigger")
                && diagnostic.argument("reference") == Some("없는 사건")
        }));
        assert!(
            !output
                .module
                .as_ref()
                .is_some_and(|module| serde_json::to_string(module).unwrap().contains("_invalid"))
        );
    }

    #[test]
    fn lowers_conditional_creation_with_action_scoped_enum_references() {
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
        let parsed = parse(source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let output = lower(&parsed.document.unwrap());
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
        let module = output.module.unwrap();
        assert_eq!(module.creation_branches.len(), 2);
        let create = &module.creation_branches[0];
        assert_eq!(
            create.declaration.id.as_deref(),
            Some("received_notice_create")
        );
        assert_eq!(create.action.as_ref().unwrap().id(), "assign_request");
        assert_eq!(create.input.id(), "request_status");
        assert_eq!(create.variant.id(), "received");
        assert_eq!(create.output_model.id(), "notice");
        assert_eq!(create.decision, CreationDecision::Create);
        assert_eq!(module.creation_branches[1].decision, CreationDecision::Skip);
    }

    #[test]
    fn scalar_creation_decision_passes_a_raw_variant_to_the_common_analyzer() {
        let source = r#"@모듈 알림(notifications)
점검 요청 전달 알림(notice)은 다음 필드들로 구성되어 있다.
    내용(content): 선택 문자열
점검 요청 전달(assign_request)은 행동이다.
점검 요청 전달은 문자열을 요청 상태(request_status)로 입력받는다.
접수 상태 알림 생성(received_notice_create)은 점검 요청 전달의 요청 상태가 접수됨이면 점검 요청 전달 알림을 하나 생성한다.
"#;
        let output = KoreanFrontend.lower_source(source);
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
        let module = output.module.unwrap();
        assert_eq!(module.creation_branches[0].variant.id(), "접수됨");
        let analyzed = rspdl_domain::analyze(module);
        assert!(analyzed.module.is_none());
        assert!(analyzed.diagnostics.iter().any(|diagnostic| {
            diagnostic.rule_id == "RSPDL-PROD-002"
                && diagnostic.message_key == "semantic.creation_branch.decision_input_requires_enum"
        }));
    }
}
