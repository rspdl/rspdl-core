#!/usr/bin/env python3
"""Validate immutable release artifacts before a registry repair publish."""

from __future__ import annotations

import base64
import csv
import hashlib
import json
import os
import re
import sys
import urllib.error
import urllib.parse
import urllib.request
import zipfile
from email.parser import Parser
from pathlib import Path
from typing import Any


WORKFLOW_PATH = ".github/workflows/release.yml"
EXPECTED_PYTHON_JOBS = frozenset(
    {
        "Python wheel (aarch64-apple-darwin)",
        "Python wheel (x86_64-apple-darwin)",
        "Python wheel (x86_64-pc-windows-msvc)",
        "Python wheel (x86_64-unknown-linux-gnu)",
    }
)
EXPECTED_PYTHON_ARTIFACTS = frozenset(
    {
        "python-aarch64-apple-darwin",
        "python-x86_64-apple-darwin",
        "python-x86_64-pc-windows-msvc",
        "python-x86_64-unknown-linux-gnu",
    }
)
EXPECTED_WHEEL_TAGS = frozenset(
    {
        "cp311-abi3-macosx_14_0_arm64",
        "cp311-abi3-macosx_14_0_x86_64",
        "cp311-abi3-manylinux_2_28_x86_64",
        "cp311-abi3-win_amd64",
    }
)
VERSION = re.compile(r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\Z")
SHA = re.compile(r"[0-9a-f]{40}\Z")


class RepairValidationError(RuntimeError):
    """The requested release repair cannot prove its artifact provenance."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise RepairValidationError(message)


class GitHubClient:
    def __init__(self, repository: str, token: str) -> None:
        require(
            re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repository) is not None,
            "repository must be an owner/name pair",
        )
        require(bool(token), "GITHUB_TOKEN is required")
        self._api = f"https://api.github.com/repos/{repository}"
        self._token = token

    def get(self, path: str) -> Any:
        request = urllib.request.Request(
            f"{self._api}/{path.lstrip('/')}",
            headers={
                "Accept": "application/vnd.github+json",
                "Authorization": f"Bearer {self._token}",
                "User-Agent": "rspdl-release-repair-validator",
                "X-GitHub-Api-Version": "2022-11-28",
            },
        )
        try:
            with urllib.request.urlopen(request, timeout=30) as response:
                return json.load(response)
        except (urllib.error.HTTPError, urllib.error.URLError, TimeoutError) as error:
            raise RepairValidationError(f"GitHub API request failed for {path}: {error}") from error


def validate_provenance(
    *,
    repository: str,
    run_id: int,
    version: str,
    run: dict[str, Any],
    jobs: list[dict[str, Any]],
    artifacts: list[dict[str, Any]],
    release: dict[str, Any],
    tag_commit: dict[str, Any],
) -> str:
    require(VERSION.fullmatch(version) is not None, "release version must be a stable X.Y.Z version")
    require(run.get("id") == run_id, "workflow run ID does not match the requested run")
    require(run.get("path") == WORKFLOW_PATH, f"workflow run must come from {WORKFLOW_PATH}")
    require(run.get("event") == "push", "artifact workflow run must be triggered by push")
    require(run.get("head_branch") == "main", "artifact workflow run must target main")
    require(
        (run.get("repository") or {}).get("full_name") == repository,
        "artifact workflow run belongs to a different repository",
    )
    require(
        (run.get("head_repository") or {}).get("full_name") == repository,
        "artifact workflow run head belongs to a different repository",
    )
    head_sha = run.get("head_sha")
    require(isinstance(head_sha, str) and SHA.fullmatch(head_sha) is not None, "invalid run head SHA")

    successful_jobs = {
        job.get("name")
        for job in jobs
        if job.get("status") == "completed" and job.get("conclusion") == "success"
    }
    missing_jobs = EXPECTED_PYTHON_JOBS.difference(successful_jobs)
    require(not missing_jobs, f"Python build jobs did not all succeed: {sorted(missing_jobs)}")

    python_artifacts = [
        artifact for artifact in artifacts if str(artifact.get("name", "")).startswith("python-")
    ]
    artifact_names = {artifact.get("name") for artifact in python_artifacts}
    require(
        artifact_names == EXPECTED_PYTHON_ARTIFACTS,
        "Python artifacts differ from the four release targets: "
        f"expected {sorted(EXPECTED_PYTHON_ARTIFACTS)}, got {sorted(artifact_names)}",
    )
    require(
        len(python_artifacts) == len(EXPECTED_PYTHON_ARTIFACTS),
        "Python artifact names are duplicated",
    )
    require(
        all(artifact.get("expired") is False for artifact in python_artifacts),
        "one or more Python artifacts have expired",
    )

    tag_name = f"rspdl-v{version}"
    require(release.get("tag_name") == tag_name, "GitHub release tag does not match requested version")
    require(release.get("draft") is False, "GitHub release is still a draft")
    require(release.get("prerelease") is False, "stable repair version points to a prerelease")
    require(tag_commit.get("sha") == head_sha, "release tag does not point to artifact run head SHA")
    return head_sha


def _verify_record(archive: zipfile.ZipFile, record_name: str) -> None:
    rows = csv.reader(archive.read(record_name).decode("utf-8").splitlines())
    seen: set[str] = set()
    for name, encoded_hash, size in rows:
        require(name not in seen, f"wheel RECORD contains duplicate path: {name}")
        seen.add(name)
        require(name in archive.namelist(), f"wheel RECORD references missing file: {name}")
        data = archive.read(name)
        if name == record_name:
            require(not encoded_hash and not size, "wheel RECORD must not hash itself")
            continue
        require(encoded_hash.startswith("sha256="), f"wheel RECORD lacks sha256 for {name}")
        expected = encoded_hash.removeprefix("sha256=")
        actual = base64.urlsafe_b64encode(hashlib.sha256(data).digest()).rstrip(b"=").decode()
        require(actual == expected, f"wheel RECORD hash mismatch for {name}")
        require(size == str(len(data)), f"wheel RECORD size mismatch for {name}")
    require(seen == set(archive.namelist()), "wheel RECORD does not cover every archive member")


def inspect_wheel(path: Path, version: str) -> str:
    with zipfile.ZipFile(path) as archive:
        require(archive.testzip() is None, f"wheel CRC validation failed: {path.name}")
        metadata_names = [name for name in archive.namelist() if name.endswith(".dist-info/METADATA")]
        wheel_names = [name for name in archive.namelist() if name.endswith(".dist-info/WHEEL")]
        record_names = [name for name in archive.namelist() if name.endswith(".dist-info/RECORD")]
        require(len(metadata_names) == 1, f"wheel must contain one METADATA file: {path.name}")
        require(len(wheel_names) == 1, f"wheel must contain one WHEEL file: {path.name}")
        require(len(record_names) == 1, f"wheel must contain one RECORD file: {path.name}")

        metadata = Parser().parsestr(archive.read(metadata_names[0]).decode("utf-8"))
        wheel = Parser().parsestr(archive.read(wheel_names[0]).decode("utf-8"))
        require(metadata.get("Name") == "rspdl", f"unexpected wheel distribution: {path.name}")
        require(metadata.get("Version") == version, f"wheel version differs from release: {path.name}")
        require(wheel.get("Root-Is-Purelib") == "false", f"wheel must contain a native extension: {path.name}")
        tags = wheel.get_all("Tag", [])
        require(len(tags) == 1, f"wheel must contain exactly one platform tag: {path.name}")
        tag = tags[0]
        expected_name = f"rspdl-{version}-{tag}.whl"
        require(path.name == expected_name, f"wheel filename disagrees with internal tag: {path.name}")
        _verify_record(archive, record_names[0])
        return tag


def validate_wheels(directory: Path, version: str) -> None:
    require(VERSION.fullmatch(version) is not None, "release version must be a stable X.Y.Z version")
    require(directory.is_dir(), f"wheel directory does not exist: {directory}")
    files = sorted(path for path in directory.rglob("*") if path.is_file())
    require(files and all(path.suffix == ".whl" for path in files), "artifact set must contain wheels only")
    require(len(files) == len(EXPECTED_WHEEL_TAGS), "artifact set must contain exactly four wheels")
    tags = {inspect_wheel(path, version) for path in files}
    require(
        tags == EXPECTED_WHEEL_TAGS,
        f"wheel platform coverage differs: expected {sorted(EXPECTED_WHEEL_TAGS)}, got {sorted(tags)}",
    )


def _required_environment(name: str) -> str:
    value = os.environ.get(name, "")
    require(bool(value), f"{name} is required")
    return value


def validate_remote() -> None:
    repository = _required_environment("GITHUB_REPOSITORY")
    token = _required_environment("GITHUB_TOKEN")
    run_text = _required_environment("RSPDL_REPAIR_RUN_ID")
    version = _required_environment("RSPDL_REPAIR_VERSION")
    require(run_text.isascii() and run_text.isdigit(), "artifact run ID must be a positive integer")
    run_id = int(run_text)
    require(run_id > 0, "artifact run ID must be a positive integer")
    tag_name = f"rspdl-v{version}"
    encoded_tag = urllib.parse.quote(tag_name, safe="")

    client = GitHubClient(repository, token)
    run = client.get(f"actions/runs/{run_id}")
    jobs_response = client.get(f"actions/runs/{run_id}/jobs?filter=all&per_page=100")
    artifacts_response = client.get(f"actions/runs/{run_id}/artifacts?per_page=100")
    release = client.get(f"releases/tags/{encoded_tag}")
    tag_commit = client.get(f"commits/{encoded_tag}")
    require(
        jobs_response.get("total_count") == len(jobs_response.get("jobs", [])),
        "workflow has more than 100 jobs; provenance would be incomplete",
    )
    require(
        artifacts_response.get("total_count") == len(artifacts_response.get("artifacts", [])),
        "workflow has more than 100 artifacts; provenance would be incomplete",
    )
    head_sha = validate_provenance(
        repository=repository,
        run_id=run_id,
        version=version,
        run=run,
        jobs=jobs_response["jobs"],
        artifacts=artifacts_response["artifacts"],
        release=release,
        tag_commit=tag_commit,
    )
    print(f"validated release provenance: {tag_name} -> {head_sha}, run {run_id}")


def main() -> None:
    try:
        command = sys.argv[1:]
        if command == ["provenance"]:
            validate_remote()
        elif command == ["wheels"]:
            validate_wheels(
                Path(_required_environment("RSPDL_REPAIR_WHEEL_DIR")),
                _required_environment("RSPDL_REPAIR_VERSION"),
            )
            print("validated four rspdl wheels and their internal platform coverage")
        else:
            raise RepairValidationError("usage: validate_release_repair.py provenance|wheels")
    except RepairValidationError as error:
        raise SystemExit(f"release repair validation failed: {error}") from error


if __name__ == "__main__":
    main()
