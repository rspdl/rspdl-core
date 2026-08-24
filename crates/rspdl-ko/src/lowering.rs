use std::collections::BTreeSet;

use rspdl_domain::{
    DataMutationKind, Diagnostic, FieldIntentKind, Frontend, FrontendOutput, PolicyEffect,
    RelationOperator, ScreenOperationKind, SurfaceRef, UnlinkedAction, UnlinkedActionDataMutation,
    UnlinkedConstraint, UnlinkedDataModel, UnlinkedDeclaration, UnlinkedEnum, UnlinkedEnumVariant,
    UnlinkedField, UnlinkedFieldIntent, UnlinkedLiteral, UnlinkedModule, UnlinkedOperand,
    UnlinkedPolicy, UnlinkedRecalculation, UnlinkedRelation, UnlinkedRelationalConstraint,
    UnlinkedRelationalConstraintKind, UnlinkedRole, UnlinkedScreen, UnlinkedSumDerivation,
    UnlinkedTypeReference,
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

#[derive(Clone, Debug, Default)]
struct StableIdIndex {
    enums: Vec<EnumSymbols>,
    models: Vec<ModelSymbols>,
    relations: Vec<Symbol>,
    roles: Vec<Symbol>,
    actions: Vec<Symbol>,
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
                _ => {}
            }
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
        policies: Vec::new(),
    };

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
                span: value.span,
            }),
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
}
