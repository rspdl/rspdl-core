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

현재 구현은 다음을 지원합니다.

- 한국어 module, enum, record field와 field constraint
- 문장형 화면 생성·입력·조회·수정·삭제 선언과 field provenance 검증
- 문장형 action 생성·수정·삭제 결과와 동일 action·model mutation 충돌 검증
- stable ID와 typed existing-model/enum·scalar input을 가진 문장형 action input 선언
- direct enum action input의 문장형 conditional ExactlyOne Create/Skip과 enum coverage·same-variant conflict 검증
- conditional production의 required output field에 direct action input, ExistingModel input field 또는 typed constant를 `PreMutation` producer로 기록하며, 무조건 또는 같은 decision enum variant 조건별 Create path gap/conflict를 진단
- `필드의 합계` 계산 dependency와 원본 변경 시 재계산 선언
- 생산자 없는 필드 사용 오류와 미조회 입력 필드 안내
- 화면 또는 action 생성 경로가 없는 모델의 조회·수정·삭제·계산 사용 오류
- role, action과 조건 없는 allow 또는 deny policy
- deterministic parser, formatter와 Canonical domain model
- 공통 `Frontend` trait과 stable-ID Unlinked IR을 통한 교체 가능한 Locale frontend
- Locale과 분리된 stable ID linking, type checking 및 data usage analyzer
- message key와 정렬된 argument를 사용하는 Locale 중립 structured diagnostic
- Z3 기반 record constraint 검사
- field를 가진 record model, 문장형 unary/binary relation과 `nonempty`, `required`, `unique`, `exclusive`, `exhaustive`, compatible `coexistent` 선언
- 실제 record 없이 finite scope의 typed field constraint와 relation rule을 검사하고 가상 entity/field/relation witness를 찾는 bounded model finder
- 단일 닫힌 enum decision point의 정적 gap, compatible overlap 및 allow/deny conflict 분석 API
- 결정적 직접 runtime policy match와 `allowed`, `denied`, `conflict`, `unmatched` 분류
- JSON compilation 및 diagnostic 출력
- 같은 versioned JSON contract를 사용하는 Python 3.11+와 Node.js 22/24 native SDK

화면 간 순서와 분기, 삭제 이후 접근, 실제 relation data binding과 join 실행, 3항 이상 관계·임의 양화식, 일반 계산식, 조건부 정책의 한국어 문법·compiler 진단 연결, default·override와 unreachable 분석은 목표 범위이지만 아직 구현되지 않았습니다. 특히 가격 산술·통화·환율·반올림은 아직 지원하지 않습니다. 현재와 목표를 구분한 상세 요구사항은 [PRD](docs/prd.md)를 참고해 주세요.

## 짧은 예시

```rspdl
@모듈 재고(inventory)

재고 항목(item)은 다음 필드들로 구성되어 있다.
    이름(name): 필수 문자열
    수량(quantity): 필수 정수

재고 항목의 수량은 0 이상이어야 한다.
```

관계와 cardinality도 annotation이 아니라 방향이 드러나는 문장으로 쓴다.

```rspdl
프로젝트는 사용자를 소유자(owner)로 가질 수 있다.
모든 프로젝트는 소유자를 하나 이상 가져야 한다.
각 프로젝트는 소유자를 최대 하나만 가질 수 있다.
```

`@`는 현재 문서 metadata인 `@모듈`에만 허용된다. 데이터 모델은 하나 이상의 field를 가져야 하며 빈 모델을 추상 개체처럼 사용할 수 없다.

```console
cargo run -p rspdl-cli -- compile examples/inventory.rspdl --json
cargo run -p rspdl-cli -- model examples/project-ownership.rspdl --scope 3 --json
cargo run -p rspdl-cli -- compile examples/field-provenance.rspdl --json
cargo run -p rspdl-cli -- check examples/expense-approval.rspdl \
  --data crates/rspdl-cli/tests/fixtures/expense-approval-data.json --json
```

### 거부 예시

