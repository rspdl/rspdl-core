//! Bounded model finding for finite entity scopes and unary/binary relations.
//!
//! Quantified relational constraints are grounded into the shared typed
//! Boolean IR. `UNSAT` therefore means unsatisfiable only within the requested
//! per-model scope; it is never promoted to an unbounded theorem.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::{
    Atom, BooleanExpression, CanonicalId, CanonicalType, CanonicalValue, ComparisonOperator,
    ConstraintDefinition, ConstraintOperand, ConstraintProblem, ConstraintSolver, Domain,
    FieldDefinition, RelationDefinition, RelationOperator, RelationalConstraintDefinition,
    RelationalConstraintKind, SemanticModule, SolveOptions, SolveResult, SolverContractError, Term,
    Variable, VariableDomain,
};

/// Implementation safety limit for eager quantifier grounding. This is not a
/// semantic claim about the maximum size of a product data world.
pub const MAX_BOUNDED_SCOPE_PER_MODEL: usize = 32;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BoundedModelOptions {
    scope_per_model: usize,
    solve_options: SolveOptions,
}

impl BoundedModelOptions {
    pub fn new(
        scope_per_model: usize,
        solve_options: SolveOptions,
    ) -> Result<Self, BoundedModelConfigurationError> {
        if scope_per_model == 0 {
            Err(BoundedModelConfigurationError::ZeroScope)
        } else if scope_per_model > MAX_BOUNDED_SCOPE_PER_MODEL {
            Err(BoundedModelConfigurationError::ScopeTooLarge {
                requested: scope_per_model,
                maximum: MAX_BOUNDED_SCOPE_PER_MODEL,
            })
        } else {
            Ok(Self {
                scope_per_model,
                solve_options,
            })
        }
    }

    pub const fn scope_per_model(self) -> usize {
        self.scope_per_model
    }

