#!/usr/bin/env bash
set -euo pipefail

REPOSITORY_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPOSITORY_ROOT"

python3 .agents/skills/discover-rspdl-knowledge/scripts/knowledge_index.py validate
python3 -m unittest discover \
  -s .agents/skills/discover-rspdl-knowledge/scripts \
  -p 'test_*.py'
python3 scripts/check-release-metadata.py
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
