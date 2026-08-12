use rspdl_domain::{
    Atom, BooleanExpression, CanonicalId, CanonicalModel, CanonicalType, CanonicalValue, Domain,
    EnumType, PolicyBranch, PolicyEffect, SolveOptions, Term, TotalDecisionPoint, Variable,
    VariableDomain, analyze_total_decision_point,
};
use rspdl_solver_z3::Z3Solver;

fn id(value: &str) -> CanonicalId {
    CanonicalId::new(value).expect("example IDs are canonical")
}

fn project_status() -> EnumType {
    EnumType::new(
        id("project_status"),
        [id("active"), id("ended"), id("paused"), id("scheduled")],
    )
    .expect("the example enum is valid")
}

fn status_domain(kind: &EnumType) -> VariableDomain {
    let values = kind.variants().iter().cloned().map(|variant| {
        CanonicalValue::enum_variant(kind.clone(), variant)
            .expect("declared variants construct canonical values")
    });
    VariableDomain::new(
        id("status"),
        Domain::finite(CanonicalType::Enum(kind.clone()), values)
            .expect("the complete enum is a finite domain"),
    )
}

fn when_status_is(kind: &EnumType, variant: &str) -> BooleanExpression {
    BooleanExpression::atom(
        Atom::equal(
            Term::Variable(Variable::new(
                id("status"),
                CanonicalType::Enum(kind.clone()),
            )),
            Term::Constant(
                CanonicalValue::enum_variant(kind.clone(), id(variant))
                    .expect("the branch uses a declared variant"),
            ),
        )
        .expect("the condition compares values of the same enum type"),
    )
}

fn branch(kind: &EnumType, branch_id: &str, variant: &str, effect: PolicyEffect) -> PolicyBranch {
    PolicyBranch::new(id(branch_id), when_status_is(kind, variant), effect)
}

fn witness_status(witness: &CanonicalModel) -> &CanonicalId {
    witness
        .0
        .get(&id("status"))
        .and_then(CanonicalValue::as_enum_variant)
        .expect("every finding has a status witness")
}

fn main() {
    let status = project_status();
    let solver = Z3Solver::new();

    let gap_point = TotalDecisionPoint::new(
        status_domain(&status),
        [branch(
            &status,
            "active_policy",
            "active",
            PolicyEffect::Allow,
        )],
    )
    .expect("the gap example is a valid decision point");
    let gap_analysis = analyze_total_decision_point(&gap_point, &solver, SolveOptions::default())
        .expect("Z3 should analyze the gap example");
    for gap in gap_analysis.gaps() {
        println!(
            "GAP {}: status={}",
            gap.variant(),
            witness_status(gap.witness())
        );
    }

    let conflict_point = TotalDecisionPoint::new(
        status_domain(&status),
        [
            branch(&status, "active_allow", "active", PolicyEffect::Allow),
            branch(&status, "active_deny", "active", PolicyEffect::Deny),
        ],
    )
    .expect("the conflict example is a valid decision point");
    let conflict_analysis =
        analyze_total_decision_point(&conflict_point, &solver, SolveOptions::default())
            .expect("Z3 should analyze the conflict example");
    for conflict in conflict_analysis.conflicts() {
        println!(
            "CONFLICT {} + {}: status={}",
            conflict.branch_ids()[0],
            conflict.branch_ids()[1],
            witness_status(conflict.witness())
        );
    }

    let overlap_point = TotalDecisionPoint::new(
        status_domain(&status),
        [
            branch(&status, "active_first", "active", PolicyEffect::Allow),
            branch(&status, "active_second", "active", PolicyEffect::Allow),
        ],
    )
    .expect("the overlap example is a valid decision point");
    let overlap_analysis =
        analyze_total_decision_point(&overlap_point, &solver, SolveOptions::default())
            .expect("Z3 should analyze the overlap example");
    for overlap in overlap_analysis.compatible_overlaps() {
        println!(
            "COMPATIBLE_OVERLAP {} + {}: status={}",
            overlap.branch_ids()[0],
            overlap.branch_ids()[1],
            witness_status(overlap.witness())
        );
    }
}
