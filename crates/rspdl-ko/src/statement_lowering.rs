use super::*;
use rspdl_domain::{
    UnlinkedStatement, UnlinkedStatementActor, UnlinkedStatementAssignment,
    UnlinkedStatementBinding, UnlinkedStatementCondition as Condition,
    UnlinkedStatementEffect as Effect, UnlinkedStatementOperand as Operand,
};
struct Scope<'a> {
    value: &'a StatementAst,
    index: &'a StableIdIndex,
    trigger: SurfaceRef,
    kind: ProductionTriggerKind,
}
impl Scope<'_> {
    fn binding(&self, name: &str, span: Span, d: &mut Vec<Diagnostic>) -> Option<SurfaceRef> {
        let symbols = self
            .value
            .bindings
            .iter()
            .map(|b| Symbol::from(&b.declaration))
            .collect::<Vec<_>>();
        resolve_symbols(symbols.iter(), name, "statement_binding", span, d)
    }
    fn input(&self, b: &StatementBindingAst, d: &mut Vec<Diagnostic>) -> Option<SurfaceRef> {
        let owner = match self.kind {
            ProductionTriggerKind::Action => self.index.action_reference(&b.owner, b.span, d),
            ProductionTriggerKind::Event => self.index.event_reference(&b.owner, b.span, d),
        }?;
        if owner.id() != self.trigger.id() {
            d.push(Diagnostic::error(
                "RSPDL-KO-REF-003",
                "ko.statement.binding_trigger_mismatch",
                b.span,
            ));
            return None;
        }
        match self.kind {
            ProductionTriggerKind::Action => {
                self.index
                    .action_input_reference(Some(&self.trigger), &b.input, b.span, d)
            }
            ProductionTriggerKind::Event => {
                self.index
                    .event_input_reference(Some(&self.trigger), &b.input, b.span, d)
            }
        }
    }
    fn model(&self, binding: &SurfaceRef, d: &mut Vec<Diagnostic>) -> Option<SurfaceRef> {
        let b = self
            .value
            .bindings
            .iter()
            .find(|b| b.declaration.id == binding.id())?;
        let input = self.input(b, d)?;
        match self.kind {
            ProductionTriggerKind::Action => self.index.action_input_model_reference(
                Some(&self.trigger),
                Some(&input),
                b.span,
                d,
            ),
            ProductionTriggerKind::Event => {
                self.index
                    .event_input_model_reference(Some(&self.trigger), Some(&input), b.span, d)
            }
        }
    }
    fn field(
        &self,
        b: &SurfaceRef,
        field: &str,
        span: Span,
        d: &mut Vec<Diagnostic>,
    ) -> SurfaceRef {
        let model = self.model(b, d);
        required_reference(
            self.index.field_reference(model.as_ref(), field, span, d),
            span,
        )
    }
    fn operand(&self, value: &StatementOperandAst, span: Span, d: &mut Vec<Diagnostic>) -> Operand {
        match value {
            StatementOperandAst::Input(name) => {
                // Declared aliases take precedence; an undeclared bare name is an enum variant.
                if self
                    .value
                    .bindings
                    .iter()
                    .any(|b| b.declaration.id == *name || b.declaration.name == *name)
                {
                    Operand::Input(required_reference(self.binding(name, span, d), span))
                } else {
                    Operand::Literal(UnlinkedLiteral::Named(required_reference(
                        self.index.enum_variant_reference_any(name, span, d),
                        span,
                    )))
                }
            }
            StatementOperandAst::InputField { binding, field } => {
                let b = required_reference(self.binding(binding, span, d), span);
                let f = self.field(&b, field, span, d);
                Operand::InputField {
                    binding: b,
                    field: f,
                }
            }
            StatementOperandAst::Literal(l) => Operand::Literal(match l {
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
                LiteralAst::Named(name) => UnlinkedLiteral::Named(required_reference(
                    self.index.enum_variant_reference_any(name, span, d),
                    span,
                )),
            }),
        }
    }
    fn condition(&self, v: &StatementConditionAst, d: &mut Vec<Diagnostic>) -> Condition {
        match v {
            StatementConditionAst::True => Condition::True,
            StatementConditionAst::Compare {
                left,
                operator,
                right,
                span,
            } => Condition::Compare {
                left: self.operand(left, *span, d),
                operator: match operator {
                    RelationOperatorAst::Equal => RelationOperator::Equal,
                    RelationOperatorAst::NotEqual => RelationOperator::NotEqual,
                    RelationOperatorAst::LessThan => RelationOperator::LessThan,
                    RelationOperatorAst::LessThanOrEqual => RelationOperator::LessThanOrEqual,
                    RelationOperatorAst::GreaterThan => RelationOperator::GreaterThan,
                    RelationOperatorAst::GreaterThanOrEqual => RelationOperator::GreaterThanOrEqual,
                },
                right: self.operand(right, *span, d),
            },
            StatementConditionAst::And(xs) => {
                Condition::And(xs.iter().map(|x| self.condition(x, d)).collect())
            }
            StatementConditionAst::Or(xs) => {
                Condition::Or(xs.iter().map(|x| self.condition(x, d)).collect())
            }
            StatementConditionAst::Not(x) => Condition::Not(Box::new(self.condition(x, d))),
        }
    }
}
pub(super) fn lower(
    value: &StatementAst,
    index: &StableIdIndex,
    d: &mut Vec<Diagnostic>,
) -> Option<UnlinkedStatement> {
    let kind = match value.trigger.kind {
        ProducerTriggerKindAst::Action => ProductionTriggerKind::Action,
        ProducerTriggerKindAst::Event => ProductionTriggerKind::Event,
    };
    let trigger = match kind {
        ProductionTriggerKind::Action => {
            index.action_reference(&value.trigger.name, value.trigger_span, d)
        }
        ProductionTriggerKind::Event => {
            index.event_reference(&value.trigger.name, value.trigger_span, d)
        }
    }?;
    let scope = Scope {
        value,
        index,
        trigger: trigger.clone(),
        kind,
    };
    let actor = if value.actor == "시스템" {
        UnlinkedStatementActor::System
    } else {
        UnlinkedStatementActor::Role(required_reference(
            index.role_reference(&value.actor, value.actor_span, d),
            value.actor_span,
        ))
    };
    let bindings = value
        .bindings
        .iter()
        .map(|b| UnlinkedStatementBinding {
            declaration: declaration(&b.declaration, true),
            input: required_reference(scope.input(b, d), b.span),
        })
        .collect();
    let condition = scope.condition(&value.condition, d);
    let effects = value
        .effects
        .iter()
        .map(|effect| match effect {
            StatementEffectAst::Read { binding, span } => Effect::Read {
                binding: required_reference(scope.binding(binding, *span, d), *span),
                span: *span,
            },
            StatementEffectAst::Delete { binding, span } => Effect::Delete {
                binding: required_reference(scope.binding(binding, *span, d), *span),
                span: *span,
            },
            StatementEffectAst::Update {
                binding,
                assignments,
                span,
            } => {
                let b = required_reference(scope.binding(binding, *span, d), *span);
                let assignments = assignments
                    .iter()
                    .map(|a| UnlinkedStatementAssignment {
                        field: scope.field(&b, &a.field, a.span, d),
                        value: scope.operand(&a.value, a.span, d),
                        span: a.span,
                    })
                    .collect();
                Effect::Update {
                    binding: b,
                    assignments,
                    span: *span,
                }
            }
        })
        .collect();
    Some(UnlinkedStatement {
        declaration: declaration(&value.declaration, true),
        trigger: UnlinkedProductionTrigger {
            kind,
            reference: trigger,
        },
        actor,
        actor_span: value.actor_span,
        bindings,
        condition,
        policy: value.policy,
        effects,
        span: value.span,
    })
}
