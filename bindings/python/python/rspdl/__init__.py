"""Python SDK for deterministic RSPDL analysis."""

from __future__ import annotations

import json
from importlib.metadata import version
from typing import Any, Mapping, Sequence, TypedDict

from . import _native

WIRE_SCHEMA_VERSION = 1
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


def compile(
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


__all__ = [
    "SUPPORTED_LOCALE",
    "WIRE_SCHEMA_VERSION",
    "SdkResponse",
    "Source",
    "__version__",
    "check",
    "compile",
    "find_model",
]
