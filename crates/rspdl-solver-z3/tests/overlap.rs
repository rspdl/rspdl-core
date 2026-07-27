use rspdl_core::*;
use rspdl_solver_z3::Z3Solver;
fn id(x: &str) -> CanonicalId {
    CanonicalId::new(x).unwrap()
}
#[test]
fn accounting_overlap_is_sat_and_exclusive_is_unsat() {
    let applicant = Variable::new(id("applicant"), CanonicalType::String);
    let approver = Variable::new(id("approver"), CanonicalType::String);
    let role = Variable::new(id("role"), CanonicalType::String);
    let manager = CanonicalValue::string("accounting_manager");
    let permit = BooleanExpression::atom(
        Atom::equal(
            Term::Variable(role.clone()),
            Term::Constant(manager.clone()),
        )
        .unwrap(),
    );
    let denial = BooleanExpression::atom(
        Atom::equal(
            Term::Variable(applicant.clone()),
            Term::Variable(approver.clone()),
        )
        .unwrap(),
    );
    let vars = vec![
        VariableDomain::new(id("applicant"), Domain::strings()),
        VariableDomain::new(id("approver"), Domain::strings()),
        VariableDomain::new(
            id("role"),
            Domain::finite(CanonicalType::String, [manager.clone()]).unwrap(),
        ),
    ];
    let sat = ConstraintProblem::from_overlap(vars.clone(), permit.clone(), denial.clone());
    assert!(matches!(
        Z3Solver::new()
            .solve(&sat, SolveOptions::default())
            .unwrap(),
        SolveResult::Sat(_)
    ));
    let no_self = BooleanExpression::and([permit, BooleanExpression::negate(denial.clone())]);
    let unsat = ConstraintProblem::from_overlap(vars, no_self, denial);
    assert!(matches!(
        Z3Solver::new()
            .solve(&unsat, SolveOptions::default())
            .unwrap(),
        SolveResult::Unsat
    ));
}
