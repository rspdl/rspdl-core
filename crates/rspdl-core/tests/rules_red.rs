use rspdl_core::{
    CanonicalId, CanonicalType, Fact, LogicProgram, ModelError, PredicateApplication,
    PredicateSignature, Term, Variable,
};

#[test]
fn rule_program_rejects_conflicting_predicate_signatures() {
    let id = CanonicalId::new("p").unwrap();
    let first = PredicateSignature::new(id.clone(), vec![CanonicalType::Boolean]);
    let second = PredicateSignature::new(id.clone(), vec![CanonicalType::Integer]);
    assert!(matches!(
        LogicProgram::new(vec![first, second], vec![], vec![]),
        Err(ModelError::ConflictingPredicateSignature { predicate }) if predicate == id
    ));
}

#[test]
fn facts_reject_variables_at_the_model_boundary() {
    let predicate = CanonicalId::new("employee").unwrap();
    let variable = CanonicalId::new("person").unwrap();
    let signature = PredicateSignature::new(predicate.clone(), vec![CanonicalType::String]);
    let application = PredicateApplication::new(
        signature,
        vec![Term::Variable(Variable::new(
            variable.clone(),
            CanonicalType::String,
        ))],
    )
    .unwrap();

    assert!(matches!(
        Fact::new(application),
        Err(ModelError::NonGroundFact {
            predicate: actual_predicate,
            variable: actual_variable,
        }) if actual_predicate == predicate && actual_variable == variable
    ));
}
