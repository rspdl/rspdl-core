# RSPDL knowledge front matter

Use flat YAML so the indexer can read metadata without loading document bodies.

## Required fields

| Field | Meaning |
| --- | --- |
| `id` | Stable, unique kebab-case document ID |
| `title` | Human-readable title |
| `type` | `prd`, `adr`, `rfc`, `architecture`, `spec`, `guide`, or `index` |
| `status` | `draft`, `proposed`, `accepted`, `active`, `final`, `superseded`, or `deprecated` |
| `version` | Document or specification revision as a quoted string |
| `summary` | One-line discovery summary; not a substitute for the body |
| `topics` | Search terms describing the document |
| `related` | IDs of directly related documents; use an empty list when none |
| `last_updated` | ISO date (`YYYY-MM-DD`) |

## Optional fields

- `owners`: maintainer IDs
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
---
```

Use document IDs in relations, not file paths. Paths may change; IDs must remain stable.
