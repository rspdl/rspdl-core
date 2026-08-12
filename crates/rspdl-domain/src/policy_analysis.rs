//! Backend-neutral analysis for total policy decision points.
//!
//! This initial slice deliberately accepts one closed enum variable.  It does
//! not assign priority, infer defaults, or give source order semantic meaning.

use std::collections::BTreeSet;

use serde::Serialize;

use crate::{
    Atom, BooleanExpression, CanonicalId, CanonicalModel, CanonicalType, CanonicalValue,
    ConstraintProblem, ConstraintSolver, PolicyEffect, SolveOptions, SolveResult, Term, Variable,
    VariableDomain,
};

/// One independently effective branch in a total decision point.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PolicyBranch {
    id: CanonicalId,
    condition: BooleanExpression,
    effect: PolicyEffect,
}

impl PolicyBranch {
    pub fn new(id: CanonicalId, condition: BooleanExpression, effect: PolicyEffect) -> Self {
        Self {
            id,
            condition,
            effect,
        }
    }

    pub fn id(&self) -> &CanonicalId {
        &self.id
    }

    pub fn condition(&self) -> &BooleanExpression {
        &self.condition
    }

    pub const fn effect(&self) -> PolicyEffect {
        self.effect
    }
}

/// A decision that must be made for every value of one declared enum domain.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TotalDecisionPoint {
    variable: VariableDomain,
    branches: Vec<PolicyBranch>,
}

impl TotalDecisionPoint {
    /// Constructs a total decision point after validating its closed enum domain
    /// and stable branch identities.
    pub fn new(
        variable: VariableDomain,
        branches: impl IntoIterator<Item = PolicyBranch>,
    ) -> Result<Self, DecisionPointError> {
        let CanonicalType::Enum(enum_type) = variable.domain().value_type() else {
            return Err(DecisionPointError::NonEnumVariable {
                variable: variable.id().clone(),
                value_type: variable.domain().value_type().clone(),
            });
        };

        let expected = enum_type
            .variants()
            .iter()
            .cloned()
            .map(|variant| CanonicalValue::enum_variant(enum_type.clone(), variant))
            .collect::<Result<BTreeSet<_>, _>>()
            .expect("declared enum variants must construct canonical values");
        if variable.domain().finite_values() != Some(&expected) {
            return Err(DecisionPointError::NonClosedEnumDomain {
                variable: variable.id().clone(),
            });
        }

        let mut ids = BTreeSet::new();
        let mut branches: Vec<_> = branches.into_iter().collect();
        for branch in &branches {
            if !ids.insert(branch.id.clone()) {
                return Err(DecisionPointError::DuplicateBranch(branch.id.clone()));
            }
        }
        branches.sort_by(|left, right| left.id.cmp(&right.id));
        Ok(Self { variable, branches })
    }

    pub fn variable(&self) -> &VariableDomain {
        &self.variable
    }

    /// Branches are returned in canonical stable-ID order.
    pub fn branches(&self) -> &[PolicyBranch] {
        &self.branches
    }
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum DecisionPointError {
    #[error("decision variable `{variable}` has non-enum type `{value_type}`")]
    NonEnumVariable {
        variable: CanonicalId,
        value_type: CanonicalType,
    },
    #[error("decision variable `{variable}` must contain every declared enum variant")]
    NonClosedEnumDomain { variable: CanonicalId },
    #[error("duplicate policy branch `{0}`")]
    DuplicateBranch(CanonicalId),
}

/// A reproducible uncovered enum value and the solver model that proved it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EnumGap {
    variant: CanonicalId,
    witness: CanonicalModel,
}

impl EnumGap {
    pub fn variant(&self) -> &CanonicalId {
        &self.variant
    }

    pub fn witness(&self) -> &CanonicalModel {
        &self.witness
    }
}

/// Two compatible branches that apply to the same declared input.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CompatibleOverlap {
    branch_ids: [CanonicalId; 2],
    effect: PolicyEffect,
    witness: CanonicalModel,
}

impl CompatibleOverlap {
    pub fn branch_ids(&self) -> &[CanonicalId; 2] {
        &self.branch_ids
    }

    pub const fn effect(&self) -> PolicyEffect {
        self.effect
    }

    pub fn witness(&self) -> &CanonicalModel {
        &self.witness
    }
}

/// Two incompatible authorization effects that apply to the same input.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PolicyConflict {
    branch_ids: [CanonicalId; 2],
    effects: [PolicyEffect; 2],
    witness: CanonicalModel,
}

impl PolicyConflict {
    pub fn branch_ids(&self) -> &[CanonicalId; 2] {
        &self.branch_ids
    }

    pub fn effects(&self) -> &[PolicyEffect; 2] {
        &self.effects
    }

    pub fn witness(&self) -> &CanonicalModel {
        &self.witness
    }
}

/// A query whose result was not a proof of either satisfiability or unsatisfiability.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AnalysisUnknown {
    query: PolicyAnalysisQuery,
    reason: String,
}

impl AnalysisUnknown {
    pub fn query(&self) -> &PolicyAnalysisQuery {
        &self.query
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }
}

