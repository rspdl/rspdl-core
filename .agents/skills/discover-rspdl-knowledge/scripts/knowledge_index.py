#!/usr/bin/env python3
"""Front-matter-only discovery for RSPDL repository knowledge."""

from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable


REQUIRED_FIELDS = {
    "id",
    "title",
    "type",
    "status",
    "version",
    "summary",
    "topics",
    "related",
    "last_updated",
}
ALLOWED_TYPES = {"prd", "adr", "rfc", "architecture", "spec", "guide", "index"}
ALLOWED_STATUSES = {
    "draft",
    "proposed",
    "accepted",
    "active",
    "final",
    "superseded",
    "deprecated",
}
RELATION_FIELDS = ("related", "supersedes", "superseded_by")
IGNORED_PARTS = {
    ".git",
    ".agents",
    ".claude",
    "node_modules",
    "vendor",
    "target",
    "build",
    "dist",
}
ID_PATTERN = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")
DATE_PATTERN = re.compile(r"^\d{4}-\d{2}-\d{2}$")


@dataclass(frozen=True)
class Document:
    path: Path
    metadata: dict[str, Any]

    @property
    def id(self) -> str:
        return str(self.metadata["id"])


class FrontMatterError(ValueError):
    pass


def repository_root() -> Path:
    return Path(__file__).resolve().parents[4]


def parse_scalar(raw: str) -> Any:
    value = raw.strip()
    if not value:
        return ""
    if value in {"[]", "[ ]"}:
        return []
    if value.startswith("[") and value.endswith("]"):
        inner = value[1:-1].strip()
        if not inner:
            return []
        return [parse_scalar(part) for part in inner.split(",")]
    if len(value) >= 2 and value[0] == value[-1] and value[0] in {'"', "'"}:
        return value[1:-1]
    lowered = value.lower()
    if lowered == "true":
        return True
    if lowered == "false":
        return False
    if lowered in {"null", "~"}:
        return None
    return value


def read_frontmatter(path: Path) -> dict[str, Any] | None:
    with path.open("r", encoding="utf-8") as handle:
        if handle.readline().rstrip("\r\n") != "---":
            return None

        lines: list[str] = []
        for line_number, line in enumerate(handle, start=2):
            if line.rstrip("\r\n") == "---":
                break
            if line_number > 256:
                raise FrontMatterError("front matter exceeds 255 lines")
            lines.append(line.rstrip("\r\n"))
        else:
            raise FrontMatterError("front matter has no closing delimiter")

    result: dict[str, Any] = {}
    active_list: str | None = None
    for offset, line in enumerate(lines, start=2):
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue

        if line.startswith("  - "):
            if active_list is None:
                raise FrontMatterError(f"line {offset}: list item has no field")
            result[active_list].append(parse_scalar(line[4:]))
            continue

        if line.startswith((" ", "\t")):
            raise FrontMatterError(f"line {offset}: nested mappings are not supported")

        if ":" not in line:
            raise FrontMatterError(f"line {offset}: expected 'key: value'")

        key, raw_value = line.split(":", 1)
        key = key.strip()
        if not re.fullmatch(r"[a-z][a-z0-9_]*", key):
            raise FrontMatterError(f"line {offset}: invalid field name '{key}'")
        if key in result:
            raise FrontMatterError(f"line {offset}: duplicate field '{key}'")

        raw_value = raw_value.strip()
        if raw_value:
            result[key] = parse_scalar(raw_value)
            active_list = None
        else:
            result[key] = []
            active_list = key

    return result


def markdown_paths(root: Path) -> Iterable[Path]:
    for path in sorted(root.rglob("*.md")):
        relative = path.relative_to(root)
        if any(part in IGNORED_PARTS for part in relative.parts):
            continue
        yield path


def load_documents(root: Path, include_index: bool = True) -> tuple[list[Document], list[str]]:
    documents: list[Document] = []
    errors: list[str] = []

    for path in markdown_paths(root):
        relative = path.relative_to(root)
        try:
            metadata = read_frontmatter(path)
        except (OSError, UnicodeError, FrontMatterError) as exc:
            errors.append(f"{relative}: {exc}")
            continue
        if metadata is None:
            if relative.parts and relative.parts[0] == "docs":
                errors.append(f"{relative}: missing YAML front matter")
            continue
        if not include_index and metadata.get("type") == "index":
            continue
        documents.append(Document(relative, metadata))

    return documents, errors


def as_list(value: Any) -> list[str]:
    if value is None:
        return []
    if isinstance(value, list):
        return [str(item) for item in value]
    return [str(value)]


