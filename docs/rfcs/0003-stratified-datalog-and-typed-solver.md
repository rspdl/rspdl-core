---
id: stratified-datalog-and-typed-solver
title: Stratified Datalog and Typed Solver
type: rfc
status: proposed
version: "1.1"
summary: Defines active-domain stratified Datalog evaluation and the typed constraint-solver contract.
topics: [datalog, stratification, smt, z3, constraints]
related: [typed-domains-and-logic-core, total-policy-condition-space-analysis]
problem_refs:
  - policy-consistency-blind-spots
last_updated: "2026-08-12"
owners: [rspdl-maintainers]
---

# Stratified Datalog and Typed Solver

Rules use active-domain values and have no function terms. A rule head, negative
literal, equality, or membership test may only use variables bound by a positive
predicate literal. Positive recursion reaches a deterministic fixed point.

`not p(x)` is closed-world negation evaluated only after lower strata have
materialized. A negative edge in a recursive component is rejected, rather than
assigned an implementation-specific meaning.

The constraint API returns `Sat(model)`, `Unsat`, or `Unknown { reason }`.
The default timeout is five seconds and zero is invalid. Solvers must not
approximate prime refinements or predicate applications; unsupported constructs
are structured errors. SAT models contain every declared variable in canonical
identifier order.

## Delivery sequencing

Static `conflict`, `gap`, `overlap`, and `unreachable` analysis follows the
SMT-first contract in [Total Policy Condition Spaces and SMT-First Consistency
Analysis](0006-total-policy-condition-space-analysis.md). The existing Datalog
runtime matcher remains in scope, but new Datalog semantics are deferred until a
product scenario requires recursive closure over declared finite relations.

Runtime `unmatched` does not prove a static policy gap, and absence of a Datalog
fact is not automatically lowered to SMT logical negation.
