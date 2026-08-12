use std::collections::{BTreeMap, VecDeque};
use std::convert::Infallible;
use std::sync::Mutex;

use rspdl_domain::{
    BoundedModelConfigurationError, BoundedModelOptions, BoundedModelResult, CanonicalId,
    CanonicalModel, CanonicalType, CanonicalValue, ConstraintProblem, ConstraintSolver,
    DataModelDefinition, DerivationDefinition, DerivationExpression, FieldDefinition,
    MAX_BOUNDED_SCOPE_PER_MODEL, RelationDefinition, RelationalConstraintDefinition,
    RelationalConstraintKind, SemanticModule, SolveOptions, SolveResult,
    find_bounded_relational_model,
};

struct UnknownSolver;

impl ConstraintSolver for UnknownSolver {
    type Error = Infallible;

    fn solve(
        &self,
        _problem: &ConstraintProblem,
        _options: SolveOptions,
    ) -> Result<SolveResult, Self::Error> {
        Ok(SolveResult::Unknown {
            reason: "test timeout".into(),
        })
    }
}

struct CompleteSatSolver;

impl ConstraintSolver for CompleteSatSolver {
    type Error = Infallible;

    fn solve(
        &self,
        problem: &ConstraintProblem,
        _options: SolveOptions,
    ) -> Result<SolveResult, Self::Error> {
        let assignments = problem
            .variables()
            .iter()
            .map(|variable| {
                let value = match variable.domain().value_type() {
                    CanonicalType::Boolean => CanonicalValue::boolean(true),
                    CanonicalType::Integer => CanonicalValue::integer(1),
                    CanonicalType::String => CanonicalValue::string("witness"),
                    CanonicalType::Enum(_) => variable
                        .domain()
                        .finite_values()
                        .and_then(|values| values.first())
                        .expect("enum domains are non-empty")
                        .clone(),
                    CanonicalType::Refinement(_) => CanonicalValue::prime(2).unwrap(),
                };
                (variable.id().clone(), value)
            })
            .collect();
        Ok(SolveResult::Sat(CanonicalModel(assignments)))
    }
}

struct MissingFieldAssignmentSolver;

impl ConstraintSolver for MissingFieldAssignmentSolver {
    type Error = Infallible;

    fn solve(
        &self,
        problem: &ConstraintProblem,
        _options: SolveOptions,
    ) -> Result<SolveResult, Self::Error> {
        let assignments = problem
            .variables()
            .iter()
            .filter(|variable| variable.domain().value_type() == &CanonicalType::Boolean)
            .map(|variable| (variable.id().clone(), CanonicalValue::boolean(true)))
            .collect();
        Ok(SolveResult::Sat(CanonicalModel(assignments)))
    }
}

struct ScriptedSolver {
    results: Mutex<VecDeque<SolveResult>>,
}

impl ScriptedSolver {
    fn new(results: impl IntoIterator<Item = SolveResult>) -> Self {
        Self {
            results: Mutex::new(results.into_iter().collect()),
        }
    }
}

impl ConstraintSolver for ScriptedSolver {
    type Error = Infallible;

    fn solve(
        &self,
        _problem: &ConstraintProblem,
        _options: SolveOptions,
    ) -> Result<SolveResult, Self::Error> {
        Ok(self
            .results
            .lock()
            .unwrap()
            .pop_front()
            .expect("test should provide one result per solve"))
    }
}

fn id(value: &str) -> CanonicalId {
    CanonicalId::new(value).unwrap()
}

fn model_with_field(
    model_id: &str,
    field_id: &str,
    value_type: CanonicalType,
) -> DataModelDefinition {
    DataModelDefinition {
        id: id(model_id),
        name: model_id.into(),
        fields: vec![FieldDefinition {
            id: id(field_id),
            local_id: id(field_id.rsplit('.').next().unwrap()),
            name: field_id.into(),
            required: true,
            value_type,
        }],
    }
}

fn empty_module() -> SemanticModule {
    SemanticModule {
        id: CanonicalId::new("test").unwrap(),
        name: "Test".into(),
        enums: Vec::new(),
        models: Vec::new(),
        relations: Vec::new(),
        relational_constraints: Vec::new(),
        screens: Vec::new(),
        derivations: Vec::new(),
        field_intents: Vec::new(),
        constraints: Vec::new(),
        roles: Vec::new(),
        actions: Vec::new(),
        policies: Vec::new(),
    }
}

#[test]
fn zero_scope_is_rejected_before_solving() {
    assert_eq!(
        BoundedModelOptions::new(0, SolveOptions::default()),
        Err(BoundedModelConfigurationError::ZeroScope)
    );
}

#[test]
fn eager_grounding_scope_has_a_structured_safety_limit() {
    assert!(BoundedModelOptions::new(1, SolveOptions::default()).is_ok());
    assert!(BoundedModelOptions::new(MAX_BOUNDED_SCOPE_PER_MODEL, SolveOptions::default()).is_ok());
    assert_eq!(
        BoundedModelOptions::new(MAX_BOUNDED_SCOPE_PER_MODEL + 1, SolveOptions::default()),
        Err(BoundedModelConfigurationError::ScopeTooLarge {
            requested: MAX_BOUNDED_SCOPE_PER_MODEL + 1,
            maximum: MAX_BOUNDED_SCOPE_PER_MODEL,
        })
    );
}

