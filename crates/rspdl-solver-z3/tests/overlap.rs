use rspdl_domain::{
    Atom, BooleanExpression, CanonicalId, CanonicalModel, CanonicalType, CanonicalValue,
    ConstraintProblem, ConstraintSolver, Domain, EnumType, SolveOptions, SolveResult, Term,
    Variable, VariableDomain,
};
use rspdl_solver_z3::Z3Solver;
fn id(x: &str) -> CanonicalId {
    CanonicalId::new(x).unwrap()
}
#[test]
fn accounting_overlap_is_sat_and_exclusive_is_unsat() {
    let applicant = Variable::new(id("applicant"), CanonicalType::String);
    let approver = Variable::new(id("approver"), CanonicalType::String);
    let roles = EnumType::new(id("role_type"), [id("accounting_manager"), id("other")]).unwrap();
    let role = Variable::new(id("role"), CanonicalType::Enum(roles.clone()));
    let manager = CanonicalValue::enum_variant(roles.clone(), id("accounting_manager")).unwrap();
    let other = CanonicalValue::enum_variant(roles.clone(), id("other")).unwrap();
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
            Domain::finite(CanonicalType::Enum(roles), [manager.clone(), other]).unwrap(),
        ),
    ];
    let sat =
        ConstraintProblem::from_overlap(vars.clone(), permit.clone(), denial.clone()).unwrap();
    let SolveResult::Sat(CanonicalModel(model)) = Z3Solver::new()
        .solve(&sat, SolveOptions::default())
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(model.get(&id("role")), Some(&manager));
    assert_eq!(model.get(&id("applicant")), model.get(&id("approver")));
    let no_self = BooleanExpression::and([permit, BooleanExpression::negate(denial.clone())]);
    let unsat = ConstraintProblem::from_overlap(vars, no_self, denial).unwrap();
    assert!(matches!(
        Z3Solver::new()
            .solve(&unsat, SolveOptions::default())
            .unwrap(),
        SolveResult::Unsat
    ));
}
