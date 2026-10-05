#!/usr/bin/env bash
set -euo pipefail

REPOSITORY_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPOSITORY_ROOT"

cargo build -p rspdl-cli
python3 -m unittest discover -s examples/project-schedule -p 'test_*.py' -v
node --test examples/project-schedule/projection.test.mjs
