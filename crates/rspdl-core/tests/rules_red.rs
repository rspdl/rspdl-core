use rspdl_core::{CanonicalId, CanonicalType, LogicProgram, PredicateSignature};

#[test]
fn rule_program_rejects_conflicting_predicate_signatures() {
    let id = CanonicalId::new("p").unwrap();
    let first = PredicateSignature::new(id.clone(), vec![CanonicalType::Boolean]);
    let second = PredicateSignature::new(id, vec![CanonicalType::Integer]);
    assert!(LogicProgram::new(vec![first, second], vec![], vec![]).is_err());
}
