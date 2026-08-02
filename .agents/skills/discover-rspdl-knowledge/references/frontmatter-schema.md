# RSPDL knowledge front matter

Use flat YAML so the indexer can read metadata without loading document bodies.

## Required fields

| Field | Meaning |
| --- | --- |
| `id` | Stable, unique kebab-case document ID |
| `title` | Human-readable title |
| `type` | `prd`, `adr`, `rfc`, `architecture`, `spec`, `guide`, `problem`, or `index` |
| `status` | `draft`, `proposed`, `accepted`, `active`, `final`, `implemented`, `superseded`, or `deprecated` |
| `version` | Document or specification revision as a quoted string |
| `summary` | One-line discovery summary; not a substitute for the body |
| `topics` | Search terms describing the document |
| `related` | IDs of directly related documents; use an empty list when none |
| `last_updated` | ISO date (`YYYY-MM-DD`) |

## Optional fields

- `owners`: maintainer IDs
- `created`: creation date; required for `problem` documents
- `problem_refs`: stable IDs of causal `problem` documents; required for `prd`, `adr`, `rfc`, `architecture`, and `spec`
- `supersedes`: IDs replaced by this document
- `superseded_by`: ID replacing this document
- `target_spec`: RSPDL specification version affected by the document

## Example

```yaml
---
id: rspdl-language-prd
title: RSPDL Language Product Requirements Document
type: prd
status: draft
version: "0.3"
summary: Defines the goals, semantics, multilingual model, and conformance requirements of RSPDL.
topics:
  - language-design
  - multilingual-frontends
  - semantic-ir
related: []
last_updated: "2026-07-26"
owners:
  - rspdl-maintainers
target_spec: "0.1.0"
problem_refs:
  - data-lifecycle-modeling-gap
---
```

Use document IDs in relations, not file paths. Paths may change; IDs must remain stable.

## Problem topic contract

- Store durable problem topics under `docs/problems/` with `type: problem`.
- Keep each problem topic to one causal mechanism and at most 150 lines.
- Use the ordered sections `Why`, `What`, `How`, `Constraints`, and `References`.
- Link solution documents through `problem_refs`; do not encode a feature name as the problem.
