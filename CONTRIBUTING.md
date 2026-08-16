# RSPDL에 기여하기

RSPDL에 관심을 갖고 기여해 주셔서 감사합니다. 이 문서는 저장소를 처음 접한 기여자가 제품의 원인과 의도를 공유하고, 개발 환경을 준비하고, 변경에 맞는 테스트를 추가하고, 검토 가능한 Pull Request를 만드는 데 필요한 기준을 설명합니다.

RSPDL의 출발점은 기능 목록이 아니라 [Product Vision](docs/product/vision.md)과 [Problem Topic](docs/guides/problem-driven-development.md)입니다. 제품 또는 언어 동작을 바꾸기 전에 해결하려는 반복 원인을 먼저 연결해 주세요.

## 시작하기

다음 도구가 필요합니다.

- Git
- `rustup`
- C/C++ 빌드 도구와 CMake (`rspdl-solver-z3` 빌드에 필요)

저장소를 clone하면 `rust-toolchain.toml`에 지정된 Rust와 `rustfmt`, Clippy가 자동으로 선택됩니다.

```console
cargo build --workspace
cargo test --workspace
```

첫 빌드에서는 vendored Z3를 컴파일하므로 시간이 더 걸릴 수 있습니다.

## 저장소 구조

- `crates/rspdl-domain`: Locale에 독립적인 타입, 논리 모델과 의미 규칙
- `crates/rspdl-grammar-compiler`: 실행 가능한 제한 EBNF compiler와 generated parser runtime
- `crates/rspdl-ko`: 한국어 scanner, parser, AST, lowering과 formatter
- `crates/rspdl-compiler`: frontend와 backend를 연결하는 공개 compiler facade
- `crates/rspdl-solver-z3`: Z3 기반 constraint solver
- `crates/rspdl-sdk`: Python과 Node.js binding이 공유하는 versioned JSON facade
- `crates/rspdl-cli`: 파일 입출력, 출력 형식과 exit code
- `bindings/python`: PyO3 stable-ABI Python package
- `bindings/node`: napi-rs Node.js package와 TypeScript declaration
- `examples`: 사람이 읽고 직접 실행할 수 있는 RSPDL 예제
- `docs/product`: 제품의 북극성과 사용자 약속
- `docs/problems`: 반복되는 인과 원인을 기록한 Problem Topic
- `docs/guides`: 기여와 검증 워크플로우
- `conformance`: 구현 독립적인 공개 언어 계약 fixture가 추가될 위치
- `docs/adr`: 이미 결정된 중요한 기술 선택
- `docs/rfcs`: 문법, 의미와 호환성 변경 제안
- `.agents/skills`: 사람과 AI 에이전트가 공유하는 context discovery 및 문제 기반 개발 절차
- `scripts/check.sh`: knowledge, format, lint와 test를 묶은 제출 전 harness

의존성 방향과 각 crate의 책임은 `docs/architecture.md`를 참고해 주세요.

## 변경 전 확인하기

오타 수정, 문서 개선과 범위가 작은 버그 수정은 바로 Pull Request로 제안할 수 있습니다.

다음 변경은 구현 전에 Issue 또는 RFC에서 사용 사례와 호환성 영향을 먼저 논의해 주세요.

- 문법 또는 formatter 출력 변경
- Canonical IR, Semantic Graph 또는 Diagnostic schema 변경
- 공개 API의 호환되지 않는 변경
- 새로운 Locale 또는 solver backend 추가
- 여러 crate의 책임이나 의존성 방향 변경

하나의 Pull Request에는 가능한 한 하나의 목적만 담아 주세요. 리팩터링과 동작 변경은 검토할 수 있도록 분리하는 것을 권장합니다.

## Problem Topic에서 시작하기

제품 기능, 문법, Canonical IR, 의미 규칙 또는 아키텍처를 바꿀 때는 다음 순서를 따릅니다.

1. metadata에서 관련 원인을 검색합니다.

   ```console
   python3 .agents/skills/discover-rspdl-knowledge/scripts/knowledge_index.py query "<원인과 영역>"
   ```

