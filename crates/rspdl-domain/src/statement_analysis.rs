use crate::*;
use std::collections::{BTreeMap, BTreeSet};

/// Appends a statement-scoped error at the supplied source byte range.
fn error(d: &mut Vec<Diagnostic>, code: &str, key: &str, span: TextRange) {
    d.push(Diagnostic::error(
        code,
        format!("semantic.statement.{key}"),
        span,
    ));
}
/// Accepts an exact canonical ID, or a final ID segment for an unqualified reference.
fn matches(id: &CanonicalId, r: &SurfaceRef) -> bool {
    id.as_str() == r.id()
        || (!r.id().contains('.') && id.as_str().rsplit('.').next() == Some(r.id()))
}
/// Resolves exactly one matching declaration; missing and ambiguous references both
/// return `None` so callers can attach their context-specific diagnostic.
fn resolve<'a, T>(
    values: &'a [T],
    id: impl Fn(&T) -> &CanonicalId,
    r: &SurfaceRef,
) -> Option<&'a T> {
    let mut found = values.iter().filter(|v| matches(id(v), r));
    let first = found.next()?;
    if found.next().is_some() {
        None
    } else {
        Some(first)
    }
}

/// Links trigger-owned inputs, actors, guards, and effects without executing policy.
/// Statements with any new diagnostic are omitted; IDs remain reserved in `used` even
/// when later checks fail. Output is sorted canonically and retains diagnostic spans.
#[allow(clippy::too_many_arguments)]
pub(crate) fn analyze_statements(
    values: Vec<UnlinkedStatement>,
    module: &CanonicalId,
    actions: &[ActionDefinition],
    events: &[EventDefinition],
    roles: &[RoleDefinition],
    models: &[DataModelDefinition],
    enums: &[EnumDefinition],
    used: &mut BTreeSet<CanonicalId>,
    d: &mut Vec<Diagnostic>,
) -> Vec<StatementDefinition> {
    let mut result = Vec::new();
    for value in values {
        let before = d.len();
        let Some(raw) = value.declaration.id.as_ref() else {
            error(d, "RSPDL-STMT-001", "stable_id_required", value.span);
            continue;
        };
        let id = match CanonicalId::new(if raw.contains('.') {
            raw.clone()
        } else {
            format!("{module}.{raw}")
        }) {
            Ok(id) => id,
            Err(_) => {
                error(d, "RSPDL-STMT-001", "invalid_id", value.span);
                continue;
            }
        };
        if !used.insert(id.clone()) {
            error(d, "RSPDL-STMT-001", "duplicate_id", value.span);
        }
        let (trigger, inputs) = match value.trigger.kind {
            ProductionTriggerKind::Action => {
                match resolve(actions, |v| &v.id, &value.trigger.reference) {
                    Some(a) => (
                        ProductionTriggerDefinition::Action(a.id.clone()),
                        a.inputs
                            .iter()
                            .map(|i| {
                                (
                                    i.id.clone(),
                                    i.local_id.clone(),
                                    match &i.kind {
                                        ActionInputKind::ExistingModel { model_id } => {
                                            StatementBindingKind::ExistingModel {
                                                model_id: model_id.clone(),
                                            }
                                        }
                                        ActionInputKind::Value { value_type } => {
                                            StatementBindingKind::Value {
                                                value_type: value_type.clone(),
                                            }
                                        }
                                    },
                                )
                            })
                            .collect::<Vec<_>>(),
                    ),
                    None => {
                        error(
                            d,
                            "RSPDL-STMT-002",
                            "trigger_not_found",
                            value.trigger.reference.span,
                        );
                        continue;
                    }
                }
            }
            ProductionTriggerKind::Event => {
                match resolve(events, |v| &v.id, &value.trigger.reference) {
                    Some(a) => (
                        ProductionTriggerDefinition::Event(a.id.clone()),
                        a.inputs
                            .iter()
                            .map(|i| {
                                (
                                    i.id.clone(),
                                    i.local_id.clone(),
                                    match &i.kind {
                                        EventInputKind::ExistingModel { model_id } => {
                                            StatementBindingKind::ExistingModel {
                                                model_id: model_id.clone(),
                                            }
                                        }
                                        EventInputKind::Value { value_type } => {
                                            StatementBindingKind::Value {
                                                value_type: value_type.clone(),
                                            }
                                        }
                                    },
                                )
                            })
                            .collect::<Vec<_>>(),
                    ),
                    None => {
                        error(
                            d,
                            "RSPDL-STMT-002",
                            "trigger_not_found",
                            value.trigger.reference.span,
                        );
                        continue;
                    }
                }
            }
        };
        let actor_span = match &value.actor {
            UnlinkedStatementActor::System => value.actor_span,
            UnlinkedStatementActor::Role(r) => r.span,
        };
        let actor = match value.actor {
            UnlinkedStatementActor::System => StatementActor::System,
            UnlinkedStatementActor::Role(r) => match resolve(roles, |v| &v.id, &r) {
                Some(role) => StatementActor::Role(role.id.clone()),
                None => {
                    error(d, "RSPDL-STMT-003", "role_not_found", r.span);
                    continue;
                }
            },
        };
        let mut bindings = Vec::new();
        let mut names = BTreeSet::new();
        for b in value.bindings {
            let Some(raw) = b.declaration.id.as_ref() else {
                error(
                    d,
                    "RSPDL-STMT-001",
                    "stable_id_required",
                    b.declaration.span,
                );
                continue;
            };
            let bid = match CanonicalId::new(raw) {
                Ok(v) => v,
                Err(_) => {
                    error(d, "RSPDL-STMT-001", "invalid_id", b.declaration.span);
                    continue;
                }
            };
            if !names.insert(bid.clone()) {
                error(d, "RSPDL-STMT-004", "duplicate_binding", b.declaration.span);
                continue;
            }
            let found = inputs
                .iter()
                .filter(|(iid, local, _)| {
                    iid.as_str() == b.input.id() || local.as_str() == b.input.id()
                })
                .collect::<Vec<_>>();
            if found.len() != 1 {
                error(d, "RSPDL-STMT-004", "input_owner_mismatch", b.input.span);
                continue;
            }
            bindings.push(StatementBinding {
                id: bid,
                name: b.declaration.name,
                input_id: found[0].0.clone(),
                input_span: b.input.span,
                phase: match trigger {
                    ProductionTriggerDefinition::Action(_) => ProducerPhase::PreMutation,
                    ProductionTriggerDefinition::Event(_) => ProducerPhase::TriggerPayload,
                },
                kind: found[0].2.clone(),
                span: b.declaration.span,
            });
        }
        bindings.sort_by(|a, b| a.id.cmp(&b.id));
        let ctx = Context {
            span: value.span,
            bindings: &bindings,
            models,
            enums,
        };
        let condition = ctx.condition(value.condition, d);
        let mut effects = Vec::new();
        let mut writes = BTreeSet::new();
        let mut accesses = BTreeMap::<CanonicalId, (bool, bool)>::new();
        for effect in value.effects {
            let (reference, span) = match &effect {
                UnlinkedStatementEffect::Read { binding, span }
                | UnlinkedStatementEffect::Delete { binding, span }
                | UnlinkedStatementEffect::Update { binding, span, .. } => (binding, *span),
            };
            let Some(binding) = ctx.binding(reference, d) else {
                continue;
            };
            let StatementBindingKind::ExistingModel { model_id } = &binding.kind else {
                error(d, "RSPDL-STMT-007", "effect_requires_model", span);
                continue;
            };
            if matches!(trigger, ProductionTriggerDefinition::Event(_))
                && !matches!(effect, UnlinkedStatementEffect::Read { .. })
            {
                error(d, "RSPDL-STMT-012", "event_payload_read_only", span);
                continue;
            }
            let state = accesses.entry(binding.input_id.clone()).or_default();
            match effect {
                UnlinkedStatementEffect::Read { .. } => {
                    state.1 = true;
                    effects.push(StatementEffect::Read {
                        binding_id: binding.id.clone(),
                        model_id: model_id.clone(),
                        span,
                    });
                }
                UnlinkedStatementEffect::Delete { .. } => {
                    if state.0 {
                        error(d, "RSPDL-STMT-009", "duplicate_delete", span);
                    }
                    state.0 = true;
                    effects.push(StatementEffect::Delete {
                        binding_id: binding.id.clone(),
                        model_id: model_id.clone(),
                        span,
                    });
                }
                UnlinkedStatementEffect::Update { assignments, .. } => {
                    state.1 = true;
                    let mut linked = Vec::new();
                    if assignments.is_empty() {
                        error(d, "RSPDL-STMT-007", "empty_update", span);
                    }
                    for a in assignments {
                        let Some(field) = ctx.field(model_id, &a.field, d) else {
                            continue;
                        };
                        if !writes.insert((binding.input_id.clone(), field.id.clone())) {
                            d.push(
                                Diagnostic::error(
                                    "RSPDL-STMT-008",
                                    "semantic.statement.duplicate_write",
                                    a.span,
                                )
                                .with_argument("statement_id", &id)
                                .with_argument("input_id", &binding.input_id)
                                .with_argument("field_id", &field.id),
                            );
                        }
                        if let Some(value) = ctx.operand(a.value, Some(&field.value_type), d) {
                            if value.value_type() != &field.value_type {
                                error(d, "RSPDL-STMT-006", "type_mismatch", a.span);
                            } else {
                                linked.push(StatementAssignment {
                                    field_id: field.id.clone(),
                                    value,
                                    span: a.span,
                                });
                            }
                        }
                    }
                    linked.sort_by(|a, b| a.field_id.cmp(&b.field_id));
                    effects.push(StatementEffect::Update {
                        binding_id: binding.id.clone(),
                        model_id: model_id.clone(),
                        assignments: linked,
                        span,
                    });
                }
            }
        }
        if effects.is_empty() {
            error(d, "RSPDL-STMT-007", "effects_required", value.span);
        }
        if accesses.values().any(|(delete, other)| *delete && *other) {
            error(d, "RSPDL-STMT-009", "delete_access_conflict", value.span);
        }
        effects.sort_by_key(effect_key);
        if d.len() == before {
            result.push(StatementDefinition {
                id,
                name: value.declaration.name,
                trigger_span: value.trigger.reference.span,
                actor_span,
                trigger,
                actor,
                bindings,
                condition: condition.unwrap_or(StatementCondition::True),
                policy: value.policy,
                effects,
                analysis_status: StatementAnalysisStatus::LinkedAndTypeCheckedOnly,
                span: value.span,
            });
        }
    }
    result.sort_by(|a, b| a.id.cmp(&b.id));
    result
}
/// Produces a source-position-independent key for deterministic effect ordering.
fn effect_key(e: &StatementEffect) -> String {
    semantic_key(e)
}

