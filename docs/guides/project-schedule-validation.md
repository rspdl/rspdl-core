---
id: project-schedule-validation
title: 프로젝트 일정 입력과 저장 검증 예제
type: guide
status: active
version: "1"
summary: Connects one compiled date-order invariant to date input bounds, compiler-backed validation, and guarded local saving in an application example.
topics:
  - date-ordering
  - application-projection
  - input-validation
  - diagnostics
related:
  - unified-statements
  - core-application-boundary
  - frontend-semantic-analysis-contract
problem_refs:
  - data-lifecycle-modeling-gap
  - policy-consistency-blind-spots
  - semantic-source-provenance-loss
last_updated: "2026-10-05"
owners:
  - rspdl-maintainers
---

# 프로젝트 일정 입력과 저장 검증 예제

## 해결하는 문제

[#44](https://github.com/rspdl/rspdl-core/issues/44)의 날짜 순서 제약을 입력 단계와 저장 단계에
각각 작성하면 한쪽만 수정되거나 브라우저 입력 제한을 우회해 잘못된 일정을 저장할 수 있다.
이 예제는 한국어 원문 하나에서 얻은 Canonical IR을 날짜 입력에 투영하고, 같은 원문으로
저장 직전 compiler 검사를 실행한다. 화면과 서버에 별도의 프로젝트 날짜 정책을 작성하지 않는다.

이슈 원문의 같은 날 허용 여부와 날짜 선택기 문구는 미결정이다. 예제의 엄격한 순서는
재현 가능한 fixture의 선택이며 원문 정책의 확정이 아니다. 같은 날 허용 여부는 다음처럼
원문에 명시하고 서버를 다시 시작한다.

```rspdl
프로젝트의 마감일은 시작일보다 커야 한다.
```

```rspdl
프로젝트의 마감일은 시작일보다 크거나 같아야 한다.
```

## 실행

저장소 루트에서 실행한다. Python 표준 라이브러리와 빌드한 CLI를 사용하며 웹 패키지 설치는
필요하지 않다.

```sh
cargo build -p rspdl-cli
python3 examples/project-schedule/server.py
```

브라우저에서 `http://127.0.0.1:8765`를 연다. 입력값을 바꾸면 날짜 선택 범위와 검증 결과가
갱신된다. 저장은 실제 compiler 검사를 통과한 경우에만 이루어진다. 이 예제는 로컬에서
일정 하나를 생성·교체하며 제품의 사용자·권한·다중 프로젝트 저장 API를 제공하지 않는다.
CLI와 원문, 포트, 저장 파일은 `server.py --help`의 옵션으로 지정할 수 있다.

## 원문에서 화면까지의 계약

1. 서버는 시작할 때 원문을 고정해 CLI로 컴파일한다. 화면과 후속 검사는 같은 원문을 사용한다.
2. 화면은 `module.models[].fields`와 `module.constraints`를 읽는다. 한국어를 다시 파싱하지 않는다.
3. 날짜 제약의 좌우 field ID와 operator로 두 입력의 `min`/`max`를 계산한다. 엄격한 비교는
   달력 날짜 하루를 더하거나 빼고, 포함 비교는 같은 날을 경계로 삼는다. 입력값을 자동 보정하지 않는다.
4. 입력 변경과 저장 요청을 CLI `check`로 검사한다. 서버의 저장 판단은 compiler의 오류·위반·정책 결과에
   따른다. 클라이언트가 검증을 생략하거나 제한된 날짜를 직접 전송해도 저장 검사를 거친다.
5. 위반의 `constraint_id`를 IR 제약에 연결해 관련 필드와 실제 좌우 값을 표시한다. 원문은
   UTF-8 byte span으로 찾는다. 필수값과 잘못된 날짜는 compiler runtime diagnostic으로 전달한다.

GET `/api/schema`는 `{source, compilation}`을, POST `/api/check`와 `/api/save`는
`{record: {...}}`에 대해 `{accepted, saved, report}`를 반환한다. `compilation`과 `report`는
단일 파일 CLI의 JSON 결과다. compiler 프로세스 실패·시간 초과는 성공한 검사 결과로 바꾸지 않는다.

날짜 선택 제한은 application projection이고 유효성 판정은 compiler 책임이다. UI의 범위 계산은
서버 저장 검사를 대체하지 않는다. core에 브라우저 component나 view model API를 추가하지 않는다.

## 검증해야 할 사례

| 사례 | 기대 결과 |
| --- | --- |
| 마감일이 시작일 다음 날 | 검사와 저장 성공 |
| 마감일이 시작일 전날 | 양쪽 필드와 원문 근거 표시, 저장 거부 |
| 같은 날 | 엄격한 비교에서 거부, 포함 비교에서 허용 |
| 저장 가능한 입력 뒤 시작일을 마감일 이후로 변경 | 범위와 오류 갱신, 이전 검사 성공으로 저장하지 않음 |
| 필수 날짜 누락·존재하지 않는 날짜 | runtime diagnostic, 저장 거부 |
| 윤년·월말·연말 | 시간대와 무관한 달력 날짜 경계 |
| 입력 비우기 | 이전 상대 필드에서 계산한 제한과 검사 결과를 유지하지 않음 |
| 클라이언트 제한 우회 | 서버 재검사로 저장 거부, 이전 저장 파일 유지 |
| compiler 오류·시간 초과·지원 밖 결과 | 유효함으로 표시하거나 저장하지 않음 |

SDK 회귀 검사는 `crates/rspdl-sdk/tests/project_schedule.rs`에, application 예제 검사는
`examples/project-schedule/`에 있다. `./scripts/check-project-schedule.sh`는 실제 CLI를 빌드한 뒤
서버 통합 검사와 Node의 날짜 범위·진단 projection 검사를 실행한다.

2026-10-05 검증 결과:

- `./scripts/check.sh`: Rust 393개, Python 14개 테스트와 formatting·strict Clippy 통과.
  이 합계에 새 SDK 경계 검사 5개가 포함된다.
- `./scripts/check-project-schedule.sh`: 실제 CLI를 사용하는 서버 검사 6개와 Node 검사 14개 통과.
  Node 검사는 실제 compile/check JSON의 날짜 표현도 검증하며 CLI가 없으면 생략하지 않는다.
- Chromium에서 실제 입력·저장, 엄격한 같은 날 거부, 포함 비교의 같은 날 저장, 역전 위반의
  날짜·필드·원문 표시, 입력 비우기, 오래된 응답 무시와 모바일 폭을 확인했다. 잘못된 원문과
  예제 지원 밖 원문은 화면을 시작하지 않고 직접 저장 요청도 거부했다.

독립 검토에서 발견한 화면·서버 지원 범위 불일치와 실제 브라우저에서 발견한 날짜의 내부 JSON
노출을 수정하고 회귀 검사를 추가했다. 검증 로그의 성공은 아래 예제 범위에 한정한다.

## 범위

이 전달물은 두 날짜 필드와 지원하는 field 간 순서 제약을 가진 로컬 application 예제다.
실제 제품의 날짜 선택기 연결 및 배포 완료를 의미하지 않는다. 원문의 나머지 프로젝트 정보,
원문 정책 미결정, 사용자 인증, 다중 사용자 동시 갱신과 새 statement 실행은 후속 범위다.
삭제·관계·집계나 시간 트리거를 추가하지 않는다. 새 데이터의 검증과 기존 일정의 교체를 같은
규칙으로 처리하며, 거부된 변경이 기존 저장 데이터에 영향을 주지 않는 것을 확인한다.
