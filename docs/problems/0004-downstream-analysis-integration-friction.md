---
id: downstream-analysis-integration-friction
title: Downstream Analysis Integration Friction
type: problem
status: active
created: 2026-08-16
version: "1"
summary: Applications outside Rust must rebuild process, serialization, and release glue before they can consume the same RSPDL analysis contract.
topics:
  - downstream-integration
  - language-sdk
  - package-distribution
  - analysis-contract
related:
  - rspdl-product-vision
  - core-application-boundary
  - rspdl-compiler-architecture
last_updated: "2026-08-16"
owners:
  - rspdl-maintainers
---

# Downstream Analysis Integration Friction

## Why

- Python과 Node.js 애플리케이션 개발자는 RSPDL 분석을 사용하기 전에 Rust workspace를 직접 빌드하거나 CLI subprocess와 파일 I/O를 감싸야 한다.
- 각 애플리케이션이 JSON 입력, 오류 분류, timeout과 결과 parsing을 다시 구현하면 같은 compiler 결과를 사용하면서도 통합마다 동작이 달라진다.
- 플랫폼별 native build와 package registry 배포를 소비자에게 넘기면 설치 실패를 제품 통합 코드가 떠안고, 분석 도입 전에 Rust·CMake·Z3 환경을 먼저 맞춰야 한다.

## What

- 원인은 Locale 중립 compiler 결과가 없어서가 아니라, 그 결과를 다른 runtime에 같은 버전 계약과 설치 가능한 artifact로 전달하는 경계가 없다는 점이다.
- 새 Locale 문법을 추가하는 일과 Python·Node.js에서 기존 `ko-KR` 분석을 호출하는 일은 구분한다.
- 제품별 정책표, 검색, 집계와 UI projection을 원하는 것은 application 책임이며 이 문제에 포함하지 않는다.
- 같은 source와 option이 언어별 wrapper에서 다른 JSON을 만들거나, package 설치에 local Rust toolchain이 필요하면 문제가 존재한다.

## How

- 지원 runtime은 package manager만으로 플랫폼용 artifact를 설치하고 같은 versioned request/response 계약을 호출할 수 있어야 한다.
- 정상 사례는 Python과 Node.js가 같은 source workspace를 compile해 같은 Canonical IR과 structured diagnostics를 반환하는 것이다.
- 실패 사례는 syntax·semantic·runtime 오류를 wrapper exception으로 바꾸지 않고 compiler report에 그대로 보존하는 것이다.
- 경계 사례는 복수 source 순서를 바꿔도 같은 canonical 결과를 반환하고 bounded model의 `UNKNOWN`과 `UNSUPPORTED`를 성공으로 바꾸지 않는 것이다.
- 오탐 방지 사례는 지원하지 않는 Locale·schema version·잘못된 SDK option만 binding error로 분류하고 유효한 compiler finding은 정상 응답으로 반환하는 것이다.

## Constraints

- SDK는 표현되지 않은 정책, lifecycle 동작 또는 UI projection을 추측하지 않는다.
- Locale AST, token stream과 사람용 번역 문자열을 cross-language 호환 계약으로 만들지 않는다.
- package version과 wire schema version을 같은 것으로 취급하지 않는다.
- native dependency의 license notice와 unsupported platform을 누락한 채 설치 성공을 약속하지 않는다.
- 첫 범위에는 browser WASM, Bun, Deno, PyPy, Alpine/musl과 새 Locale frontend를 포함하지 않는다.

## References

- [RSPDL Product Vision](../product/vision.md)
- [Core와 Application Projection 경계](../adr/0002-core-application-boundary.md)
- [RSPDL Compiler Architecture](../architecture.md)
