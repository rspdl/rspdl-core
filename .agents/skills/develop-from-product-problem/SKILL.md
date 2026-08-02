---
name: develop-from-product-problem
description: Trace RSPDL product, language, semantic, or architecture changes from a durable causal problem through requirements, implementation, diagnostics, and conformance evidence. Use before implementing features or changing grammar, Canonical IR, semantic rules, RFCs, ADRs, architecture, or specs.
---

# Develop from a Product Problem

Preserve product intent by choosing the causal problem before choosing a feature.

## Context discovery

1. Read `README.md` and `docs/product/vision.md` when the product outcome is relevant.
2. Run the knowledge query with causal terms from the request:

```bash
python3 "$(git rev-parse --show-toplevel)/.agents/skills/discover-rspdl-knowledge/scripts/knowledge_index.py" query "<cause and domain terms>"
```

3. Prefer a matching `problem` document and inspect its graph.
4. Read the relevant problem body and only the solution sections needed for the change.
5. If no problem topic explains the cause, create one from `references/problem-topic-template.md` before proposing the implementation.

## Design gate

Define these items before editing product behavior:

- failing user or developer scenario;
- causal mechanism, not only the requested feature;
- current workaround, wait, ambiguity, or rework cost;
- smallest end-to-end vertical slice;
- expected structured diagnostic and evidence;
- normal, failure, boundary, and false-positive-prevention cases;
- intentionally unsupported behavior and `unknown` semantics.

For data behavior, also define create, read, update, delete, derive, state reachability, and dependency impact.

For policy behavior, also define conflict, gap, overlap, unreachable, totality, default, override, and a reproducible witness.

## Traceability gate

1. Add the selected problem ID to `problem_refs` in every affected PRD, RFC, ADR, architecture, or spec.
2. Keep solution decisions in RFCs or ADRs; keep the problem document solution-neutral.
3. Use the same domain terms and Rule IDs in docs, code, diagnostics, and fixtures.
4. Add tests at the closest owning layer and conformance fixtures for public semantics.
5. Rebuild and validate the knowledge index.

## Completion gate

Run:

```bash
./scripts/check.sh
```

Review the diff for a complete chain:

```text
Problem Topic -> Requirement/Decision -> Code -> Test/Diagnostic Evidence
```

Do not claim completion when any required link is absent.
