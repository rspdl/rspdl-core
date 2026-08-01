use crate::logic::validate_predicate_application;
use crate::{Atom, CanonicalId, ModelError, PredicateSignature, Term};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PredicateApplication {
    signature: PredicateSignature,
    arguments: Vec<Term>,
}
impl PredicateApplication {
    pub fn new(signature: PredicateSignature, arguments: Vec<Term>) -> Result<Self, ModelError> {
        validate_predicate_application(&signature, &arguments)?;
        Ok(Self {
            signature,
            arguments,
        })
    }
    pub fn signature(&self) -> &PredicateSignature {
        &self.signature
    }
    pub fn arguments(&self) -> &[Term] {
        &self.arguments
    }
}
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Fact {
    application: PredicateApplication,
}
impl Fact {
    pub fn new(application: PredicateApplication) -> Result<Self, ModelError> {
        let variable = application.arguments().iter().find_map(|term| match term {
            Term::Variable(variable) => Some(variable.id().clone()),
            Term::Constant(_) => None,
        });
        if let Some(variable) = variable {
            return Err(ModelError::NonGroundFact {
                predicate: application.signature().id().clone(),
                variable,
            });
        }
        Ok(Self { application })
    }
    pub fn application(&self) -> &PredicateApplication {
        &self.application
    }
}
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum RuleLiteral {
    Positive(PredicateApplication),
    Negative(PredicateApplication),
    Constraint(Atom),
}
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DerivationRule {
    id: CanonicalId,
    head: PredicateApplication,
    body: Vec<RuleLiteral>,
}
impl DerivationRule {
    pub fn new(id: CanonicalId, head: PredicateApplication, body: Vec<RuleLiteral>) -> Self {
        Self { id, head, body }
    }
    pub fn id(&self) -> &CanonicalId {
        &self.id
    }
    pub fn head(&self) -> &PredicateApplication {
        &self.head
    }
    pub fn body(&self) -> &[RuleLiteral] {
        &self.body
    }
}
#[derive(Clone, Debug)]
pub struct LogicProgram {
    predicates: BTreeMap<CanonicalId, PredicateSignature>,
    facts: Vec<Fact>,
    rules: Vec<DerivationRule>,
}
impl LogicProgram {
    pub fn new(
        predicates: Vec<PredicateSignature>,
        facts: Vec<Fact>,
        rules: Vec<DerivationRule>,
    ) -> Result<Self, ModelError> {
        let mut map = BTreeMap::new();
        for p in predicates {
            if let Some(old) = map.insert(p.id().clone(), p.clone()) {
                if old != p {
                    return Err(ModelError::ConflictingPredicateSignature {
                        predicate: p.id().clone(),
                    });
                }
            }
        }
        for f in &facts {
            Self::known(&map, f.application())?;
        }
        for r in &rules {
            Self::known(&map, r.head())?;
            for x in r.body() {
                match x {
                    RuleLiteral::Positive(a) | RuleLiteral::Negative(a) => Self::known(&map, a)?,
                    RuleLiteral::Constraint(_) => {}
                }
            }
        }
        Ok(Self {
            predicates: map,
            facts,
            rules,
        })
    }
    fn known(
        map: &BTreeMap<CanonicalId, PredicateSignature>,
        a: &PredicateApplication,
    ) -> Result<(), ModelError> {
        match map.get(a.signature().id()) {
            Some(p) if p == a.signature() => Ok(()),
            Some(_) => Err(ModelError::ConflictingPredicateSignature {
                predicate: a.signature().id().clone(),
            }),
            None => Err(ModelError::UnknownPredicate {
                predicate: a.signature().id().clone(),
            }),
        }
    }
    pub fn predicates(&self) -> &BTreeMap<CanonicalId, PredicateSignature> {
        &self.predicates
    }
    pub fn facts(&self) -> &[Fact] {
        &self.facts
    }
    pub fn rules(&self) -> &[DerivationRule] {
        &self.rules
    }
}
