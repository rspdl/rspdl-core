use std::convert::Infallible;

use rspdl_domain::{
    BoundedModelConfigurationError, BoundedModelOptions, BoundedModelResult, CanonicalId,
    ConstraintProblem, ConstraintSolver, MAX_BOUNDED_SCOPE_PER_MODEL, SemanticModule, SolveOptions,
    SolveResult, find_bounded_relational_model,
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
    assert_eq!(
        BoundedModelOptions::new(MAX_BOUNDED_SCOPE_PER_MODEL + 1, SolveOptions::default()),
        Err(BoundedModelConfigurationError::ScopeTooLarge {
            requested: MAX_BOUNDED_SCOPE_PER_MODEL + 1,
            maximum: MAX_BOUNDED_SCOPE_PER_MODEL,
        })
    );
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
