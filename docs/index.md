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
  - rust-korean-first-frontend
  - rspdl-compiler-architecture
  - rspdl-language-prd
  - controlled-korean-surface-grammar
  - typed-domains-and-logic-core
last_updated: "2026-07-28"
owners:
  - rspdl-maintainers
---

# RSPDL Knowledge Index

> Generated from document front matter. Run the knowledge skill's `build` command; do not edit entries manually.

| ID | Type | Status | Document | Summary | Topics |
| --- | --- | --- | --- | --- | --- | --- |
| `rust-korean-first-frontend` | `adr` | `accepted` | [Rust와 한국어 우선 독립 Locale Frontend](adr/0001-rust-korean-first-frontends.md) | Selects Rust, a Korean-first rollout, and fully independent deterministic frontends without morphological analysis. | `rust`, `korean-first`, `controlled-language`, `locale-frontends`, `deterministic-parsing` |
| `rspdl-compiler-architecture` | `architecture` | `proposed` | [RSPDL Compiler Architecture](architecture.md) | Defines the Korean-first Rust compiler boundaries, dependency direction, pipeline, and test architecture. | `rust`, `compiler-architecture`, `ko-KR`, `semantic-ir`, `diagnostics`, `conformance` |
| `rspdl-language-prd` | `prd` | `draft` | [RSPDL Language Product Requirements Document](prd.md) | Defines the goals, semantics, multilingual model, and conformance requirements of the RSPDL language. | `language-design`, `multilingual-frontends`, `semantic-ir`, `semantic-analysis`, `conformance` |
| `controlled-korean-surface-grammar` | `rfc` | `proposed` | [Controlled Korean Surface Grammar](rfcs/0001-controlled-korean-surface-grammar.md) | Proposes a deterministic Korean surface grammar that treats particles and endings as structural markers rather than morphology. | `ko-KR`, `controlled-language`, `surface-grammar`, `cfg`, `parser`, `diagnostics` |
| `typed-domains-and-logic-core` | `rfc` | `proposed` | [정규화 타입·도메인과 논리 IR 코어](rfcs/0002-typed-domains-and-logic-core.md) | Defines normalized data types, finite and computable infinite domains, typed set algebra, and the shared logical expression core. | `type-system`, `data-model`, `domains`, `set-algebra`, `datalog`, `smt` |
