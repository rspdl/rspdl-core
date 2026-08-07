use rspdl_domain::{
    FieldIntentKind, Frontend, FrontendOutput, PolicyEffect, RelationOperator, ScreenOperationKind,
    SurfaceRef, UnlinkedAction, UnlinkedConstraint, UnlinkedDataModel, UnlinkedDeclaration,
    UnlinkedEnum, UnlinkedEnumVariant, UnlinkedField, UnlinkedFieldIntent, UnlinkedLiteral,
    UnlinkedModule, UnlinkedOperand, UnlinkedPolicy, UnlinkedRecalculation, UnlinkedRole,
    UnlinkedScreen, UnlinkedSumDerivation, UnlinkedTypeReference,
};

use crate::ast::*;
use crate::{Span, parse};

pub type LowerOutput = FrontendOutput;

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
/// This phase deliberately performs no symbol resolution, type checking, or
/// semantic analysis. Those rules belong to the shared domain analyzer.
pub fn lower(document: &DocumentAst) -> LowerOutput {
    let mut module = UnlinkedModule {
        declaration: declaration(&document.module.declaration, true),
        enums: Vec::new(),
        models: Vec::new(),
        screens: Vec::new(),
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
                    })
                    .collect(),
            }),
            DeclarationAst::DataModel(value) => module.models.push(UnlinkedDataModel {
                declaration: declaration(&value.declaration, true),
                fields: value
                    .fields
                    .iter()
                    .map(|field| UnlinkedField {
                        declaration: declaration(&field.declaration, true),
                        required: field.required,
                        value_type: type_reference(&field.value_type, field.declaration.span),
                    })
                    .collect(),
            }),
            DeclarationAst::Screen(value) => module.screens.push(UnlinkedScreen {
                declaration: declaration(&value.declaration, true),
                model: reference(&value.model, value.span),
                fields: value
                    .fields
                    .iter()
                    .map(|field| reference(field, value.span))
                    .collect(),
                operation: screen_operation(value.operation),
                span: value.span,
            }),
            DeclarationAst::SumDerivation(value) => {
                module.derivations.push(UnlinkedSumDerivation {
                    target_model: reference(&value.target_model, value.span),
                    target_field: reference(&value.target_field, value.span),
                    source_model: reference(&value.source_model, value.span),
                    source_field: reference(&value.source_field, value.span),
                    span: value.span,
                });
            }
            DeclarationAst::Recalculation(value) => {
                module.recalculations.push(UnlinkedRecalculation {
                    source_model: reference(&value.source_model, value.span),
                    source_field: reference(&value.source_field, value.span),
                    target_model: reference(&value.target_model, value.span),
                    target_field: reference(&value.target_field, value.span),
                    span: value.span,
                });
            }
            DeclarationAst::FieldIntent(value) => {
                module.field_intents.push(UnlinkedFieldIntent {
                    model: reference(&value.model, value.span),
                    field: reference(&value.field, value.span),
                    intent: match value.intent {
                        FieldIntentKindAst::Internal => FieldIntentKind::Internal,
                        FieldIntentKindAst::Hidden => FieldIntentKind::Hidden,
                    },
                    span: value.span,
                });
            }
            DeclarationAst::Constraint(value) => {
                module.constraints.push(UnlinkedConstraint {
                    // Anonymous semantic IDs are generated by the shared linker
                    // after references have been resolved to stable IDs.
                    declaration: declaration(&value.declaration, false),
                    model: reference(&value.expression.model, value.expression.span),
                    left: operand(&value.expression.left, value.expression.span),
                    operator: relation(value.expression.operator),
                    right: operand(&value.expression.right, value.expression.span),
                    span: value.expression.span,
                });
            }
            DeclarationAst::Role(value) => module.roles.push(UnlinkedRole {
                declaration: declaration(&value.declaration, true),
            }),
            DeclarationAst::Action(value) => module.actions.push(UnlinkedAction {
                declaration: declaration(&value.declaration, true),
            }),
            DeclarationAst::Policy(value) => module.policies.push(UnlinkedPolicy {
                // See the constraint note above. Locale display text never
                // participates in the canonical generated ID.
                declaration: declaration(&value.declaration, false),
                role: reference(&value.role, value.span),
                model: reference(&value.model, value.span),
                field: reference(&value.field, value.span),
                action: reference(&value.action, value.span),
                effect: match value.effect {
                    PolicyEffectAst::Allow => PolicyEffect::Allow,
                    PolicyEffectAst::Deny => PolicyEffect::Deny,
                },
                span: value.span,
            }),
        }
    }

    FrontendOutput {
        module: Some(module),
        diagnostics: Vec::new(),
    }
}

fn declaration(value: &NamedIdAst, keep_id: bool) -> UnlinkedDeclaration {
    UnlinkedDeclaration {
        name: value.name.clone(),
        id: keep_id.then(|| value.id.clone()),
        span: value.span,
    }
}

fn reference(value: &str, span: Span) -> SurfaceRef {
    SurfaceRef::new(value, span)
}

fn type_reference(value: &TypeReferenceAst, span: Span) -> UnlinkedTypeReference {
    match value {
        TypeReferenceAst::String => UnlinkedTypeReference::String,
        TypeReferenceAst::Integer => UnlinkedTypeReference::Integer,
        TypeReferenceAst::Boolean => UnlinkedTypeReference::Boolean,
        TypeReferenceAst::Named(value) => UnlinkedTypeReference::Named(reference(value, span)),
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

fn operand(value: &OperandAst, span: Span) -> UnlinkedOperand {
    match value {
        OperandAst::Field(value) => UnlinkedOperand::Field(reference(value, span)),
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
            LiteralAst::Named(value) => UnlinkedLiteral::Named(reference(value, span)),
        }),
    }
}

#[cfg(test)]
mod tests {
    use rspdl_domain::{Frontend, UnlinkedOperand};

    use super::*;

    #[test]
    fn lowers_surface_references_without_resolving_them() {
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
        let output = lower(&parsed.document.unwrap());
        assert!(output.diagnostics.is_empty());
        let module = output.module.unwrap();

        assert_eq!(module.declaration.id.as_deref(), Some("expense"));
        assert_eq!(module.constraints[0].model.text, "신청");
        assert!(module.constraints[0].declaration.id.is_none());
        assert!(matches!(
            &module.constraints[0].left,
            UnlinkedOperand::Field(reference) if reference.text == "금액"
        ));
        assert_eq!(module.policies[0].role.text, "관리자");
        assert!(module.policies[0].declaration.id.is_none());
    }

    #[test]
    fn frontend_contract_stops_after_syntax_and_desugaring() {
        let source = r#"@모듈 승인(expense)
신청(request)은 다음 필드들로 구성되어 있다.
    금액(amount): 필수 정수
없는 역할은 신청의 금액을 없는 행동할 수 있다.
"#;
        let output = KoreanFrontend.lower_source(source);

        assert!(output.module.is_some());
        assert!(
            output
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.rule_id.starts_with("RSPDL-KO-"))
        );
    }
}
