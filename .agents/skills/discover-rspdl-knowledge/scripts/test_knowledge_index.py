import tempfile
import unittest
from pathlib import Path

from knowledge_index import load_documents, validate_documents


PROBLEM = """---
id: causal-gap
title: Causal Gap
type: problem
status: active
created: 2026-08-02
version: "1"
summary: Describes one recurring causal gap.
topics: [gap]
related: []
last_updated: 2026-08-02
---

# Causal Gap

## Why

- A failure repeats.

## What

- One mechanism causes it.

## How

- Evidence can distinguish it.

## Constraints

- Do not infer missing intent.

## References

- None.
"""


def solution(problem_refs: str) -> str:
    return f"""---
id: candidate-spec
title: Candidate Spec
type: spec
status: draft
version: "1"
summary: Defines a candidate semantic contract.
topics: [semantics]
related: []
problem_refs: [{problem_refs}]
last_updated: 2026-08-02
---

# Candidate Spec
"""


class KnowledgeIndexValidationTests(unittest.TestCase):
    def validate(self, files: dict[str, str]) -> list[str]:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for relative, content in files.items():
                path = root / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(content, encoding="utf-8")
            documents, load_errors = load_documents(root)
            return validate_documents(root, documents, load_errors)

    def test_accepts_solution_linked_to_problem(self) -> None:
        errors = self.validate(
            {
                "docs/problems/causal-gap.md": PROBLEM,
                "docs/spec.md": solution("causal-gap"),
            }
        )
        self.assertEqual([], errors)

    def test_rejects_solution_without_problem_reference(self) -> None:
        errors = self.validate({"docs/spec.md": solution("")})
        self.assertTrue(
            any("requires at least one problem_refs entry" in error for error in errors),
            errors,
        )

    def test_rejects_problem_reference_to_solution_document(self) -> None:
        errors = self.validate(
            {
                "docs/first.md": solution("candidate-spec"),
            }
        )
        self.assertTrue(
            any("is not a problem document" in error for error in errors), errors
        )

    def test_rejects_problem_without_required_sections(self) -> None:
        incomplete = PROBLEM.replace("## Constraints", "## Limits")
        errors = self.validate({"docs/problems/causal-gap.md": incomplete})
        self.assertTrue(
            any("missing required section '## Constraints'" in error for error in errors),
            errors,
        )


if __name__ == "__main__":
    unittest.main()