def validate_documents(documents: list[Document], initial_errors: list[str]) -> list[str]:
    errors = list(initial_errors)
    by_id: dict[str, Document] = {}

    for document in documents:
        metadata = document.metadata
        missing = sorted(REQUIRED_FIELDS - metadata.keys())
        if missing:
            errors.append(f"{document.path}: missing fields: {', '.join(missing)}")
            continue

        document_id = document.id
        if not ID_PATTERN.fullmatch(document_id):
            errors.append(f"{document.path}: invalid id '{document_id}'")
        if document_id in by_id:
            errors.append(
                f"{document.path}: duplicate id '{document_id}' also used by {by_id[document_id].path}"
            )
        else:
            by_id[document_id] = document

        if metadata["type"] not in ALLOWED_TYPES:
            errors.append(f"{document.path}: unsupported type '{metadata['type']}'")
        if metadata["status"] not in ALLOWED_STATUSES:
            errors.append(f"{document.path}: unsupported status '{metadata['status']}'")
        if not isinstance(metadata["topics"], list):
            errors.append(f"{document.path}: topics must be a list")
        if not isinstance(metadata["related"], list):
            errors.append(f"{document.path}: related must be a list")
        if not DATE_PATTERN.fullmatch(str(metadata["last_updated"])):
            errors.append(f"{document.path}: last_updated must be YYYY-MM-DD")

    known_ids = set(by_id)
    for document in documents:
        for field in RELATION_FIELDS:
            for target in as_list(document.metadata.get(field)):
                if target and target not in known_ids:
                    errors.append(f"{document.path}: {field} references unknown id '{target}'")

    return errors


def document_text(document: Document) -> str:
    metadata = document.metadata
    topics = " ".join(as_list(metadata.get("topics")))
    return " ".join(
        [
            document.id,
            str(metadata.get("title", "")),
            str(metadata.get("summary", "")),
            topics,
            str(metadata.get("type", "")),
            str(metadata.get("status", "")),
        ]
    ).lower()


def print_document(document: Document, score: int | None = None) -> None:
    metadata = document.metadata
    prefix = f"[{score}] " if score is not None else ""
    topics = ", ".join(as_list(metadata.get("topics")))
    print(f"{prefix}{document.id} ({metadata.get('type')}, {metadata.get('status')})")
    print(f"  path: {document.path}")
    print(f"  title: {metadata.get('title')}")
    print(f"  summary: {metadata.get('summary')}")
    print(f"  topics: {topics or '-'}")
    relations = as_list(metadata.get("related"))
    if relations:
        print(f"  related: {', '.join(relations)}")


def find_document(documents: list[Document], identifier: str) -> Document:
    for document in documents:
        if document.id == identifier or str(document.path) == identifier:
            return document
    raise KeyError(identifier)


def command_catalog(documents: list[Document], json_output: bool) -> None:
    if json_output:
        payload = [
            {"path": str(document.path), **document.metadata}
            for document in sorted(documents, key=lambda item: item.id)
        ]
        print(json.dumps(payload, ensure_ascii=False, indent=2))
        return
    for document in sorted(documents, key=lambda item: item.id):
        print_document(document)


def command_query(documents: list[Document], query: str, limit: int) -> None:
    terms = [term for term in re.split(r"\s+", query.lower().strip()) if term]
    ranked: list[tuple[int, Document]] = []
    for document in documents:
        metadata = document.metadata
        haystack = document_text(document)
        score = 0
        for term in terms:
            if term in document.id.lower():
                score += 8
            if term in str(metadata.get("title", "")).lower():
                score += 5
            if term in " ".join(as_list(metadata.get("topics"))).lower():
                score += 4
            if term in str(metadata.get("summary", "")).lower():
                score += 2
            if term in haystack:
                score += 1
        if score:
            ranked.append((score, document))

    ranked.sort(key=lambda item: (-item[0], item[1].id))
    if not ranked:
        print("No metadata matches. Run 'catalog' or try broader terms.")
        return
    for score, document in ranked[:limit]:
        print_document(document, score)


def command_graph(documents: list[Document], identifier: str) -> None:
    target = find_document(documents, identifier)
    outgoing: list[tuple[str, str]] = []
    for field in RELATION_FIELDS:
        outgoing.extend((field, item) for item in as_list(target.metadata.get(field)) if item)

    incoming: list[tuple[str, str]] = []
    for document in documents:
        if document.id == target.id:
            continue
        for field in RELATION_FIELDS:
            if target.id in as_list(document.metadata.get(field)):
                incoming.append((f"backlink:{field}", document.id))

    print_document(target)
    print("  edges:")
    if not outgoing and not incoming:
        print("    - none")
        return
    for relation, document_id in sorted(outgoing + incoming):
        print(f"    - {relation} -> {document_id}")


