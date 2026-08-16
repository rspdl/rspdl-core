---
id: package-release-guide
title: Python and Node.js Package Release Guide
type: guide
status: active
version: "1"
summary: Configures Release Please, PyPI and npm so a reviewed Release PR merge publishes tested native packages.
topics:
  - release
  - pypi
  - npm
  - trusted-publishing
  - native-package
related:
  - python-node-sdk-distribution
  - rspdl-compiler-architecture
problem_refs:
  - downstream-analysis-integration-friction
last_updated: "2026-08-16"
owners:
  - rspdl-maintainers
---

# Python and Node.js Package Release Guide

## Release contract

`main`의 일반 변경은 `release-please`가 관리하는 Release PR만 갱신한다. Maintainer가 그 PR의 version, changelog와 CI 결과를 검토해 merge하면 같은 `release.yml` 실행이 GitHub Release를 만들고 다음 순서로 배포한다.

1. Python과 Node.js의 Linux x64, macOS x64·arm64, Windows x64 artifact를 각각 빌드한다.
2. 각 artifact를 해당 runner에서 import/load하고 SDK smoke test를 실행한다.
3. 네 Python wheel을 PyPI에 게시한다.
4. 네 npm platform package를 게시한 뒤 마지막으로 `rspdl` root package를 게시한다.

최초 bootstrap에서는 `.release-please-manifest.json`을 비워 두고 `initial-version`을 `0.1.0`으로 고정한다. `bootstrap-sha`는 SDK 패키징 작업 직전의 `main` commit이므로 첫 Release PR에는 이번 공개 패키징 변경부터 포함된다. 첫 Release PR이 merge되면 Release Please가 manifest에 `0.1.0`을 기록하며, 이후에는 그 값을 마지막 배포 버전으로 사용한다.

Build job에는 registry credential이 없고 publish job만 `id-token: write`를 가진다. npm의 여러 package publish는 원자적이지 않지만, root package를 마지막에 게시해 불완전한 version을 일반 설치 경로에 노출하지 않는다. 실패한 release는 같은 source와 artifact로 재실행하며 이미 게시된 Python wheel은 건너뛴다. 같은 version의 binary를 다시 빌드해 교체하지 않는다.

## GitHub one-time setup

- Repository 이름과 metadata를 `rspdl/rspdl-core`로 맞춘다.
- `pypi`와 `npm` GitHub Environment를 만들고 deployment branch를 `main`으로 제한한다. 완전 자동 게시를 원하면 required reviewer는 두지 않는다.
- Release Please PR도 일반 CI를 실행하게 하려면 fine-grained token 또는 GitHub App token을 `RELEASE_PLEASE_TOKEN` secret으로 등록한다. 권한은 repository Contents와 Pull requests write로 제한한다. Secret이 없으면 workflow는 기본 `GITHUB_TOKEN`을 사용하지만, 이 token이 갱신한 PR은 별도 CI 실행을 시작하지 않을 수 있다.
- Branch protection은 `Verify`의 core, license, Python SDK와 Node.js SDK job을 요구한다.

## PyPI Trusted Publisher

PyPI에서 아직 `rspdl` project가 없어도 pending publisher를 먼저 만들 수 있다.

- PyPI project: `rspdl`
- Owner: `rspdl`
- Repository: `rspdl-core`
- Workflow: `release.yml`
- Environment: `pypi`

Workflow는 API token 없이 `pypa/gh-action-pypi-publish`와 OIDC로 wheel만 게시한다. 지원하지 않는 플랫폼에서 local Rust/Z3 build로 우회하지 않도록 첫 범위에서는 source distribution을 게시하지 않는다.

## npm first-release bootstrap

npm은 Trusted Publisher를 설정할 package가 먼저 존재해야 한다. 현재 `rspdl`과 다음 napi-rs platform package 이름이 비어 있는지 첫 release 직전에 다시 확인한다.

- `rspdl`
- `rspdl-linux-x64-gnu`
- `rspdl-darwin-x64`
- `rspdl-darwin-arm64`
- `rspdl-win32-x64-msvc`

첫 release에만 이 이름들을 만들 수 있는 최소 범위의 npm token을 GitHub `npm` Environment의 `NPM_BOOTSTRAP_TOKEN` secret으로 둔다. Release PR merge가 다섯 package를 게시한 직후 npm 11.15 이상과 2FA가 설정된 maintainer session에서 각 package를 같은 publisher에 연결한다.

```console
for package in \
  rspdl \
  rspdl-linux-x64-gnu \
  rspdl-darwin-x64 \
  rspdl-darwin-arm64 \
  rspdl-win32-x64-msvc
do
  npm trust github "$package" \
    --repo rspdl/rspdl-core \
    --file release.yml \
    --env npm \
    --allow-publish \
    --yes
done
```

연결을 `npm trust list <package>`로 확인한 뒤 `NPM_BOOTSTRAP_TOKEN`을 GitHub와 npm에서 모두 폐기한다. 이후 workflow에 고정한 npm CLI 11.15.0은 OIDC를 자동 감지하며 public repository의 provenance attestation도 함께 생성한다.

## Release PR review checklist

- 최초 release 전에는 `initial-version`, 이후에는 `.release-please-manifest.json`의 version이 Cargo workspace, npm `package.json`과 lockfile version과 같다.
- `CHANGELOG.md`가 실제 사용자·호환성 변경을 설명한다.
- `cargo deny check licenses sources`가 통과하고 `THIRD_PARTY_LICENSES.html`이 최신이다.
- 네 Python wheel과 네 Node.js addon build job이 모두 성공했다.
- macOS artifact는 deployment target 14.0, Linux Python wheel은 manylinux 2.28이다.
- Browser, musl, Linux arm64 등 지원하지 않는 target을 package metadata가 약속하지 않는다.

## References

- [Python and Node.js SDK Distribution](../adr/0004-python-node-sdk-distribution.md)
- [RSPDL Compiler Architecture](../architecture.md)
