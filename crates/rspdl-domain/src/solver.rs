use crate::{BooleanExpression, CanonicalId, CanonicalValue, Domain};
use serde::Serialize;
use std::collections::BTreeMap;
use std::time::Duration;
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct VariableDomain {
    id: CanonicalId,
    domain: Domain,
}
impl VariableDomain {
    pub fn new(id: CanonicalId, domain: Domain) -> Self {
        Self { id, domain }
    }
    pub fn id(&self) -> &CanonicalId {
        &self.id
    }
    pub fn domain(&self) -> &Domain {
        &self.domain
    }
}
#[derive(Clone, Debug)]
pub struct ConstraintProblem {
    variables: Vec<VariableDomain>,
    assertion: BooleanExpression,
}
impl ConstraintProblem {
    pub fn new(
        variables: Vec<VariableDomain>,
        assertion: BooleanExpression,
    ) -> Result<Self, SolverContractError> {
        let mut declared = BTreeMap::new();
        for variable in &variables {
            if declared.insert(variable.id().clone(), ()).is_some() {
                return Err(SolverContractError::DuplicateVariable(
                    variable.id().clone(),
                ));
            }
        }
        Ok(Self {
            variables,
            assertion,
        })
    }
    pub fn from_overlap(
        variables: Vec<VariableDomain>,
        left: BooleanExpression,
        right: BooleanExpression,
    ) -> Result<Self, SolverContractError> {
        Self::new(variables, BooleanExpression::and([left, right]))
    }
    pub fn variables(&self) -> &[VariableDomain] {
        &self.variables
    }
    pub fn assertion(&self) -> &BooleanExpression {
        &self.assertion
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SolveOptions {
    timeout: Duration,
}
impl Default for SolveOptions {
    fn default() -> Self {
        Self {
            timeout: Duration::from_millis(5000),
        }
    }
}
impl SolveOptions {
    pub fn with_timeout(timeout: Duration) -> Result<Self, SolverContractError> {
        if timeout.is_zero() {
            Err(SolverContractError::ZeroTimeout)
        } else {
            Ok(Self { timeout })
        }
    }
    pub fn timeout(&self) -> Duration {
        self.timeout
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CanonicalModel(pub BTreeMap<CanonicalId, CanonicalValue>);
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SolveResult {
    Sat(CanonicalModel),
    Unsat,
    Unknown { reason: String },
}
pub trait ConstraintSolver {
    type Error: std::error::Error + Send + Sync + 'static;
    fn solve(
        &self,
        problem: &ConstraintProblem,
        options: SolveOptions,
    ) -> Result<SolveResult, Self::Error>;
}
#[derive(Clone, Debug, thiserror::Error, Eq, PartialEq)]
pub enum SolverContractError {
    #[error("solver timeout must be non-zero")]
    ZeroTimeout,
    #[error("duplicate variable `{0}`")]
    DuplicateVariable(CanonicalId),
}
