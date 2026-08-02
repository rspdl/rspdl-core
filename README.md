# RSPDL

RSPDL은 제품 기획의 데이터와 정책을 사람이 읽고 기계가 검증할 수 있게 만드는 한국어 우선 선언형 언어입니다.

정책 검토는 제품 개발의 설거지와 비슷합니다. 반드시 해야 하지만 잘 끝냈다고 눈에 띄는 사용자 가치가 생기지는 않고, 빠뜨리면 구현 중단과 재작업이 생깁니다. RSPDL은 이 부담을 기획자 개인이나 기획까지 맡은 개발자의 꼼꼼함에 남겨두지 않고, 구현 전에 반복 가능한 검증으로 옮기려 합니다.

## 우리가 해결하려는 일

- 여러 역할을 동시에 수행하는 기획자가 모든 데이터 상태와 정책 조합을 미리 검토하기 어렵습니다.
- 개발자는 모호한 부분을 알아서 결정하면 의도와 어긋나 재작업하고, 결정을 기다리면 구현이 멈춥니다.
- 자연어 기획은 데이터가 언제 생기고 사라지는지, 어떤 조건이 충돌하거나 비어 있는지 드러내기 어렵습니다.
- AI 에이전트도 같은 기획을 매번 다시 해석하면 토큰을 쓰고 서로 다른 가정을 만들 수 있습니다.

RSPDL의 약속은 단순합니다.

> 한 번 명시하고, 일찍 검증하고, 영향과 근거를 함께 전달한다.

RSPDL은 명시된 의도를 Canonical Semantic IR로 손실 없이 전달하는 기반을 지향합니다. 작성자가 말하지 않은 의도까지 추측하거나 현실의 요구사항과 100% 일치한다고 주장하지는 않습니다.

## 지향하는 흐름

```text
기획 작성 → Canonical IR → lifecycle·정책 검증 → 결정 보완
         → 구현 가능한 context 전달 → 제품 완성 → 사용자 피드백
```

- 기획자는 개발 단계에서 생길 질문과 반례를 작성 중에 받습니다.
- 개발자는 추측 대신 검증된 결정과 명시적인 미결정 목록을 받습니다.
- 팀은 첫 전달 뒤 동작하는 결과를 만들고 실제 피드백으로 다음 반복을 시작합니다.
- AI와 code generator는 같은 stable ID와 semantic graph에서 필요한 context만 읽을 수 있습니다.
- 한 번의 의미 수정이 영향을 주는 정책, 데이터, 플로우와 downstream artifact를 추적할 수 있습니다.

## 현재 구현된 범위

현재 `0.1` vertical slice는 다음을 지원합니다.

- 한국어 module, enum, record field와 field constraint
- role, action과 조건 없는 allow 또는 deny policy
- deterministic parser, formatter와 Canonical domain model
- Z3 기반 record constraint 검사
- Datalog 기반 runtime policy match와 `allowed`, `denied`, `conflict`, `unmatched` 분류
- JSON compilation 및 diagnostic 출력

일반 데이터 lifecycle과 상태 전이, 조건부 정책의 전체 조건 공간 분석, 영향 분석과 code generation은 목표 범위이지만 아직 구현되지 않았습니다. 현재와 목표를 구분한 상세 요구사항은 [PRD](docs/prd.md)를 참고해 주세요.

## 짧은 예시

```rspdl
@모듈 재고(inventory)

재고 항목(item)은 다음 필드들로 구성되어 있다.
    이름(name): 필수 문자열
    수량(quantity): 필수 정수

재고 항목의 수량은 0 이상이어야 한다.
```

```console
cargo run -p rspdl-cli -- compile examples/inventory.rspdl --json
cargo run -p rspdl-cli -- check examples/expense-approval.rspdl \
  --data crates/rspdl-cli/tests/fixtures/expense-approval-data.json --json
```

`check`는 compile 또는 input 오류에 exit code `1`, 정책 충돌·미일치나 제약 위반 발견에 `2`, 발견 사항이 없으면 `0`을 반환합니다.

## 개발 시작하기

Rust toolchain, C/C++ build tools와 CMake가 필요합니다. 첫 빌드는 vendored Z3 컴파일로 시간이 걸릴 수 있습니다.

```console
cargo build --workspace
./scripts/check.sh
```

## 문서 지도

- [Product Vision](docs/product/vision.md): 누구의 어떤 고통을 왜 해결하는가
- [PRD](docs/prd.md): 제품·언어 요구사항과 현재 구현 경계
- [Data Lifecycle Modeling Gap](docs/problems/0001-data-lifecycle-modeling-gap.md): 데이터 존재 시점과 연산 공백
- [Policy Consistency Blind Spots](docs/problems/0002-policy-consistency-blind-spots.md): 충돌·누락·중첩·도달 불가
- [Problem-driven Development](docs/guides/problem-driven-development.md): 원인에서 코드와 증명까지 연결하는 기여 흐름
- [Knowledge Index](docs/index.md): RFC, ADR, architecture를 포함한 전체 문서 인덱스
- [Contributing](CONTRIBUTING.md): 환경 설정, 테스트와 PR 기준

기능을 제안하거나 구현할 때는 먼저 기존 Problem Topic을 연결하세요. 기존 원인으로 설명할 수 없을 때만 새 Problem Topic을 만듭니다. 자세한 절차는 [CONTRIBUTING.md](CONTRIBUTING.md)에 있습니다.

## 프로젝트 상태

RSPDL은 초기 단계의 실험적 오픈소스 프로젝트입니다. 문법과 public API는 `0.x` 동안 변경될 수 있습니다.