생성·입력되지 않은 데이터를 조회·계산·수정·삭제하거나, 하나의 행동이 같은 데이터에 `수정`과 `삭제`를 동시에 결과로 내면 compilation error로 거부한다. 실행 가능한 전체 입력과 기대 진단은 [`examples/rejected`](examples/rejected/README.md)에 있다.

```console
cargo run -p rspdl-cli -- compile examples/rejected/unproduced-data-usage.rspdl --json
cargo run -p rspdl-cli -- compile examples/rejected/unproduced-calculation.rspdl --json
cargo run -p rspdl-cli -- compile examples/rejected/conflicting-action-results.rspdl --json
```

`check`는 compile 또는 input 오류에 exit code `1`, 정책 충돌·미일치나 제약 위반 발견에 `2`, 발견 사항이 없으면 `0`을 반환합니다.

`model`은 실제 data file을 받지 않습니다. 선언을 만족하는 가상 세계가 scope 안에 있으면 `SAT`과 witness를 반환하고, 없으면 전역 모순이 아닌 `UNSAT_WITHIN_BOUND`와 관련 Rule ID를 반환합니다.

### 분석 결과를 직접 확인하는 예시

정책 조건 공간 분석은 아직 한국어 문법이나 CLI에 연결되지 않은 Rust API입니다. 실행 가능한 Z3 예시는 하나의 `active` 정책만 있을 때 누락된 상태, 같은 상태의 allow/deny 충돌, 같은 effect의 compatible overlap을 각각 보여줍니다.

```console
$ cargo run -p rspdl-solver-z3 --example total_policy_analysis
GAP ended: status=ended
GAP paused: status=paused
GAP scheduled: status=scheduled
CONFLICT active_allow + active_deny: status=active
COMPATIBLE_OVERLAP active_first + active_second: status=active
```

Bounded relational model은 같은 선언도 scope에 따라 결과가 달라질 수 있습니다. [프로젝트 배정 예시](examples/project-assignment-bound.rspdl)는 주 담당자와 보조 담당자가 모두 필요하지만 같은 `(프로젝트, 사용자)` tuple에서는 둘이 겹칠 수 없다고 선언합니다.

```console
$ cargo run -p rspdl-cli -- model examples/project-assignment-bound.rspdl --scope 1
UNSAT_WITHIN_BOUND (모델별 scope: 1, 규칙: ...)

$ cargo run -p rspdl-cli -- model examples/project-assignment-bound.rspdl --scope 2
SAT (모델별 scope: 2)
...
```

Scope 1에서는 사용자 slot이 하나뿐이라 두 관계를 분리할 수 없습니다. Scope 2에서는 서로 다른 가상 사용자를 선택하는 witness가 존재합니다. 작은 scope의 `UNSAT_WITHIN_BOUND`는 무한한 모든 세계에서의 모순을 의미하지 않습니다.

## 개발 시작하기

Rust toolchain, C/C++ build tools와 CMake가 필요합니다. 첫 빌드는 vendored Z3 컴파일로 시간이 걸릴 수 있습니다.

```console
cargo build --workspace
./scripts/check.sh
```

## Python과 TypeScript에서 사용하기

RSPDL 분석 core는 Python과 Node.js에 같은 versioned JSON 결과를 제공합니다. 지원 플랫폼에서는 Rust, CMake 또는 Z3를 설치하지 않고 package manager만으로 미리 빌드된 native artifact를 설치합니다.

Python 3.11 이상:

```console
pip install rspdl
```

```python
import rspdl

source = {
    "path": "inventory.rspdl",
    "text": """@모듈 재고(inventory)

재고 항목(item)은 다음 필드들로 구성되어 있다.
    이름(name): 필수 문자열
""",
}

response = rspdl.compile([source])
print(response["result"]["files"][0]["diagnostics"])
```

Node.js 22 또는 24와 TypeScript:

```console
npm install rspdl-core
```

