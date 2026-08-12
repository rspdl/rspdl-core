use rspdl_domain::{
    Atom, BooleanExpression, CanonicalId, CanonicalType, CanonicalValue, Domain, EnumType,
    PolicyBranch, PolicyEffect, SolveOptions, Term, TotalDecisionPoint, Variable, VariableDomain,
    analyze_total_decision_point,
};
use rspdl_solver_z3::Z3Solver;

fn id(value: &str) -> CanonicalId {
    CanonicalId::new(value).unwrap()
}

fn status() -> EnumType {
    EnumType::new(
        id("project_status"),
        [id("active"), id("ended"), id("paused"), id("scheduled")],
    )
    .unwrap()
}

fn variable(kind: &EnumType) -> VariableDomain {
    let values = kind
        .variants()
        .iter()
        .cloned()
        .map(|variant| CanonicalValue::enum_variant(kind.clone(), variant).unwrap());
    VariableDomain::new(
        id("status"),
        Domain::finite(CanonicalType::Enum(kind.clone()), values).unwrap(),
    )
}

fn when(kind: &EnumType, variant: &str) -> BooleanExpression {
    BooleanExpression::atom(
        Atom::equal(
            Term::Variable(Variable::new(
                id("status"),
                CanonicalType::Enum(kind.clone()),
            )),
            Term::Constant(CanonicalValue::enum_variant(kind.clone(), id(variant)).unwrap()),
        )
        .unwrap(),
    )
}

fn branch(kind: &EnumType, branch_id: &str, variant: &str, effect: PolicyEffect) -> PolicyBranch {
    PolicyBranch::new(id(branch_id), when(kind, variant), effect)
}

fn analyze(point: &TotalDecisionPoint) -> rspdl_domain::TotalDecisionAnalysis {
    analyze_total_decision_point(point, &Z3Solver::new(), SolveOptions::default()).unwrap()
}

#[test]
fn complete_closed_enum_has_no_gap_or_pair_finding() {
    let kind = status();
    let branches = kind.variants().iter().map(|variant| {
        branch(
            &kind,
            variant.as_str(),
            variant.as_str(),
            PolicyEffect::Allow,
        )
    });
    let point = TotalDecisionPoint::new(variable(&kind), branches).unwrap();

    let analysis = analyze(&point);

    assert!(analysis.coverage_is_exact());
    assert!(analysis.gaps().is_empty());
    assert!(analysis.compatible_overlaps().is_empty());
    assert!(analysis.conflicts().is_empty());
    assert!(analysis.unknowns().is_empty());
}

#[test]
fn gap_lists_every_uncovered_variant_in_canonical_order_with_witnesses() {
    let kind = status();
    let point = TotalDecisionPoint::new(
        variable(&kind),
        [branch(
            &kind,
            "active_policy",
            "active",
            PolicyEffect::Allow,
        )],
    )
    .unwrap();

    let analysis = analyze(&point);

    assert!(analysis.coverage_is_exact());
    assert_eq!(
        analysis
            .gaps()
            .iter()
            .map(|gap| gap.variant().as_str())
            .collect::<Vec<_>>(),
        ["ended", "paused", "scheduled"]
    );
    for gap in analysis.gaps() {
        assert_eq!(
            gap.witness()
                .0
                .get(&id("status"))
                .and_then(CanonicalValue::as_enum_variant),
            Some(gap.variant())
        );
    }
}

#[test]
fn same_effect_overlap_is_not_a_conflict() {
    let kind = status();
    let point = TotalDecisionPoint::new(
        variable(&kind),
        [
            branch(&kind, "active_first", "active", PolicyEffect::Allow),
            branch(&kind, "active_second", "active", PolicyEffect::Allow),
        ],
    )
    .unwrap();

    let analysis = analyze(&point);

    assert_eq!(analysis.compatible_overlaps().len(), 1);
    assert!(analysis.conflicts().is_empty());
}

#[test]
fn incompatible_allow_and_deny_overlap_is_a_conflict() {
    let kind = status();
    let point = TotalDecisionPoint::new(
        variable(&kind),
        [
            branch(&kind, "active_allow", "active", PolicyEffect::Allow),
            branch(&kind, "active_deny", "active", PolicyEffect::Deny),
        ],
    )
    .unwrap();

    let analysis = analyze(&point);

    assert_eq!(analysis.conflicts().len(), 1);
    assert!(analysis.compatible_overlaps().is_empty());
}

#[test]
fn distinct_enum_conditions_do_not_create_a_false_positive_pair_finding() {
    let kind = status();
    let point = TotalDecisionPoint::new(
        variable(&kind),
        [
            branch(&kind, "active_allow", "active", PolicyEffect::Allow),
            branch(&kind, "ended_deny", "ended", PolicyEffect::Deny),
        ],
    )
    .unwrap();

    let analysis = analyze(&point);

    assert!(analysis.compatible_overlaps().is_empty());
    assert!(analysis.conflicts().is_empty());
}

#[test]
fn findings_are_independent_of_input_branch_order() {
    let kind = status();
    let branches = [
        branch(&kind, "active_allow", "active", PolicyEffect::Allow),
        branch(&kind, "active_deny", "active", PolicyEffect::Deny),
        branch(&kind, "ended_allow", "ended", PolicyEffect::Allow),
    ];
    let forward = TotalDecisionPoint::new(variable(&kind), branches.clone()).unwrap();
    let reverse = TotalDecisionPoint::new(variable(&kind), branches.into_iter().rev()).unwrap();

    assert_eq!(analyze(&forward), analyze(&reverse));
}