def command_outline(root: Path, documents: list[Document], identifier: str) -> None:
    document = find_document(documents, identifier)
    path = root / document.path
    found = False
    with path.open("r", encoding="utf-8") as handle:
        for line_number, line in enumerate(handle, start=1):
            if re.match(r"^#{1,6}\s+\S", line):
                print(f"{line_number}: {line.rstrip()}")
                found = True
    if not found:
        print("No Markdown headings found.")


def yaml_scalar(value: Any) -> str:
    return json.dumps(str(value), ensure_ascii=False)


def render_index(documents: list[Document]) -> str:
    ordered = sorted(documents, key=lambda item: (str(item.metadata.get("type")), item.id))
    last_updated = max(
        (str(document.metadata.get("last_updated", "1970-01-01")) for document in ordered),
        default="1970-01-01",
    )
    related = [document.id for document in ordered]
    lines = [
        "---",
        "id: rspdl-knowledge-index",
        "title: RSPDL Knowledge Index",
        "type: index",
        "status: active",
        'version: "1"',
        "summary: Generated metadata catalog for progressively discovering RSPDL repository knowledge.",
        "topics:",
        "  - knowledge-navigation",
        "  - document-index",
        "related:",
    ]
    lines.extend(f"  - {item}" for item in related)
    lines.extend(
        [
            f'last_updated: "{last_updated}"',
            "owners:",
            "  - rspdl-maintainers",
            "---",
            "",
            "# RSPDL Knowledge Index",
            "",
            "> Generated from document front matter. Run the knowledge skill's `build` command; do not edit entries manually.",
            "",
            "| ID | Type | Status | Document | Summary | Topics |",
            "| --- | --- | --- | --- | --- | --- | --- |",
        ]
    )
    for document in ordered:
        metadata = document.metadata
        topics = ", ".join(f"`{topic}`" for topic in as_list(metadata.get("topics")))
        if document.path.parts and document.path.parts[0] == "docs":
            path = Path(*document.path.parts[1:]).as_posix()
        else:
            path = (Path("..") / document.path).as_posix()
        title = str(metadata.get("title", "")).replace("|", "\\|")
        summary = str(metadata.get("summary", "")).replace("|", "\\|")
        lines.append(
            f"| `{document.id}` | `{metadata.get('type')}` | `{metadata.get('status')}` "
            f"| [{title}]({path}) | {summary} | {topics} |"
        )
    lines.append("")
    return "\n".join(lines)


def command_build(root: Path, documents: list[Document]) -> None:
    destination = root / "docs" / "index.md"
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(render_index(documents), encoding="utf-8")
    print(f"Wrote {destination.relative_to(root)} with {len(documents)} documents.")


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=repository_root())
    subparsers = parser.add_subparsers(dest="command", required=True)

    catalog = subparsers.add_parser("catalog", help="List indexed metadata.")
    catalog.add_argument("--json", action="store_true", dest="json_output")

    query = subparsers.add_parser("query", help="Search front matter metadata.")
    query.add_argument("text")
    query.add_argument("--limit", type=int, default=8)

    graph = subparsers.add_parser("graph", help="Show document relations.")
    graph.add_argument("identifier")

    outline = subparsers.add_parser("outline", help="Show document headings.")
    outline.add_argument("identifier")

    subparsers.add_parser("validate", help="Validate metadata and relations.")
    subparsers.add_parser("build", help="Regenerate docs/index.md.")
    return parser


def main() -> int:
    args = build_parser().parse_args()
    root = args.root.resolve()
    include_index = args.command != "build"
    documents, load_errors = load_documents(root, include_index=include_index)
    errors = validate_documents(documents, load_errors)

    if args.command == "validate":
        source_documents = [
            document for document in documents if document.metadata.get("type") != "index"
        ]
        index_path = root / "docs" / "index.md"
        if not index_path.exists():
            errors.append("docs/index.md: generated knowledge index is missing")
        elif index_path.read_text(encoding="utf-8") != render_index(source_documents):
            errors.append("docs/index.md: generated knowledge index is stale; run 'build'")
        if errors:
            for error in errors:
                print(f"ERROR: {error}", file=sys.stderr)
            return 1
        print(f"Validated {len(documents)} documents.")
        return 0

    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        print("Fix metadata errors before discovery or index generation.", file=sys.stderr)
        return 1

    try:
        if args.command == "catalog":
            command_catalog(documents, args.json_output)
        elif args.command == "query":
            command_query(documents, args.text, args.limit)
        elif args.command == "graph":
            command_graph(documents, args.identifier)
        elif args.command == "outline":
            command_outline(root, documents, args.identifier)
        elif args.command == "build":
            command_build(root, documents)
    except KeyError as exc:
        print(f"Unknown document: {exc.args[0]}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