```typescript
import { compile, type Source } from 'rspdl-core'

const source: Source = {
  path: 'inventory.rspdl',
  text: `@모듈 재고(inventory)

재고 항목(item)은 다음 필드들로 구성되어 있다.
    이름(name): 필수 문자열`,
}

const response = await compile([source])
console.log(response.result.files[0].diagnostics)
```

두 package는 `compile`, runtime data를 검사하는 `check`, bounded virtual model을 찾는 `find_model`/`findModel`을 제공합니다. 문법·의미·runtime 진단과 model finding은 exception이 아니라 `result`에 보존됩니다. Native wire contract의 malformed JSON, 지원하지 않는 schema/Locale와 잘못된 SDK option은 stable `RSPDL-SDK-*` 오류가 됩니다.

첫 배포 범위는 Linux x86_64 glibc, macOS 14 이상 x86_64·arm64, Windows x86_64입니다. Browser, Alpine/musl, Linux arm64, Bun, Deno와 PyPy는 아직 지원하지 않습니다. Python wheel은 CPython stable ABI로 3.11 이상을 지원합니다.

## 문서 지도

- [Product Vision](docs/product/vision.md): 누구의 어떤 고통을 왜 해결하는가
- [PRD](docs/prd.md): 제품·언어 요구사항과 현재 구현 경계
- [Data Lifecycle Modeling Gap](docs/problems/0001-data-lifecycle-modeling-gap.md): 데이터 존재 시점과 연산 공백
- [Field Provenance, Screen Usage, Action Data Mutations, and Sum Derivation Grammar](docs/rfcs/0005-field-provenance-and-sum-derivation.md): 화면·행동의 생산·소비와 합계 계산 문법
- [Policy Consistency Blind Spots](docs/problems/0002-policy-consistency-blind-spots.md): 충돌·누락·중첩·도달 불가
- [Total Policy Condition Spaces and SMT-First Consistency Analysis](docs/rfcs/0006-total-policy-condition-space-analysis.md): 닫힌 vocabulary, 전체 조건 공간과 명시적 override의 SMT 분석 계약
- [Conditional Data Production for Notifications and Prices](docs/rfcs/0008-conditional-data-production.md): 입력 provenance와 lifecycle을 검증하는 알림·가격 output 생산의 제안 의미 계약
- [Finite Relational Rules and Bounded Model Finding](docs/rfcs/0007-finite-relational-model-finding.md): typed relation, 명시적 cardinality/compatibility와 가상 데이터 모델 탐색
- [Problem-driven Development](docs/guides/problem-driven-development.md): 원인에서 코드와 증명까지 연결하는 기여 흐름
- [Frontend and Semantic Analysis Contract](docs/specs/frontend-semantic-analysis-contract.md): 다른 표현 언어가 구현할 stable-ID IR과 진단 계약
- [Knowledge Index](docs/index.md): RFC, ADR, architecture를 포함한 전체 문서 인덱스
- [Python and Node.js SDK Distribution](docs/adr/0004-python-node-sdk-distribution.md): 공통 wire contract, native package와 version 결정
- [Package Release Guide](docs/guides/releasing-packages.md): Release PR, registry Trusted Publishing과 첫 npm bootstrap 절차
- [Contributing](CONTRIBUTING.md): 환경 설정, 테스트와 PR 기준

기능을 제안하거나 구현할 때는 먼저 기존 Problem Topic을 연결하세요. 기존 원인으로 설명할 수 없을 때만 새 Problem Topic을 만듭니다. 자세한 절차는 [CONTRIBUTING.md](CONTRIBUTING.md)에 있습니다.

## 프로젝트 상태

RSPDL은 초기 단계의 실험적 오픈소스 프로젝트입니다. 문법과 public API는 `0.x` 동안 변경될 수 있습니다.

Source와 공식 package는 [Apache License 2.0](LICENSE)으로 배포되며, binary package에는 [제3자 의존성 라이선스](THIRD_PARTY_LICENSES.html)가 함께 포함됩니다.
