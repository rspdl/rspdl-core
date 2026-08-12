---
id: rspdl-knowledge-index
title: RSPDL Knowledge Index
type: index
status: active
version: "1"
summary: Generated metadata catalog for progressively discovering RSPDL repository knowledge.
topics:
  - knowledge-navigation
  - document-index
related:
  - core-application-boundary
  - rust-korean-first-frontend
  - rspdl-compiler-architecture
  - problem-driven-development
  - rspdl-language-prd
  - rspdl-product-vision
  - data-lifecycle-modeling-gap
  - policy-consistency-blind-spots
  - controlled-korean-surface-grammar
  - field-provenance-and-sum-derivation
  - finite-relational-model-finding
  - natural-korean-domain-grammar
  - total-policy-condition-space-analysis
  - typed-domains-and-logic-core
  - frontend-semantic-analysis-contract
last_updated: "2026-08-12"
owners:
  - rspdl-maintainers
---

# RSPDL Knowledge Index

> Generated from document front matter. Run the knowledge skill's `build` command; do not edit entries manually.

| ID | Type | Status | Document | Summary | Problems | Topics |
| --- | --- | --- | --- | --- | --- | --- |
| `core-application-boundary` | `adr` | `active` | [Core와 Application Projection 경계](adr/0002-core-application-boundary.md) | Keeps compiler, IR, semantic analysis, and diagnostics in the RSPDL core while assigning view projections, filtering, and aggregation to applications. | `data-lifecycle-modeling-gap`, `policy-consistency-blind-spots` | `compiler-boundary`, `semantic-ir`, `application-projection`, `policy-tables` |
| `rust-korean-first-frontend` | `adr` | `accepted` | [Rust와 한국어 우선 독립 Locale Frontend](adr/0001-rust-korean-first-frontends.md) | Selects Rust, a Korean-first rollout, and independent deterministic frontends that lower surface names to a shared stable-ID Unlinked IR. | `data-lifecycle-modeling-gap`, `policy-consistency-blind-spots` | `rust`, `korean-first`, `controlled-language`, `locale-frontends`, `deterministic-parsing` |
| `rspdl-compiler-architecture` | `architecture` | `proposed` | [RSPDL Compiler Architecture](architecture.md) | Defines the stable-ID frontend boundary, locale-neutral analyzer, bounded relational model-finding path, dependency direction, and tests. | `data-lifecycle-modeling-gap`, `policy-consistency-blind-spots` | `rust`, `compiler-architecture`, `ko-KR`, `semantic-ir`, `diagnostics`, `conformance` |
| `problem-driven-development` | `guide` | `active` | [Problem-driven Development](guides/problem-driven-development.md) | Defines how contributors trace every product or language change from a durable causal problem through evidence and conformance tests. | - | `contribution-workflow`, `intent-traceability`, `problem-topic`, `definition-of-done` |
| `rspdl-language-prd` | `prd` | `draft` | [RSPDL Product Requirements](prd.md) | Defines the product and language requirements for turning explicit planning intent into deterministic, explainable implementation context. | `data-lifecycle-modeling-gap`, `policy-consistency-blind-spots` | `language-design`, `data-lifecycle`, `policy-analysis`, `semantic-ir`, `diagnostics`, `conformance` |
| `rspdl-product-vision` | `prd` | `active` | [RSPDL Product Vision](product/vision.md) | Defines the product promise of moving policy and data decisions before implementation while preserving explicitly modeled intent. | `data-lifecycle-modeling-gap`, `policy-consistency-blind-spots` | `product-vision`, `planning-to-implementation`, `shift-left-validation`, `canonical-intent` |
| `data-lifecycle-modeling-gap` | `problem` | `active` | [Data Lifecycle Modeling Gap](problems/0001-data-lifecycle-modeling-gap.md) | Planning artifacts often omit when data comes into existence, changes, disappears, and remains available to dependent behavior. | - | `data-lifecycle`, `state-transition`, `derivation`, `deletion-impact` |
| `policy-consistency-blind-spots` | `problem` | `active` | [Policy Consistency Blind Spots](problems/0002-policy-consistency-blind-spots.md) | Prose planning hides contradictory, uncovered, overlapping, and unreachable policy branches that become visible only during implementation. | - | `policy-conflict`, `policy-gap`, `condition-coverage`, `counterexample` |
| `controlled-korean-surface-grammar` | `rfc` | `proposed` | [Controlled Korean Surface Grammar](rfcs/0001-controlled-korean-surface-grammar.md) | Proposes a deterministic Korean surface grammar that treats particles and endings as structural markers rather than morphology. | `data-lifecycle-modeling-gap`, `policy-consistency-blind-spots` | `ko-KR`, `controlled-language`, `surface-grammar`, `cfg`, `parser`, `diagnostics` |
| `field-provenance-and-sum-derivation` | `rfc` | `implemented` | [Field Provenance, Screen Usage, and Sum Derivation Grammar](rfcs/0005-field-provenance-and-sum-derivation.md) | Defines sentence-shaped screen data operations, field provenance checks, cross-model sum dependencies, and explicit recalculation triggers. | `data-lifecycle-modeling-gap` | `data-lifecycle`, `field-provenance`, `screen-usage`, `derivation`, `aggregation`, `diagnostics` |
| `finite-relational-model-finding` | `rfc` | `implemented` | [Finite Relational Rules and Bounded Model Finding](rfcs/0007-finite-relational-model-finding.md) | Defines unary and binary relations, explicit relational meta-rules, and bounded virtual-data model finding without runtime records. | `data-lifecycle-modeling-gap`, `policy-consistency-blind-spots` | `first-order-logic`, `relation`, `bounded-model-finding`, `cardinality`, `counterexample` |
| `natural-korean-domain-grammar` | `rfc` | `implemented` | [Korean Domain Frontend Language Specification](rfcs/0004-natural-korean-domain-grammar.md) | Defines Korean record, relation, constraint and policy grammar and its deterministic lowering to the locale-neutral Unlinked IR contract. | `data-lifecycle-modeling-gap`, `policy-consistency-blind-spots` | `ko-KR`, `controlled-language`, `data-model`, `constraints`, `policies`, `relations`, `bounded-model-finding`, `cfg` |
| `total-policy-condition-space-analysis` | `rfc` | `proposed` | [Total Policy Condition Spaces and SMT-First Consistency Analysis](rfcs/0006-total-policy-condition-space-analysis.md) | Defines closed policy vocabulary, exhaustive condition-space coverage, explicit override semantics, and SMT-first consistency analysis. | `policy-consistency-blind-spots`, `data-lifecycle-modeling-gap` | `policy-analysis`, `smt`, `condition-coverage`, `totality`, `override`, `closed-vocabulary` |
| `typed-domains-and-logic-core` | `rfc` | `proposed` | [정규화 타입·도메인과 논리 IR 코어](rfcs/0002-typed-domains-and-logic-core.md) | Defines normalized value domains, typed set and Boolean IR, and its boundary with finite relational model finding. | `data-lifecycle-modeling-gap`, `policy-consistency-blind-spots` | `type-system`, `data-model`, `domains`, `set-algebra`, `smt` |
| `frontend-semantic-analysis-contract` | `spec` | `implemented` | [Frontend and Semantic Analysis Contract](specs/frontend-semantic-analysis-contract.md) | Defines stable-ID Unlinked records, relations and rules plus the structured diagnostic boundary shared by independent frontends. | `data-lifecycle-modeling-gap`, `policy-consistency-blind-spots` | `compiler-frontend`, `unlinked-ir`, `semantic-analysis`, `locale-independence`, `conformance` |
