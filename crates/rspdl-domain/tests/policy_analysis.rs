use std::{collections::BTreeMap, fmt};

use rspdl_domain::{
    Atom, BooleanExpression, CanonicalId, CanonicalModel, CanonicalType, CanonicalValue,
    ConstraintProblem, ConstraintSolver, DecisionPointError, Domain, EnumType, PolicyBranch,
    PolicyEffect, SolveOptions, SolveResult, Term, TotalDecisionPoint, Variable, VariableDomain,
    analyze_total_decision_point,
};

fn id(value: &str) -> CanonicalId {
    CanonicalId::new(value).unwrap()
}

fn status() -> EnumType {
    EnumType::new(id("status"), [id("active"), id("ended")]).unwrap()
}

fn variable(kind: EnumType) -> VariableDomain {
    let values = kind
        .variants()
        .iter()
        .cloned()
        .map(|variant| CanonicalValue::enum_variant(kind.clone(), variant).unwrap());
    VariableDomain::new(
        id("project_status"),
        Domain::finite(CanonicalType::Enum(kind.clone()), values).unwrap(),
    )
}

fn equals(kind: &EnumType, variant: &str) -> BooleanExpression {
    BooleanExpression::atom(
        Atom::equal(
            Term::Variable(Variable::new(
                id("project_status"),
                CanonicalType::Enum(kind.clone()),
            )),
            Term::Constant(CanonicalValue::enum_variant(kind.clone(), id(variant)).unwrap()),
        )
        .unwrap(),
    )
}

#[derive(Debug)]
struct StubError;

impl fmt::Display for StubError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("stub failure")
    }
}

impl std::error::Error for StubError {}

struct UnknownSolver;

impl ConstraintSolver for UnknownSolver {
    type Error = StubError;

    fn solve(
        &self,
        _problem: &ConstraintProblem,
        _options: SolveOptions,
    ) -> Result<SolveResult, Self::Error> {
        Ok(SolveResult::Unknown {
            reason: "timeout".into(),
        })
    }
}

struct FailingSolver;

impl ConstraintSolver for FailingSolver {
    type Error = StubError;

    fn solve(
        &self,
        _problem: &ConstraintProblem,
        _options: SolveOptions,
    ) -> Result<SolveResult, Self::Error> {
        Err(StubError)
    }
}

#[test]
fn constructor_rejects_non_enum_domains_and_duplicate_branch_ids() {
    let branch = PolicyBranch::new(
        id("active"),
        BooleanExpression::literal(true),
        PolicyEffect::Allow,
    );
    let error = TotalDecisionPoint::new(
        VariableDomain::new(id("value"), Domain::integers()),
        [branch.clone()],
    )
    .unwrap_err();
    assert!(matches!(error, DecisionPointError::NonEnumVariable { .. }));

    let error = TotalDecisionPoint::new(variable(status()), [branch.clone(), branch]).unwrap_err();
    assert!(
        matches!(error, DecisionPointError::DuplicateBranch(branch_id) if branch_id == id("active"))
    );
}

#[test]
fn constructor_rejects_a_subset_of_the_declared_enum() {
    let kind = status();
    let only_active = CanonicalValue::enum_variant(kind.clone(), id("active")).unwrap();
    let error = TotalDecisionPoint::new(
        VariableDomain::new(
            id("project_status"),
            Domain::finite(CanonicalType::Enum(kind), [only_active]).unwrap(),
        ),
        [],
    )
    .unwrap_err();
    assert!(matches!(
        error,
        DecisionPointError::NonClosedEnumDomain { .. }
    ));
}

#[test]
fn unknown_is_not_accepted_as_coverage_success() {
    let kind = status();
    let point = TotalDecisionPoint::new(
        variable(kind.clone()),
        [PolicyBranch::new(
            id("active"),
            equals(&kind, "active"),
            PolicyEffect::Allow,
        )],
    )
    .unwrap();
    let analysis =
        analyze_total_decision_point(&point, &UnknownSolver, SolveOptions::default()).unwrap();
    assert!(!analysis.coverage_is_exact());
    assert_eq!(analysis.unknowns().len(), 2);
    assert!(analysis.gaps().is_empty());
}

#[test]
fn solver_errors_are_not_converted_to_no_findings() {
    let point = TotalDecisionPoint::new(variable(status()), []).unwrap();
    assert!(analyze_total_decision_point(&point, &FailingSolver, SolveOptions::default()).is_err());
}

#[test]
fn canonical_model_is_serializable_for_finding_evidence() {
    let model = CanonicalModel(BTreeMap::from([(
        id("value"),
        CanonicalValue::boolean(true),
    )]));
    assert_eq!(
        serde_json::to_string(&model).unwrap(),
        r#"{"value":{"value_type":{"kind":"boolean"},"representation":{"kind":"boolean","value":true}}}"#
    );
}
