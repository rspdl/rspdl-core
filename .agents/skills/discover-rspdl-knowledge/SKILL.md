---
name: discover-rspdl-knowledge
description: Discover and navigate RSPDL repository knowledge from Markdown YAML front matter and document relationships without loading whole files. Use when an agent needs project context, requirements, decisions, specifications, architecture rationale, related documents, or when creating and validating repository knowledge documents.
---

# Discover RSPDL Knowledge

Use document metadata as a map and document bodies as the source of truth. Load context progressively.

## Read workflow

Run the helper from anywhere inside the repository:

```bash
python3 "$(git rev-parse --show-toplevel)/.agents/skills/discover-rspdl-knowledge/scripts/knowledge_index.py" query "<keywords>"
```

Then narrow context in this order:

1. Run `query "<keywords>"` to inspect matching metadata only.
2. Run `graph <document-id>` to inspect related documents and backlinks.
3. Run `outline <document-id>` to inspect headings and line numbers.
4. Read only the relevant section of the selected document.
5. Read the full document only when its complete contract is required.

Do not answer substantive questions from front matter summaries alone.

## Available commands

```bash
python3 <script> catalog
python3 <script> query "semantic ir"
python3 <script> graph rspdl-language-prd
python3 <script> outline rspdl-language-prd
python3 <script> validate
python3 <script> build
```

- `catalog`: list indexed documents without bodies.
- `query`: rank documents using IDs, titles, summaries, and topics.
- `graph`: show outgoing relations and computed backlinks.
- `outline`: show headings with source line numbers.
- `validate`: check front matter, IDs, relations, and controlled values.
- `build`: regenerate `docs/index.md` from document front matter.

## Write workflow

When creating or changing a knowledge document:

1. Read [references/frontmatter-schema.md](references/frontmatter-schema.md).
2. Add or update its YAML front matter.
3. Use stable document IDs; never reuse an ID for a different concept.
4. Add `related` links only when they improve discovery. Backlinks are computed automatically.
5. Keep `summary` factual and limited to one line.
6. Run `build`, then `validate`.
7. Review the generated index diff with the document diff.

Treat generated indexes as rebuildable navigation aids, not authoritative content.