2. 일치하는 `problem` 문서의 graph와 본문을 읽습니다.
3. 기존 원인에 포함되면 새 토픽을 만들지 않고 해당 stable ID를 사용합니다.
4. 기존 원인으로 설명할 수 없으면 Problem Topic Issue로 원인을 논의하고 `docs/problems/`에 하나의 인과 메커니즘을 기록합니다.
5. PRD, RFC, ADR, architecture 또는 spec의 `problem_refs`에 원인 ID를 연결합니다.
6. 구현 전에 정상, 실패, 경계, 오탐 방지 사례와 기대 diagnostic evidence를 정의합니다.

데이터 변경은 create, read, update, delete, derive와 참조 영향을 확인합니다. 정책 변경은 conflict, gap, overlap, unreachable, totality, default와 override를 확인합니다.

전체 흐름과 완료 조건은 [Problem-driven Development](docs/guides/problem-driven-development.md)를 따릅니다. AI 에이전트는 `.agents/skills/develop-from-product-problem/SKILL.md`에서 같은 절차를 읽습니다.

## 테스트 위치

변경을 소유하는 가장 가까운 계층에 테스트를 추가합니다.

- private 함수와 parser production: 해당 `src` module의 unit test
- crate 공개 API: 해당 crate의 `tests/` integration test
- CLI 입력, 출력과 exit code: `crates/rspdl-cli/tests/`
- 공개 언어 의미: repository root의 `conformance/`

Conformance fixture는 구현 세부사항인 token, CST 또는 AST를 계약으로 고정하지 않습니다. Canonical IR, 의미 분석 결과, 구조화된 진단과 결정론처럼 다른 구현도 동일하게 제공해야 하는 결과만 비교합니다.

공개 문법이나 의미 규칙을 추가할 때는 가능하면 다음 사례를 함께 제공해 주세요.

- 정상 사례
- 실패 사례
- 경계 사례
- 오류와 비슷하지만 허용되어야 하는 사례

Golden fixture를 갱신할 때는 변경된 파일뿐 아니라 언어 의미가 달라지는 이유를 Pull Request에 설명해 주세요.

## 제출 전 검사

다음 harness를 실행해 knowledge index, metadata 추적성, Rust format, Clippy와 전체 test를 함께 검사해 주세요.

```console
./scripts/check.sh
```

knowledge 문서를 추가하거나 front matter를 변경했다면 먼저 index를 재생성합니다.

```console
python3 .agents/skills/discover-rspdl-knowledge/scripts/knowledge_index.py build
```

문서 또는 예제를 변경했다면 관련 명령을 직접 실행하고, 사용한 명령과 결과를 Pull Request에 적어 주세요.

SDK package를 바꿨다면 Rust harness와 함께 해당 package smoke test도 실행합니다.

```console
cargo test -p rspdl-sdk

cd bindings/node
npm ci
npm run build:debug
npm test
npm run typecheck
```

Python wheel build와 registry release 절차는 [Package Release Guide](docs/guides/releasing-packages.md)를 따릅니다. Package version은 직접 따로 올리지 않고 Release Please가 Cargo workspace, npm metadata와 changelog를 한 Release PR에서 함께 갱신하게 합니다.

## Pull Request 작성

Pull Request 설명에는 다음 내용을 포함해 주세요.

- 변경이 필요한 이유와 해결하려는 사용 사례
- 연결한 Problem Topic ID와 원인의 인과 메커니즘
- 선택한 접근 방식과 중요한 대안
- 정상, 실패, 경계와 오탐 방지 사례
- 구조화된 진단의 Rule ID, source span과 evidence 영향
- 추가하거나 실행한 테스트와 harness 결과
- 문법, Canonical IR, 진단 또는 호환성에 미치는 영향
- 명시적으로 구현하지 않은 범위 또는 `unknown` 동작

검토 의견을 반영할 때 이해하지 못한 부분이 있다면 추측하기보다 질문해 주세요. 제출한 변경의 동작과 설계 결정을 설명할 수 있어야 합니다.
