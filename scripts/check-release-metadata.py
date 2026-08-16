#!/usr/bin/env python3
"""Fail when the coordinated Rust, Python, and Node.js release metadata drifts."""

from __future__ import annotations

import json
import tomllib
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
REPOSITORY = "https://github.com/rspdl/rspdl-core"
NODE_TARGETS = {
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
    "x86_64-pc-windows-msvc",
    "x86_64-unknown-linux-gnu",
}


def read_toml(path: Path) -> dict:
    with path.open("rb") as source:
        return tomllib.load(source)


def read_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


workspace = read_toml(ROOT / "Cargo.toml")
workspace_package = workspace["workspace"]["package"]
version = workspace_package["version"]

require((ROOT / "version.txt").read_text(encoding="utf-8").strip() == version, "version.txt and Cargo differ")
require(workspace_package["license"] == "Apache-2.0", "workspace license must be Apache-2.0")
require(workspace_package["repository"] == REPOSITORY, "workspace repository metadata drifted")

for member in workspace["workspace"]["members"]:
    manifest_path = ROOT / member / "Cargo.toml"
    package = read_toml(manifest_path)["package"]
    require(package["version"].get("workspace") is True, f"{member}: version must inherit workspace")
    require(package["license"].get("workspace") is True, f"{member}: license must inherit workspace")
    require(package["repository"].get("workspace") is True, f"{member}: repository must inherit workspace")
    require(package.get("publish") is False, f"{member}: internal Cargo package must not publish")

python_project = read_toml(ROOT / "pyproject.toml")["project"]
require(python_project["name"] == "rspdl", "PyPI package name must be rspdl")
require(python_project["license"] == "Apache-2.0", "Python package license drifted")
require("version" in python_project["dynamic"], "Python version must come from Cargo metadata")
require(python_project["urls"]["Repository"] == REPOSITORY, "Python repository metadata drifted")

node = read_json(ROOT / "bindings/node/package.json")
lock = read_json(ROOT / "bindings/node/package-lock.json")
require(node["name"] == "rspdl", "npm package name must be rspdl")
require(node["version"] == version, "npm and Cargo versions differ")
require(lock["version"] == version, "npm lockfile version differs")
require(lock["packages"][""]["version"] == version, "npm root lock entry version differs")
require(node["license"] == "Apache-2.0", "Node.js package license drifted")
require(node["repository"]["url"] == f"git+{REPOSITORY}.git", "Node.js repository metadata drifted")
require(set(node["napi"]["targets"]) == NODE_TARGETS, "Node.js release target matrix drifted")

release_manifest = read_json(ROOT / ".release-please-manifest.json")
release_config = read_json(ROOT / "release-please-config.json")
if "." in release_manifest:
    require(release_manifest["."] == version, "Release Please and Cargo versions differ")
else:
    require(
        release_config.get("initial-version") == version,
        "Release Please bootstrap version and Cargo differ",
    )
    require(
        bool(release_config.get("bootstrap-sha")),
        "Release Please bootstrap must bound the initial changelog history",
    )

lockfile = read_toml(ROOT / "Cargo.lock")
for package in lockfile["package"]:
    if package["name"].startswith("rspdl-"):
        require(package["version"] == version, f"Cargo.lock version differs for {package['name']}")

license_report = (ROOT / "THIRD_PARTY_LICENSES.html").read_text(encoding="utf-8")
for dependency in ("z3 ", "z3-src ", "napi ", "pyo3 "):
    require(dependency in license_report, f"third-party license report is missing {dependency.strip()}")

print(f"release metadata is synchronized at {version}")
