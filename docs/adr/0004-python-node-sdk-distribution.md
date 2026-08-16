---
id: python-node-sdk-distribution
title: Python and Node.js SDK Distribution
type: adr
status: accepted
version: "1"
summary: Selects a shared JSON SDK facade with PyO3 and napi-rs native packages plus OIDC-based coordinated releases.
topics:
  - python
  - nodejs
  - sdk
  - native-bindings
  - package-release
related:
  - rspdl-compiler-architecture
  - core-application-boundary
problem_refs:
  - downstream-analysis-integration-friction
last_updated: "2026-08-16"
owners:
  - rspdl-maintainers
---

# Python and Node.js SDK Distribution

## 상태

Accepted.

이 결정은 기존 문법, Canonical IR과 diagnostic 의미를 바꾸지 않는다. `rspdl-compiler`의 공개 결과를 Python과 Node.js에 같은 wire contract로 전달하고 설치·릴리스 책임을 repository가 맡는다.

## 배경

`rspdl-compiler`는 `compile`, `check`와 bounded model finding 결과를 결정적으로 직렬화한다. 다른 runtime이 이를 사용하려면 현재 Rust를 직접 link하거나 CLI를 실행하고 파일·exit code·JSON parsing을 각자 구현해야 한다.

CLI subprocess wrapper는 구현은 작지만 프로세스 시작 비용, binary 탐색, signal과 stderr 계약을 SDK마다 다시 만든다. Browser WASM은 vendored native Z3 backend와 현재 dependency graph를 분리해야 하므로 첫 배포 범위보다 크다.

## 결정

### 공통 SDK facade

`rspdl-sdk` crate가 JSON request를 검증하고 `rspdl-compiler`를 호출한 뒤 versioned JSON response를 반환한다. Python과 Node.js native binding은 이 crate의 문자열 함수만 호출한다.

- wire schema version은 package SemVer와 독립된 정수다.
- `compile`과 `check`는 항상 source 배열을 받아 단일·복수 source에서 같은 workspace result shape을 반환한다.
- `find_model`은 현재 compiler 계약에 맞춰 source 하나, scope와 timeout을 받는다.
- source·semantic·runtime diagnostic과 model `UNKNOWN` 또는 `UNSUPPORTED`는 정상 response에 남긴다.
- malformed JSON, unsupported schema/Locale와 invalid SDK option만 binding error다.
- response는 구조체와 정렬된 collection에서 직렬화하고 wrapper가 필드를 재작성하지 않는다.

### Python package

PyPI package 이름은 `rspdl`이다. PyO3의 CPython stable ABI를 사용해 Python 3.11 이상용 wheel을 만들고, 공개 Python 함수는 native JSON 함수를 호출해 `dict`를 반환한다. Native 작업 중 GIL을 해제한다.

첫 지원 플랫폼은 Linux x86_64 glibc, macOS 14 이상 x86_64·arm64와 Windows x86_64다. Linux Python wheel은 manylinux 2.28을 기준으로 한다. Source distribution만으로 소비자에게 Rust·CMake·Z3 build를 강제하지 않으며 지원 플랫폼에는 wheel을 먼저 제공한다.

### Node.js package

npm package 이름은 `rspdl`이다. napi-rs와 Node-API를 사용하고 TypeScript declaration, ESM과 CommonJS entry point를 제공한다. 공개 분석 함수는 event loop를 막지 않는 Promise를 반환한다.

첫 지원 runtime은 Node.js 22와 24이며 플랫폼은 Python package와 같다. OS·CPU별 native optional package를 먼저 만들고 root package가 설치 환경에 맞는 artifact를 선택한다.

### Version과 release

Repository와 두 package는 `0.x` 동안 하나의 SemVer를 공유한다. Conventional Commit을 입력으로 release-please가 Release PR, changelog와 version 변경을 만든다.

일반 `main` merge는 Release PR만 갱신한다. Release PR merge로 GitHub Release가 만들어진 같은 workflow run에서 모든 native artifact를 먼저 빌드·설치 검증하고 PyPI와 npm에 순서대로 배포한다. Registry 인증은 GitHub-hosted runner의 OIDC Trusted Publishing을 정상 상태로 사용하고 장기 publish token을 두지 않는다. PyPI의 새 project는 pending publisher로 첫 release부터 OIDC를 사용한다. npm은 package가 존재해야 Trusted Publisher를 등록할 수 있으므로 최초 한 번만 root와 platform package 이름을 만드는 제한된 bootstrap token을 허용하고, 즉시 모든 package를 `release.yml`의 `npm` environment에 연결한 뒤 secret을 삭제한다.

### License와 공급망

RSPDL source와 package는 Apache License 2.0으로 배포한다. 모든 Cargo package metadata에 SPDX expression을 기록하고 internal crate는 crates.io publish를 막는다.

CI는 cargo-deny로 dependency license와 source registry를 검사한다. 허용된 permissive license만 dependency graph에 들어올 수 있으며 예외는 crate별 이유와 함께 추가한다. Binary package에는 cargo-about으로 생성한 dependency license text를 project license와 함께 포함한다.

Release workflow의 third-party action은 commit SHA로 고정하고 publish job에만 `id-token: write`를 부여한다. Build artifact를 publish job에서 다시 compile하지 않는다.

## 테스트 계약

- `rspdl-sdk` unit test는 정상 compile/check/model, malformed request, unsupported schema·Locale, empty source와 invalid timeout/scope를 검증한다.
- 정상, compiler failure, 복수 source 경계와 finding 오탐 방지 fixture를 Python과 Node.js package smoke test에서 실행한다.
- 같은 request에 대한 Python·Node.js response JSON은 semantic field와 순서가 같다.
- wheel과 npm tarball을 각각 빈 환경에 설치한 뒤 import, ESM, CommonJS와 분석 호출을 검증한다.
- release 전 version 일치, license report 포함과 package metadata를 검사한다.

## 비용과 비범위

- Z3를 정적으로 포함한 native artifact는 pure-language package보다 크고 플랫폼별 build 시간이 든다.
- npm의 복수 native package publish는 원자적이지 않으므로 root package는 모든 platform artifact가 준비된 뒤 마지막에 배포한다.
- vendored Z3 4.16의 C++20 standard library 요구 때문에 첫 macOS artifact의 deployment target은 14.0이다.
- 첫 범위에는 browser WASM, Alpine/musl, Linux arm64, PyPy, free-threaded CPython, Bun과 Deno를 포함하지 않는다.
- parse CST/AST, formatter, 사람용 diagnostic 번역과 application projection은 첫 SDK API에 노출하지 않는다.

## References

- [Downstream Analysis Integration Friction](../problems/0004-downstream-analysis-integration-friction.md)
- [RSPDL Compiler Architecture](../architecture.md)
- [Core와 Application Projection 경계](0002-core-application-boundary.md)
