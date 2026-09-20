# 거부되는 데이터 라이프사이클 예시

이 디렉터리의 `.rspdl` 파일은 정상 예제가 아니라 분석기가 의도적으로 거부해야 하는 입력이다. 모두 `compile --json` 실행 시 종료 코드 `1`을 반환한다.

| 예시 | 거부 이유 | 핵심 진단 |
| --- | --- | --- |
| `unproduced-data-usage.rspdl` | 생성·입력되지 않은 주문과 상태를 조회·수정·삭제함 | `RSPDL-DATA-001`, `RSPDL-DATA-002` |
| `unproduced-calculation.rspdl` | 여러 주문 항목의 금액을 합산하지만 그 금액을 입력·생산하는 경로가 없음 | `RSPDL-DATA-001` |
| `conflicting-action-results.rspdl` | 주문 취소라는 같은 행동이 같은 주문을 수정하면서 삭제함 | `RSPDL-DATA-004` |
| `layout-field-outside-screen.rspdl` | 와이어프레임의 폼에 화면이 입력하지 않는 메모 칸이 있음 | `RSPDL-LAYOUT-004` |
| `flow-to-missing-screen.rspdl` | 화면 흐름이 선언되지 않은 예약 확인 화면으로 감 | `RSPDL-KO-REF-001` |

예를 들어 다음 명령으로 각각의 structured diagnostic을 확인할 수 있다.

```console
cargo run -p rspdl-cli -- compile examples/rejected/unproduced-data-usage.rspdl --json
cargo run -p rspdl-cli -- compile examples/rejected/unproduced-calculation.rspdl --json
cargo run -p rspdl-cli -- compile examples/rejected/conflicting-action-results.rspdl --json
cargo run -p rspdl-cli -- compile examples/rejected/layout-field-outside-screen.rspdl --json
cargo run -p rspdl-cli -- compile examples/rejected/flow-to-missing-screen.rspdl --json
```

이 검사는 문장 순서를 추론하지 않는 구조적 검사다. 선언 어디에도 생산자가 없으면 소비를 거부하고, 같은 action ID와 model ID에 서로 다른 mutation 결과가 있으면 충돌로 거부한다.

`unproduced-calculation.rspdl`에서는 주문과 주문 항목 모델 자체는 생성된다. 하지만 계산 재료인 `주문 항목.금액`을 입력하는 다음 문장이 없으므로 금액뿐 아니라 그 값들로 계산할 `주문.결제 예정 금액`에도 도달 가능한 생산자가 없다.

```rspdl
주문 항목 추가 화면(create_item)에서는 주문 항목의 금액을 입력할 수 있다.
```

따라서 분석기는 원본 필드와 계산 결과 필드에 `RSPDL-DATA-001`을 발생시켜 compilation을 거부한다. 별도로 현재 교차 모델 합계에는 집계 대상을 선택할 relation/join이 없으므로 `RSPDL-DATA-W002` 안내도 함께 발생한다.

뒤의 두 예시는 머리말에 적은 구조를 문장이 선언한 의미와 대조해 거부한다. `layout-field-outside-screen.rspdl`의 폼에는 `메모` 칸이 있지만 그 화면이 메모를 입력한다는 문장이 없다. 화면 설계와 명세가 갈라진 자리이며, 진단은 폼 쪽 위치와 함께 대조된 문장의 위치(`declared_at`)를 같이 준다.

`flow-to-missing-screen.rspdl`의 흐름은 `예약 확인 화면`으로 가지만 그런 화면을 선언한 문장이 없다. 머리말의 참조는 문장의 참조와 같은 해석기가 풀므로, 없는 이름은 의미 진단이 아니라 문장과 똑같이 `ko.reference.not_found`로 보고된다.
