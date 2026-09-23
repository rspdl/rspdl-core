"""Python SDK for deterministic RSPDL analysis."""

from __future__ import annotations

import json
from importlib.metadata import version
from typing import Any, Mapping, Sequence, TypedDict

from . import _native

WIRE_SCHEMA_VERSION = 1
EDIT_SCHEMA_VERSION = 1
SUPPORTED_LOCALE = "ko-KR"
__version__ = version("rspdl")


class Source(TypedDict):
    """One identified RSPDL source document."""

    path: str
    text: str


class SdkResponse(TypedDict):
    """Versioned SDK response with a compiler-owned result object."""

    schema_version: int
    result: Any


def _decode(response_json: str) -> SdkResponse:
    return json.loads(response_json)


def _encode(request: Mapping[str, Any]) -> str:
    return json.dumps(request, ensure_ascii=False, separators=(",", ":"))


def compile(  # noqa: A001 - public SDK operation name
    sources: Sequence[Source],
    *,
    locale: str = SUPPORTED_LOCALE,
) -> SdkResponse:
    """Compile one or more sources into workspace IR and diagnostics."""

    return _decode(
        _native.compile_json(
            _encode(
                {
                    "schema_version": WIRE_SCHEMA_VERSION,
                    "locale": locale,
                    "sources": list(sources),
                }
            )
        )
    )


def check(
    sources: Sequence[Source],
    data: Mapping[str, Any],
    *,
    locale: str = SUPPORTED_LOCALE,
    timeout_ms: int = 5_000,
) -> SdkResponse:
    """Compile sources and check runtime records, constraints and policies."""

    return _decode(
        _native.check_json(
            _encode(
                {
                    "schema_version": WIRE_SCHEMA_VERSION,
                    "locale": locale,
                    "sources": list(sources),
                    "data": data,
                    "timeout_ms": timeout_ms,
                }
            )
        )
    )


def format(  # noqa: A001 - public SDK operation name
    sources: Sequence[Source],
    *,
    locale: str = SUPPORTED_LOCALE,
) -> SdkResponse:
    """Rewrite sources in canonical form.

    A source that does not parse comes back with ``text`` set to ``None`` and the
    parser's diagnostics. The input is never echoed back as if it had been formatted.
    """

    return _decode(
        _native.format_json(
            _encode(
                {
                    "schema_version": WIRE_SCHEMA_VERSION,
                    "locale": locale,
                    "sources": list(sources),
                }
            )
        )
    )


def find_model(
    source: Source,
    *,
    locale: str = SUPPORTED_LOCALE,
    scope_per_model: int = 3,
    timeout_ms: int = 5_000,
) -> SdkResponse:
    """Find a finite virtual model for one source within an explicit scope."""

    return _decode(
        _native.find_model_json(
            _encode(
                {
                    "schema_version": WIRE_SCHEMA_VERSION,
                    "locale": locale,
                    "source": source,
                    "scope_per_model": scope_per_model,
                    "timeout_ms": timeout_ms,
                }
            )
        )
    )


def edit(request: Mapping[str, Any]) -> dict[str, Any]:
    """Return a compiler-validated candidate source without saving it."""

    return _decode(_native.edit_json(_encode(request)))


def source_hash(text: str) -> str:
    """Fingerprint exact source text as lowercase SHA-256 of its UTF-8 bytes."""

    return _native.source_hash(text)


__all__ = [
    "SUPPORTED_LOCALE",
    "EDIT_SCHEMA_VERSION",
    "WIRE_SCHEMA_VERSION",
    "SdkResponse",
    "Source",
    "__version__",
    "check",
    "compile",
    "find_model",
    "format",
    "edit",
    "source_hash",
]
