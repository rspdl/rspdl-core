use rspdl_core::*;
use rspdl_datalog::DatalogEvaluator;
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
            Domain::finite(CanonicalType::Enum(roles), [manager.clone()]).unwrap(),
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

#[test]
fn ground_accounting_rules_materialize_both_conditions() {
    let string = CanonicalType::String;
    let manager = PredicateSignature::new(id("accounting_manager"), vec![string.clone()]);
    let applicant = PredicateSignature::new(id("applicant"), vec![string.clone(), string.clone()]);
    let approver = PredicateSignature::new(id("approver"), vec![string.clone(), string.clone()]);
    let approval = PredicateSignature::new(id("accounting_approval"), vec![string.clone()]);
    let self_application = PredicateSignature::new(id("self_application"), vec![string.clone()]);
    let alice = Term::Constant(CanonicalValue::string("alice"));
    let request = Term::Constant(CanonicalValue::string("request_one"));
    let r = Term::Variable(Variable::new(id("r"), string.clone()));
    let a = Term::Variable(Variable::new(id("a"), string.clone()));
    let app = |p: &PredicateSignature, args| PredicateApplication::new(p.clone(), args).unwrap();
    let facts = vec![
        Fact::new(app(&manager, vec![alice.clone()])).unwrap(),
        Fact::new(app(&applicant, vec![request.clone(), alice.clone()])).unwrap(),
        Fact::new(app(&approver, vec![request.clone(), alice.clone()])).unwrap(),
    ];
    let allow = DerivationRule::new(
        id("derive_approval"),
        app(&approval, vec![r.clone()]),
        vec![
            RuleLiteral::Positive(app(&approver, vec![r.clone(), a.clone()])),
            RuleLiteral::Positive(app(&manager, vec![a.clone()])),
        ],
    );
    let self_rule = DerivationRule::new(
        id("derive_self"),
        app(&self_application, vec![r.clone()]),
        vec![
            RuleLiteral::Positive(app(&applicant, vec![r.clone(), a.clone()])),
            RuleLiteral::Positive(app(&approver, vec![r, a])),
        ],
    );
    let program = LogicProgram::new(
        vec![
            manager,
            applicant,
            approver,
            approval.clone(),
            self_application.clone(),
        ],
        facts,
        vec![allow, self_rule],
    )
    .unwrap();
    let db = DatalogEvaluator::evaluate(&program).unwrap();
    assert_eq!(db.tuples(approval.id()).unwrap().len(), 1);
    assert_eq!(db.tuples(self_application.id()).unwrap().len(), 1);
}
