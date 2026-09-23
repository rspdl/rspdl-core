from __future__ import annotations

import base64
import csv
import hashlib
import io
import tempfile
import unittest
import zipfile
from pathlib import Path

from validate_release_repair import (
    EXPECTED_PYTHON_ARTIFACTS,
    EXPECTED_PYTHON_JOBS,
    EXPECTED_WHEEL_TAGS,
    RepairValidationError,
    validate_provenance,
    validate_wheels,
)


VERSION = "0.1.4"
RUN_ID = 35880097570
HEAD_SHA = "5" * 40


def provenance() -> dict:
    return {
        "repository": "rspdl/rspdl-core",
        "run_id": RUN_ID,
        "version": VERSION,
        "run": {
            "id": RUN_ID,
            "path": ".github/workflows/release.yml",
            "event": "push",
            "head_branch": "main",
            "head_sha": HEAD_SHA,
            "repository": {"full_name": "rspdl/rspdl-core"},
            "head_repository": {"full_name": "rspdl/rspdl-core"},
        },
        "jobs": [
            {"name": name, "status": "completed", "conclusion": "success"}
            for name in EXPECTED_PYTHON_JOBS
        ],
        "artifacts": [
            {"name": name, "expired": False} for name in EXPECTED_PYTHON_ARTIFACTS
        ],
        "release": {"tag_name": f"rspdl-v{VERSION}", "draft": False, "prerelease": False},
        "tag_commit": {"sha": HEAD_SHA},
    }


def write_wheel(
    path: Path,
    tag: str,
    *,
    metadata_version: str = VERSION,
    corrupt_record: bool = False,
) -> None:
    files = {
        "rspdl/__init__.py": b"",
        "rspdl-0.1.4.dist-info/METADATA": (
            f"Metadata-Version: 2.4\nName: rspdl\nVersion: {metadata_version}\n"
        ).encode(),
        "rspdl-0.1.4.dist-info/WHEEL": (
            f"Wheel-Version: 1.0\nRoot-Is-Purelib: false\nTag: {tag}\n"
        ).encode(),
    }
    record = io.StringIO()
    writer = csv.writer(record, lineterminator="\n")
    for index, (name, data) in enumerate(files.items()):
        digest = base64.urlsafe_b64encode(hashlib.sha256(data).digest()).rstrip(b"=").decode()
        if corrupt_record and index == 0:
            digest = "invalid"
        writer.writerow((name, f"sha256={digest}", len(data)))
    record_name = "rspdl-0.1.4.dist-info/RECORD"
    writer.writerow((record_name, "", ""))
    files[record_name] = record.getvalue().encode()
    with zipfile.ZipFile(path, "w") as archive:
        for name, data in files.items():
            archive.writestr(name, data)


class ProvenanceTests(unittest.TestCase):
    def test_accepts_matching_release_run_without_requiring_run_completion(self) -> None:
        values = provenance()
        values["run"]["status"] = "waiting"

        self.assertEqual(validate_provenance(**values), HEAD_SHA)

    def test_rejects_non_release_workflow(self) -> None:
        values = provenance()
        values["run"]["path"] = ".github/workflows/verify.yml"

        with self.assertRaisesRegex(RepairValidationError, "must come from"):
            validate_provenance(**values)

    def test_rejects_tag_that_does_not_point_to_artifact_head(self) -> None:
        values = provenance()
        values["tag_commit"]["sha"] = "6" * 40

        with self.assertRaisesRegex(RepairValidationError, "does not point"):
            validate_provenance(**values)

    def test_rejects_missing_successful_python_build(self) -> None:
        values = provenance()
        values["jobs"].pop()

        with self.assertRaisesRegex(RepairValidationError, "did not all succeed"):
            validate_provenance(**values)

    def test_rejects_extra_python_artifact(self) -> None:
        values = provenance()
        values["artifacts"].append({"name": "python-unexpected", "expired": False})

        with self.assertRaisesRegex(RepairValidationError, "differ"):
            validate_provenance(**values)


class WheelTests(unittest.TestCase):
    def test_accepts_exact_native_platform_coverage(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for tag in EXPECTED_WHEEL_TAGS:
                write_wheel(root / f"rspdl-{VERSION}-{tag}.whl", tag)

            validate_wheels(root, VERSION)

    def test_rejects_internal_version_mismatch(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            tags = iter(EXPECTED_WHEEL_TAGS)
            first = next(tags)
            write_wheel(root / f"rspdl-{VERSION}-{first}.whl", first, metadata_version="0.1.3")
            for tag in tags:
                write_wheel(root / f"rspdl-{VERSION}-{tag}.whl", tag)

            with self.assertRaisesRegex(RepairValidationError, "version differs"):
                validate_wheels(root, VERSION)

    def test_rejects_record_tampering(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for index, tag in enumerate(EXPECTED_WHEEL_TAGS):
                path = root / f"rspdl-{VERSION}-{tag}.whl"
                write_wheel(path, tag, corrupt_record=index == 0)

            with self.assertRaises(RepairValidationError):
                validate_wheels(root, VERSION)


class WorkflowTests(unittest.TestCase):
    def test_pypi_repair_publishes_only_the_validated_artifact(self) -> None:
        workflow = (Path(__file__).parents[1] / ".github/workflows/release.yml").read_text()
        validation = workflow.split("  validate-pypi-repair:\n", 1)[1].split(
            "  repair-pypi:\n", 1
        )[0]
        publication = workflow.split("  repair-pypi:\n", 1)[1].split("  repair-npm:\n", 1)[0]

        self.assertNotIn("environment: pypi", validation)
        self.assertIn("name: validated-python-wheels", validation)
        self.assertIn("needs: validate-pypi-repair", publication)
        self.assertIn("environment: pypi", publication)
        self.assertIn("name: validated-python-wheels", publication)
        self.assertIn("skip-existing: true", publication)

    def test_pypi_repair_does_not_share_stuck_release_concurrency(self) -> None:
        workflow = (Path(__file__).parents[1] / ".github/workflows/release.yml").read_text()

        self.assertIn("'release-repair-pypi' || 'release-main'", workflow)
        self.assertIn("default: npm", workflow)
        self.assertIn(
            "if: github.event_name == 'workflow_dispatch' && inputs.repair_target == 'npm'",
            workflow,
        )


if __name__ == "__main__":
    unittest.main()