#[test]
fn unsupported_derivation_is_reported_before_solving() {
    let mut module = empty_module();
    module.models = vec![model_with_field(
        "test.item",
        "test.item.value",
        CanonicalType::Integer,
    )];
    module.derivations = vec![DerivationDefinition {
        target_field_id: id("test.item.value"),
        expression: DerivationExpression::Sum {
            source_field_id: id("test.item.value"),
        },
        recalculate_when_changed_field_ids: Vec::new(),
    }];

    let result = find_bounded_relational_model(
        &module,
        &UnknownSolver,
        BoundedModelOptions::new(1, SolveOptions::default()).unwrap(),
    )
    .unwrap();

    assert!(matches!(
        result,
        BoundedModelResult::Unsupported { constructs, .. } if constructs == ["derivation"]
    ));
}

#[test]
fn refinement_fields_are_not_approximated_by_another_domain() {
    let mut module = empty_module();
    module.models = vec![model_with_field(
        "test.item",
        "test.item.value",
        CanonicalType::prime(),
    )];

    let result = find_bounded_relational_model(
        &module,
        &UnknownSolver,
        BoundedModelOptions::new(1, SolveOptions::default()).unwrap(),
    )
    .unwrap();

    assert!(matches!(
        result,
        BoundedModelResult::Unsupported { constructs, .. }
            if constructs == ["refinement_field:test.item.value"]
    ));
}

#[test]
fn relational_rule_can_produce_a_virtual_tuple() {
    let mut module = empty_module();
    module.models = vec![
        model_with_field(
            "test.project",
            "test.project.active",
            CanonicalType::Boolean,
        ),
        model_with_field("test.user", "test.user.active", CanonicalType::Boolean),
    ];
    module.relations = vec![RelationDefinition {
        id: id("test.owner"),
        name: "owner".into(),
        parameter_model_ids: vec![id("test.project"), id("test.user")],
    }];
    module.relational_constraints = vec![
        RelationalConstraintDefinition {
            id: id("test.rule.nonempty"),
            constraint: RelationalConstraintKind::NonEmpty {
                model_id: id("test.project"),
            },
        },
        RelationalConstraintDefinition {
            id: id("test.rule.required"),
            constraint: RelationalConstraintKind::Required {
                relation_id: id("test.owner"),
            },
        },
    ];

    let result = find_bounded_relational_model(
        &module,
        &CompleteSatSolver,
        BoundedModelOptions::new(1, SolveOptions::default()).unwrap(),
    )
    .unwrap();
    let BoundedModelResult::Sat { witness, .. } = result else {
        panic!("expected SAT result: {result:?}");
    };

    assert_eq!(witness.relation_tuples.len(), 1);
    assert_eq!(witness.relation_tuples[0].relation_id, id("test.owner"));
}

#[test]
fn unsat_core_rule_ids_follow_deterministic_public_order() {
    let mut module = empty_module();
    module.models = vec![
        model_with_field("test.alpha", "test.alpha.value", CanonicalType::Boolean),
        model_with_field("test.zeta", "test.zeta.value", CanonicalType::Boolean),
    ];
    module.relational_constraints = vec![
        RelationalConstraintDefinition {
            id: id("test.rule.zeta"),
            constraint: RelationalConstraintKind::NonEmpty {
                model_id: id("test.zeta"),
            },
        },
        RelationalConstraintDefinition {
            id: id("test.rule.alpha"),
            constraint: RelationalConstraintKind::NonEmpty {
                model_id: id("test.alpha"),
            },
        },
    ];
    let solver = ScriptedSolver::new([
        SolveResult::Unsat,
        SolveResult::Sat(CanonicalModel(BTreeMap::new())),
        SolveResult::Sat(CanonicalModel(BTreeMap::new())),
    ]);

    let result = find_bounded_relational_model(
        &module,
        &solver,
        BoundedModelOptions::new(1, SolveOptions::default()).unwrap(),
    )
    .unwrap();

    assert!(matches!(
        result,
        BoundedModelResult::UnsatWithinBound { core_rule_ids, .. }
            if core_rule_ids == [id("test.rule.alpha"), id("test.rule.zeta")]
    ));
}

#[test]
fn missing_present_field_assignment_becomes_unknown() {
    let mut module = empty_module();
    module.models = vec![model_with_field(
        "test.item",
        "test.item.value",
        CanonicalType::Integer,
    )];
    module.relational_constraints = vec![RelationalConstraintDefinition {
        id: id("test.rule.nonempty"),
        constraint: RelationalConstraintKind::NonEmpty {
            model_id: id("test.item"),
        },
    }];

    let result = find_bounded_relational_model(
        &module,
        &MissingFieldAssignmentSolver,
        BoundedModelOptions::new(1, SolveOptions::default()).unwrap(),
    )
    .unwrap();

    assert!(matches!(
        result,
        BoundedModelResult::Unknown { reason, .. }
            if reason == "solver model omitted assignment for present field `test.item.value`"
    ));
}

#[test]
fn solver_unknown_is_not_accepted_as_a_virtual_model() {
    let options = BoundedModelOptions::new(1, SolveOptions::default()).unwrap();
    let result = find_bounded_relational_model(&empty_module(), &UnknownSolver, options).unwrap();

    assert_eq!(
        result,
        BoundedModelResult::Unknown {
            scope_per_model: 1,
            rule_id: "RSPDL-MODEL-004".into(),
            message_key: "model_finding.unknown".into(),
            reason: "test timeout".into(),
        }
    );
}