    pub const fn solve_options(self) -> SolveOptions {
        self.solve_options
    }
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum BoundedModelConfigurationError {
    #[error("bounded model scope must be greater than zero")]
    ZeroScope,
    #[error("bounded model scope `{requested}` exceeds implementation maximum `{maximum}`")]
    ScopeTooLarge { requested: usize, maximum: usize },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct VirtualEntity {
    pub model_id: CanonicalId,
    pub entity_id: CanonicalId,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct VirtualRelationTuple {
    pub relation_id: CanonicalId,
    pub entity_ids: Vec<CanonicalId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct VirtualFieldValue {
    pub entity_id: CanonicalId,
    pub field_id: CanonicalId,
    pub value: CanonicalValue,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RelationalWitness {
    pub entities: Vec<VirtualEntity>,
    pub field_values: Vec<VirtualFieldValue>,
    pub relation_tuples: Vec<VirtualRelationTuple>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum BoundedModelResult {
    Sat {
        scope_per_model: usize,
        witness: RelationalWitness,
    },
    UnsatWithinBound {
        scope_per_model: usize,
        /// A deterministic deletion-minimal set of declared rules that remains
        /// inconsistent with the built-in endpoint-integrity axioms.
        core_rule_ids: Vec<CanonicalId>,
    },
    Unknown {
        scope_per_model: usize,
        rule_id: String,
        message_key: String,
        reason: String,
    },
    Unsupported {
        scope_per_model: usize,
        rule_id: String,
        message_key: String,
        constructs: Vec<String>,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum RelationalAnalysisError<E: std::error::Error + 'static> {
    #[error("invalid grounded solver problem: {0}")]
    Contract(#[from] SolverContractError),
    #[error("relational model solver failed: {0}")]
    Solver(E),
}

#[derive(Clone)]
struct GroundedRule {
    id: CanonicalId,
    assertion: BooleanExpression,
}

struct Grounding {
    variables: Vec<VariableDomain>,
    base_assertion: BooleanExpression,
    rules: Vec<GroundedRule>,
    relation_signatures: BTreeMap<CanonicalId, Vec<CanonicalId>>,
    field_owners: BTreeMap<CanonicalId, (CanonicalId, bool)>,
    existence_variables: BTreeMap<(CanonicalId, usize), CanonicalId>,
    field_variables: BTreeMap<(CanonicalId, usize), CanonicalId>,
    field_presence_variables: BTreeMap<(CanonicalId, usize), CanonicalId>,
    tuple_variables: BTreeMap<(CanonicalId, Vec<usize>), CanonicalId>,
}

/// Finds one finite virtual data world, or proves that none exists inside the
/// requested bound. No runtime record data is consumed.
pub fn find_bounded_relational_model<S: ConstraintSolver>(
    module: &SemanticModule,
    solver: &S,
    options: BoundedModelOptions,
) -> Result<BoundedModelResult, RelationalAnalysisError<S::Error>> {
    let mut unsupported = Vec::new();
    if !module.derivations.is_empty() {
        unsupported.push("derivation".to_owned());
    }
    unsupported.extend(module.models.iter().flat_map(|model| {
        model
            .fields
            .iter()
            .filter(|field| matches!(field.value_type, CanonicalType::Refinement(_)))
            .map(|field| format!("refinement_field:{}", field.id))
    }));
    unsupported.sort();
    unsupported.dedup();
    if !unsupported.is_empty() {
        return Ok(BoundedModelResult::Unsupported {
            scope_per_model: options.scope_per_model,
            rule_id: "RSPDL-MODEL-003".into(),
            message_key: "model_finding.unsupported_construct".into(),
            constructs: unsupported,
        });
    }
    let grounding = Grounding::new(module, options.scope_per_model);
    let mut active = vec![true; grounding.rules.len()];
    match solve_grounding(&grounding, &active, solver, options.solve_options)? {
        SolveResult::Sat(model) => match grounding.witness(&model) {
            Ok(witness) => Ok(BoundedModelResult::Sat {
                scope_per_model: options.scope_per_model,
                witness,
            }),
            Err(reason) => Ok(BoundedModelResult::Unknown {
                scope_per_model: options.scope_per_model,
                rule_id: "RSPDL-MODEL-004".into(),
                message_key: "model_finding.unknown".into(),
                reason,
            }),
        },
        SolveResult::Unknown { reason } => Ok(BoundedModelResult::Unknown {
            scope_per_model: options.scope_per_model,
            rule_id: "RSPDL-MODEL-004".into(),
            message_key: "model_finding.unknown".into(),
            reason,
        }),
        SolveResult::Unsat => {
            // Stable IDs define both the deletion order and reported evidence.
            // A rule is removed when the remaining theory is still UNSAT.
            for index in 0..active.len() {
                active[index] = false;
                match solve_grounding(&grounding, &active, solver, options.solve_options)? {
                    SolveResult::Unsat => {}
                    SolveResult::Sat(_) | SolveResult::Unknown { .. } => active[index] = true,
                }
            }
            Ok(BoundedModelResult::UnsatWithinBound {
                scope_per_model: options.scope_per_model,
                core_rule_ids: grounding
                    .rules
                    .iter()
                    .zip(active)
                    .filter(|(_, active)| *active)
                    .map(|(rule, _)| rule.id.clone())
                    .collect(),
            })
        }
    }
}

fn solve_grounding<S: ConstraintSolver>(
    grounding: &Grounding,
    active: &[bool],
    solver: &S,
    options: SolveOptions,
) -> Result<SolveResult, RelationalAnalysisError<S::Error>> {
    let assertion = BooleanExpression::and(
        [grounding.base_assertion.clone()].into_iter().chain(
            grounding
                .rules
                .iter()
                .zip(active)
                .filter(|(_, active)| **active)
                .map(|(rule, _)| rule.assertion.clone()),
        ),
    );
    let problem = ConstraintProblem::new(grounding.variables.clone(), assertion)?;
    solver
        .solve(&problem, options)
        .map_err(RelationalAnalysisError::Solver)
}

impl Grounding {
    fn new(module: &SemanticModule, scope: usize) -> Self {
        let existence_variables = module
            .models
            .iter()
            .flat_map(|model| {
                (0..scope).map(move |index| {
                    (
                        (model.id.clone(), index),
                        internal_id(&model.id, &format!("bmf_exists_{index}")),
                    )
                })
            })
            .collect::<BTreeMap<_, _>>();

        let tuple_variables = module
            .relations
            .iter()
            .flat_map(|relation| {
                tuple_indices(relation.parameter_model_ids.len(), scope)
                    .into_iter()
                    .map(move |indices| {
                        let suffix = indices
                            .iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                            .join("_");
                        (
                            (relation.id.clone(), indices),
                            internal_id(&relation.id, &format!("bmf_tuple_{suffix}")),
                        )
                    })
            })
            .collect::<BTreeMap<_, _>>();

        let field_variables = module
            .models
            .iter()
            .flat_map(|model| {
                model.fields.iter().flat_map(move |field| {
                    (0..scope).map(move |index| {
                        (
                            (field.id.clone(), index),
                            internal_id(&field.id, &format!("bmf_value_{index}")),
                        )
                    })
                })
            })
            .collect::<BTreeMap<_, _>>();
        let field_presence_variables = module
            .models
            .iter()
            .flat_map(|model| {
                model
                    .fields
                    .iter()
                    .filter(|field| !field.required)
                    .flat_map(move |field| {
                        (0..scope).map(move |index| {
                            (
                                (field.id.clone(), index),
                                internal_id(&field.id, &format!("bmf_present_{index}")),
                            )
                        })
                    })
            })
            .collect::<BTreeMap<_, _>>();

        let boolean_domain = Domain::finite(
            CanonicalType::Boolean,
            [
                CanonicalValue::boolean(false),
                CanonicalValue::boolean(true),
            ],
        )
        .expect("the Boolean domain is well typed");
        let fields = module
            .models
            .iter()
            .flat_map(|model| model.fields.iter())
            .map(|field| (field.id.clone(), field))
            .collect::<BTreeMap<_, _>>();
        let mut variable_domains = BTreeMap::new();
        for id in existence_variables
            .values()
            .chain(field_presence_variables.values())
            .chain(tuple_variables.values())
        {
            variable_domains.insert(id.clone(), boolean_domain.clone());
        }
        for ((field_id, _), variable_id) in &field_variables {
            variable_domains.insert(
                variable_id.clone(),
                domain_for_type(&fields[field_id].value_type)
                    .expect("refinement fields are rejected before grounding"),
            );
        }
        let variables = variable_domains
            .into_iter()
            .map(|(id, domain)| VariableDomain::new(id, domain))
            .collect();

        let relations = module
            .relations
            .iter()
            .map(|relation| (relation.id.clone(), relation))
            .collect::<BTreeMap<_, _>>();
        let relation_signatures = module
            .relations
            .iter()
            .map(|relation| (relation.id.clone(), relation.parameter_model_ids.clone()))
            .collect();
        let base_assertion = BooleanExpression::and([
            endpoint_integrity(&relations, &existence_variables, &tuple_variables),
            field_presence_integrity(
                module,
                &existence_variables,
                &field_presence_variables,
                scope,
            ),
        ]);
        let mut rules = module
            .relational_constraints
            .iter()
            .map(|definition| GroundedRule {
                id: definition.id.clone(),
                assertion: ground_rule(
                    definition,
                    &relations,
                    &existence_variables,
                    &tuple_variables,
                    scope,
                ),
            })
            .collect::<Vec<_>>();
        rules.extend(module.constraints.iter().map(|definition| GroundedRule {
            id: definition.id.clone(),
            assertion: ground_field_constraint(
                definition,
                module,
                &existence_variables,
                &field_variables,
                &field_presence_variables,
                scope,
            ),
        }));
        rules.sort_by(|left, right| left.id.cmp(&right.id));

        let field_owners = module
            .models
            .iter()
            .flat_map(|model| {
                model
                    .fields
                    .iter()
                    .map(move |field| (field.id.clone(), (model.id.clone(), field.required)))
            })
            .collect();

        Self {
            variables,
            base_assertion,
            rules,
            relation_signatures,
            field_owners,
            existence_variables,
            field_variables,
            field_presence_variables,
            tuple_variables,
        }
    }

    fn witness(&self, model: &crate::CanonicalModel) -> Result<RelationalWitness, String> {
        let entities = self
            .existence_variables
            .iter()
            .filter(|(_, variable)| is_true(model, variable))
            .map(|((model_id, index), _)| VirtualEntity {
                model_id: model_id.clone(),
                entity_id: virtual_entity_id(model_id, *index),
            })
            .collect();
        let mut field_values = Vec::new();
        for ((field_id, index), variable) in &self.field_variables {
            let (model_id, required) = &self.field_owners[field_id];
            if !is_true(
                model,
                &self.existence_variables[&(model_id.clone(), *index)],
            ) {
                continue;
            }
            if !required
                && !is_true(
                    model,
                    &self.field_presence_variables[&(field_id.clone(), *index)],
                )
            {
                continue;
            }
            let value = model.0.get(variable).ok_or_else(|| {
                format!("solver model omitted assignment for present field `{field_id}`")
            })?;
            field_values.push(VirtualFieldValue {
                entity_id: virtual_entity_id(model_id, *index),
                field_id: field_id.clone(),
                value: value.clone(),
            });
        }
        let relation_tuples = self
            .tuple_variables
            .iter()
            .filter(|(_, variable)| is_true(model, variable))
            .map(|((relation_id, indices), _)| {
                let model_ids = &self.relation_signatures[relation_id];
                VirtualRelationTuple {
                    relation_id: relation_id.clone(),
                    entity_ids: model_ids
                        .iter()
                        .zip(indices)
                        .map(|(model_id, index)| virtual_entity_id(model_id, *index))
                        .collect(),
                }
            })
            .collect();
        Ok(RelationalWitness {
            entities,
            field_values,
            relation_tuples,
        })
    }
}

fn field_presence_integrity(
    module: &SemanticModule,
    existence: &BTreeMap<(CanonicalId, usize), CanonicalId>,
    presence: &BTreeMap<(CanonicalId, usize), CanonicalId>,
    scope: usize,
) -> BooleanExpression {
    BooleanExpression::and(module.models.iter().flat_map(|model| {
        model
            .fields
            .iter()
            .filter(|field| !field.required)
            .flat_map(move |field| {
                (0..scope).map(move |index| {
                    implies(
                        truth(&presence[&(field.id.clone(), index)]),
                        truth(&existence[&(model.id.clone(), index)]),
                    )
                })
            })
    }))
}

fn ground_field_constraint(
    definition: &ConstraintDefinition,
    module: &SemanticModule,
    existence: &BTreeMap<(CanonicalId, usize), CanonicalId>,
    field_variables: &BTreeMap<(CanonicalId, usize), CanonicalId>,
    presence: &BTreeMap<(CanonicalId, usize), CanonicalId>,
    scope: usize,
) -> BooleanExpression {
    let model = module
        .models
        .iter()
        .find(|model| model.id == definition.model_id)
        .expect("linked constraint model exists");
    let fields = model
        .fields
        .iter()
        .map(|field| (field.id.clone(), field))
        .collect::<BTreeMap<_, _>>();
    BooleanExpression::and((0..scope).map(|index| {
        let left = constraint_term(&definition.left, index, &fields, field_variables);
        let right = constraint_term(&definition.right, index, &fields, field_variables);
        let assertion = match definition.operator {
            RelationOperator::Equal => BooleanExpression::atom(
                Atom::equal(left, right).expect("linked equality operands have the same type"),
            ),
            RelationOperator::NotEqual => BooleanExpression::negate(BooleanExpression::atom(
                Atom::equal(left, right).expect("linked equality operands have the same type"),
            )),
            RelationOperator::LessThan => integer_comparison(ComparisonOperator::Lt, left, right),
            RelationOperator::LessThanOrEqual => {
                integer_comparison(ComparisonOperator::Le, left, right)
            }
            RelationOperator::GreaterThan => {
                integer_comparison(ComparisonOperator::Gt, left, right)
            }
            RelationOperator::GreaterThanOrEqual => {
                integer_comparison(ComparisonOperator::Ge, left, right)
            }
        };
        let optional_guards = [&definition.left, &definition.right]
            .into_iter()
            .filter_map(|operand| match operand {
                ConstraintOperand::Field(field_id) if !fields[field_id].required => {
                    Some(truth(&presence[&(field_id.clone(), index)]))
                }
                _ => None,
            });
        implies(
            BooleanExpression::and(
                [truth(&existence[&(model.id.clone(), index)])]
                    .into_iter()
                    .chain(optional_guards),
            ),
            assertion,
        )
    }))
}

fn constraint_term(
    operand: &ConstraintOperand,
    index: usize,
    fields: &BTreeMap<CanonicalId, &FieldDefinition>,
    variables: &BTreeMap<(CanonicalId, usize), CanonicalId>,
) -> Term {
    match operand {
        ConstraintOperand::Field(field_id) => Term::Variable(Variable::new(
            variables[&(field_id.clone(), index)].clone(),
            fields[field_id].value_type.clone(),
        )),
        ConstraintOperand::Constant(value) => Term::Constant(value.clone()),
    }
}

fn integer_comparison(operator: ComparisonOperator, left: Term, right: Term) -> BooleanExpression {
    BooleanExpression::atom(
        Atom::integer_comparison(operator, left, right)
            .expect("linked ordered constraint operands are integers"),
    )
}

fn domain_for_type(value_type: &CanonicalType) -> Option<Domain> {
    Some(match value_type {
        CanonicalType::Boolean => Domain::finite(
            CanonicalType::Boolean,
            [
                CanonicalValue::boolean(false),
                CanonicalValue::boolean(true),
            ],
        )
        .expect("Boolean values are well typed"),
        CanonicalType::Integer => Domain::integers(),
        CanonicalType::String => Domain::strings(),
        CanonicalType::Enum(enum_type) => Domain::finite(
            value_type.clone(),
            enum_type.variants().iter().cloned().map(|variant| {
                CanonicalValue::enum_variant(enum_type.clone(), variant)
                    .expect("declared enum variant is valid")
            }),
        )
        .expect("enum values are well typed"),
        CanonicalType::Refinement(_) => return None,
    })
}

fn endpoint_integrity(
    relations: &BTreeMap<CanonicalId, &RelationDefinition>,
    existence: &BTreeMap<(CanonicalId, usize), CanonicalId>,
    tuples: &BTreeMap<(CanonicalId, Vec<usize>), CanonicalId>,
) -> BooleanExpression {
    BooleanExpression::and(tuples.iter().map(|((relation_id, indices), tuple)| {
        let relation = relations
            .get(relation_id)
            .expect("linked relation definition exists");
        let endpoints = relation
            .parameter_model_ids
            .iter()
            .zip(indices)
            .map(|(model_id, index)| truth(&existence[&(model_id.clone(), *index)]));
        implies(truth(tuple), BooleanExpression::and(endpoints))
    }))
}

fn ground_rule(
    definition: &RelationalConstraintDefinition,
    relations: &BTreeMap<CanonicalId, &RelationDefinition>,
    existence: &BTreeMap<(CanonicalId, usize), CanonicalId>,
    tuples: &BTreeMap<(CanonicalId, Vec<usize>), CanonicalId>,
    scope: usize,
) -> BooleanExpression {
    match &definition.constraint {
        RelationalConstraintKind::NonEmpty { model_id } => BooleanExpression::or(
            (0..scope).map(|index| truth(&existence[&(model_id.clone(), index)])),
        ),
        RelationalConstraintKind::Required { relation_id } => {
            let relation = relations[relation_id];
            let source_model = &relation.parameter_model_ids[0];
            BooleanExpression::and((0..scope).map(|source| {
                let related = tuples
                    .iter()
                    .filter(|((id, indices), _)| id == relation_id && indices[0] == source)
                    .map(|(_, variable)| truth(variable));
                implies(
                    truth(&existence[&(source_model.clone(), source)]),
                    BooleanExpression::or(related),
                )
            }))
        }
        RelationalConstraintKind::Unique { relation_id } => {
            BooleanExpression::and((0..scope).flat_map(|source| {
                let candidates = tuples
                    .iter()
                    .filter(|((id, indices), _)| id == relation_id && indices[0] == source)
                    .map(|(_, variable)| variable)
                    .collect::<Vec<_>>();
                pairwise_exclusive(candidates)
            }))
        }
        RelationalConstraintKind::Exclusive { relation_ids } => {
            let arity = relations[&relation_ids[0]].parameter_model_ids.len();
            BooleanExpression::and(tuple_indices(arity, scope).into_iter().flat_map(|indices| {
                let candidates = relation_ids
                    .iter()
                    .map(|id| &tuples[&(id.clone(), indices.clone())])
                    .collect::<Vec<_>>();
                pairwise_exclusive(candidates)
            }))
        }
        RelationalConstraintKind::Exhaustive { relation_ids } => {
            let signature = &relations[&relation_ids[0]].parameter_model_ids;
            BooleanExpression::and(tuple_indices(signature.len(), scope).into_iter().map(
                |indices| {
                    let endpoints = signature
                        .iter()
                        .zip(&indices)
                        .map(|(model_id, index)| truth(&existence[&(model_id.clone(), *index)]));
                    let classified = relation_ids
                        .iter()
                        .map(|id| truth(&tuples[&(id.clone(), indices.clone())]));
                    implies(
                        BooleanExpression::and(endpoints),
                        BooleanExpression::or(classified),
                    )
                },
            ))
        }
        RelationalConstraintKind::Coexistent { .. } => BooleanExpression::literal(true),
    }
}

fn pairwise_exclusive(variables: Vec<&CanonicalId>) -> Vec<BooleanExpression> {
    variables
        .iter()
        .enumerate()
        .flat_map(|(index, left)| {
            variables.iter().skip(index + 1).map(move |right| {
                BooleanExpression::negate(BooleanExpression::and([truth(left), truth(right)]))
            })
        })
        .collect()
}

fn tuple_indices(arity: usize, scope: usize) -> Vec<Vec<usize>> {
    fn visit(out: &mut Vec<Vec<usize>>, current: &mut Vec<usize>, arity: usize, scope: usize) {
        if current.len() == arity {
            out.push(current.clone());
            return;
        }
        for index in 0..scope {
            current.push(index);
            visit(out, current, arity, scope);
            current.pop();
        }
    }
    let mut out = Vec::new();
    visit(&mut out, &mut Vec::new(), arity, scope);
    out
}

fn truth(id: &CanonicalId) -> BooleanExpression {
    BooleanExpression::atom(
        Atom::equal(
            Term::Variable(Variable::new(id.clone(), CanonicalType::Boolean)),
            Term::Constant(CanonicalValue::boolean(true)),
        )
        .expect("Boolean variables compare with Boolean constants"),
    )
}

fn implies(left: BooleanExpression, right: BooleanExpression) -> BooleanExpression {
    BooleanExpression::or([BooleanExpression::negate(left), right])
}

fn internal_id(owner: &CanonicalId, suffix: &str) -> CanonicalId {
    CanonicalId::new(format!("{owner}.{suffix}"))
        .expect("canonical owner and internal suffix form a canonical ID")
}

fn virtual_entity_id(model_id: &CanonicalId, index: usize) -> CanonicalId {
    internal_id(model_id, &format!("virtual_{index}"))
}

fn is_true(model: &crate::CanonicalModel, variable: &CanonicalId) -> bool {
    model.0.get(variable).and_then(CanonicalValue::as_boolean) == Some(true)
}