struct Context<'a> {
    span: TextRange,
    bindings: &'a [StatementBinding],
    models: &'a [DataModelDefinition],
    enums: &'a [EnumDefinition],
}
impl<'a> Context<'a> {
    /// Resolves a statement-local binding ID exactly, reporting failure at the reference.
    fn binding(&self, r: &SurfaceRef, d: &mut Vec<Diagnostic>) -> Option<&'a StatementBinding> {
        let value = self.bindings.iter().find(|b| b.id.as_str() == r.id());
        if value.is_none() {
            d.push(
                Diagnostic::error(
                    "RSPDL-STMT-004",
                    "semantic.statement.binding_not_found",
                    r.span,
                )
                .with_argument("reference", r.id()),
            );
        }
        value
    }
    /// Resolves a field only within its bound model; ambiguity and wrong ownership
    /// produce a diagnostic at the field reference rather than a global fallback.
    fn field(
        &self,
        model: &CanonicalId,
        r: &SurfaceRef,
        d: &mut Vec<Diagnostic>,
    ) -> Option<&'a FieldDefinition> {
        let value = self
            .models
            .iter()
            .find(|m| &m.id == model)
            .and_then(|m| resolve(&m.fields, |f| &f.id, r));
        if value.is_none() {
            d.push(
                Diagnostic::error(
                    "RSPDL-STMT-005",
                    "semantic.statement.field_owner_mismatch",
                    r.span,
                )
                .with_argument("reference", r.id())
                .with_argument("model_id", model),
            );
        }
        value
    }
    /// Links scalar inputs, required model fields, or typed literals while retaining
    /// their source ranges. Named literals need an expected type; optional fields and
    /// model-valued inputs are unsupported and return `None` with a diagnostic.
    fn operand(
        &self,
        o: UnlinkedStatementOperand,
        expected: Option<&CanonicalType>,
        d: &mut Vec<Diagnostic>,
    ) -> Option<StatementOperand> {
        match o {
            UnlinkedStatementOperand::Input(r) => {
                let b = self.binding(&r, d)?;
                let StatementBindingKind::Value { value_type } = &b.kind else {
                    error(d, "RSPDL-STMT-006", "scalar_required", r.span);
                    return None;
                };
                Some(StatementOperand::Input {
                    binding_id: b.id.clone(),
                    input_id: b.input_id.clone(),
                    value_type: value_type.clone(),
                    phase: b.phase,
                    span: r.span,
                })
            }
            UnlinkedStatementOperand::InputField { binding, field } => {
                let b = self.binding(&binding, d)?;
                let StatementBindingKind::ExistingModel { model_id } = &b.kind else {
                    error(d, "RSPDL-STMT-005", "model_required", binding.span);
                    return None;
                };
                let f = self.field(model_id, &field, d)?;
                if !f.required {
                    error(
                        d,
                        "RSPDL-STMT-011",
                        "optional_operand_unsupported",
                        field.span,
                    );
                    return None;
                }
                Some(StatementOperand::InputField {
                    binding_id: b.id.clone(),
                    input_id: b.input_id.clone(),
                    model_id: model_id.clone(),
                    field_id: f.id.clone(),
                    value_type: f.value_type.clone(),
                    phase: b.phase,
                    span: field.span,
                })
            }
            UnlinkedStatementOperand::Literal(l) => {
                let span = match &l {
                    UnlinkedLiteral::String { span, .. }
                    | UnlinkedLiteral::Integer { span, .. }
                    | UnlinkedLiteral::Boolean { span, .. } => *span,
                    UnlinkedLiteral::Named(r) => r.span,
                };
                let inferred = match &l {
                    UnlinkedLiteral::String { .. } => Some(CanonicalType::String),
                    UnlinkedLiteral::Integer { .. } => Some(CanonicalType::Integer),
                    UnlinkedLiteral::Boolean { .. } => Some(CanonicalType::Boolean),
                    _ => None,
                };
                let Some(ty) = expected.or(inferred.as_ref()) else {
                    error(d, "RSPDL-STMT-006", "literal_type_required", span);
                    return None;
                };
                let enums = self
                    .enums
                    .iter()
                    .map(|e| (e.id.to_string(), e.clone()))
                    .collect();
                let mut errors = Vec::new();
                let value = crate::analysis::literal_value(&l, ty, &enums, &mut errors);
                if value.is_none() {
                    error(d, "RSPDL-STMT-006", "literal_type_mismatch", span);
                }
                value.map(|value| StatementOperand::Constant { value, span })
            }
        }
    }
    /// Recursively type-checks guards, inferring literal types from the other operand.
    /// Rejects empty groups and unsupported comparisons; group children are sorted
    /// semantically without flattening their boolean structure.
    fn condition(
        &self,
        c: UnlinkedStatementCondition,
        d: &mut Vec<Diagnostic>,
    ) -> Option<StatementCondition> {
        let is_and = matches!(&c, UnlinkedStatementCondition::And(_));
        match c {
            UnlinkedStatementCondition::True => Some(StatementCondition::True),
            UnlinkedStatementCondition::Compare {
                left,
                operator,
                right,
            } => {
                // Resolve a non-literal first so literals inherit the declared type.
                let (l, r) = if matches!(left, UnlinkedStatementOperand::Literal(_))
                    && !matches!(right, UnlinkedStatementOperand::Literal(_))
                {
                    let r = self.operand(right, None, d)?;
                    let l = self.operand(left, Some(r.value_type()), d)?;
                    (l, r)
                } else {
                    let l = self.operand(left, None, d)?;
                    let r = self.operand(right, Some(l.value_type()), d)?;
                    (l, r)
                };
                let span = match &l {
                    StatementOperand::Input { span, .. }
                    | StatementOperand::InputField { span, .. }
                    | StatementOperand::Constant { span, .. } => *span,
                };
                if l.value_type() != r.value_type() {
                    error(d, "RSPDL-STMT-006", "type_mismatch", span);
                    return None;
                }
                if !scalar(l.value_type())
                    || (!matches!(
                        operator,
                        RelationOperator::Equal | RelationOperator::NotEqual
                    ) && !l.value_type().is_ordered())
                {
                    error(d, "RSPDL-STMT-006", "comparison_unsupported", span);
                    return None;
                }
                Some(StatementCondition::Compare {
                    left: Box::new(l),
                    operator,
                    right: Box::new(r),
                })
            }
            UnlinkedStatementCondition::And(xs) | UnlinkedStatementCondition::Or(xs) => {
                if xs.is_empty() {
                    error(d, "RSPDL-STMT-010", "empty_condition_group", self.span);
                    return None;
                }
                let mut out = Vec::new();
                for x in xs {
                    out.push(self.condition(x, d)?);
                }
                out.sort_by_key(condition_key);

                if is_and {
                    Some(StatementCondition::And(out))
                } else {
                    Some(StatementCondition::Or(out))
                }
            }
            UnlinkedStatementCondition::Not(x) => {
                Some(StatementCondition::Not(Box::new(self.condition(*x, d)?)))
            }
        }
    }
}
/// Excludes collection and reference types from the supported comparison operands.
fn scalar(t: &CanonicalType) -> bool {
    !matches!(
        t,
        CanonicalType::List(_)
            | CanonicalType::Set(_)
            | CanonicalType::Map { .. }
            | CanonicalType::Reference(_)
    )
}

/// Produces a source-position-independent key for deterministic guard ordering.
fn condition_key(c: &StatementCondition) -> String {
    semantic_key(c)
}
/// Serializes semantic content after recursively dropping `span` fields so source
/// relocation cannot change canonical ordering. Panics if serialization fails.
fn semantic_key(c: &impl serde::Serialize) -> String {
    /// Removes source ranges from objects and their nested arrays before comparison.
    fn clean(v: &mut serde_json::Value) {
        match v {
            serde_json::Value::Object(m) => {
                m.remove("span");
                for value in m.values_mut() {
                    clean(value);
                }
            }
            serde_json::Value::Array(xs) => {
                for x in xs {
                    clean(x);
                }
            }
            _ => {}
        }
    }
    let mut value = serde_json::to_value(c).expect("statement serializes");
    clean(&mut value);
    value.to_string()
}