/// Identifies the deterministic query that returned `UNKNOWN`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "definition", rename_all = "snake_case")]
pub enum PolicyAnalysisQuery {
    Coverage { variant: CanonicalId },
    BranchPair { branch_ids: [CanonicalId; 2] },
}

/// Findings for a total decision point.
///
/// `gaps` contains every uncovered enum variant in canonical order when
/// [`Self::coverage_is_exact`] is true. If a coverage query is unknown, no
/// caller may interpret the `gaps` list as an exhaustive coverage proof.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TotalDecisionAnalysis {
    gaps: Vec<EnumGap>,
    compatible_overlaps: Vec<CompatibleOverlap>,
    conflicts: Vec<PolicyConflict>,
    unknowns: Vec<AnalysisUnknown>,
}

impl TotalDecisionAnalysis {
    pub fn gaps(&self) -> &[EnumGap] {
        &self.gaps
    }

    pub fn compatible_overlaps(&self) -> &[CompatibleOverlap] {
        &self.compatible_overlaps
    }

    pub fn conflicts(&self) -> &[PolicyConflict] {
        &self.conflicts
    }

    pub fn unknowns(&self) -> &[AnalysisUnknown] {
        &self.unknowns
    }

    pub fn coverage_is_exact(&self) -> bool {
        !self
            .unknowns
            .iter()
            .any(|unknown| matches!(&unknown.query, PolicyAnalysisQuery::Coverage { .. }))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PolicyAnalysisError<E: std::error::Error + Send + Sync + 'static> {
    #[error("constraint solver failed: {0}")]
    Solver(#[source] E),
    #[error("failed to construct a typed coverage constraint: {0}")]
    Contract(#[from] crate::SolverContractError),
    #[error("failed to construct an enum value for coverage: {0}")]
    Model(#[from] crate::ModelError),
}

/// Analyzes total coverage and pairwise policy interaction through the supplied
/// solver. Solver `UNKNOWN` is preserved in the result; backend errors remain
/// structured errors and are never interpreted as a passing result.
pub fn analyze_total_decision_point<S: ConstraintSolver>(
    decision_point: &TotalDecisionPoint,
    solver: &S,
    options: SolveOptions,
) -> Result<TotalDecisionAnalysis, PolicyAnalysisError<S::Error>> {
    let enum_type = match decision_point.variable.domain().value_type() {
        CanonicalType::Enum(enum_type) => enum_type,
        _ => unreachable!("TotalDecisionPoint validates an enum variable"),
    };
    let variable = Variable::new(
        decision_point.variable.id().clone(),
        decision_point.variable.domain().value_type().clone(),
    );
    let coverage = BooleanExpression::or(
        decision_point
            .branches
            .iter()
            .map(|branch| branch.condition.clone()),
    );
    let mut gaps = Vec::new();
    let mut compatible_overlaps = Vec::new();
    let mut conflicts = Vec::new();
    let mut unknowns = Vec::new();

    for variant in enum_type.variants() {
        let value = CanonicalValue::enum_variant(enum_type.clone(), variant.clone())?;
        let is_variant = BooleanExpression::atom(Atom::equal(
            Term::Variable(variable.clone()),
            Term::Constant(value),
        )?);
        let assertion =
            BooleanExpression::and([is_variant, BooleanExpression::negate(coverage.clone())]);
        let problem = ConstraintProblem::new(vec![decision_point.variable.clone()], assertion)?;
        match solver
            .solve(&problem, options)
            .map_err(PolicyAnalysisError::Solver)?
        {
            SolveResult::Sat(witness) => gaps.push(EnumGap {
                variant: variant.clone(),
                witness,
            }),
            SolveResult::Unsat => {}
            SolveResult::Unknown { reason } => unknowns.push(AnalysisUnknown {
                query: PolicyAnalysisQuery::Coverage {
                    variant: variant.clone(),
                },
                reason,
            }),
        }
    }

    for (index, left) in decision_point.branches.iter().enumerate() {
        for right in decision_point.branches.iter().skip(index + 1) {
            let branch_ids = [left.id.clone(), right.id.clone()];
            let problem = ConstraintProblem::from_overlap(
                vec![decision_point.variable.clone()],
                left.condition.clone(),
                right.condition.clone(),
            )?;
            match solver
                .solve(&problem, options)
                .map_err(PolicyAnalysisError::Solver)?
            {
                SolveResult::Sat(witness) if left.effect == right.effect => {
                    compatible_overlaps.push(CompatibleOverlap {
                        branch_ids,
                        effect: left.effect,
                        witness,
                    });
                }
                SolveResult::Sat(witness) => conflicts.push(PolicyConflict {
                    branch_ids,
                    effects: [left.effect, right.effect],
                    witness,
                }),
                SolveResult::Unsat => {}
                SolveResult::Unknown { reason } => unknowns.push(AnalysisUnknown {
                    query: PolicyAnalysisQuery::BranchPair { branch_ids },
                    reason,
                }),
            }
        }
    }

    Ok(TotalDecisionAnalysis {
        gaps,
        compatible_overlaps,
        conflicts,
        unknowns,
    })
}
