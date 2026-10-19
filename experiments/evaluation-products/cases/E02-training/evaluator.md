# E02 비공개 평가 기준 — 사내 교육 수료 문서 발급·정정 데스크

이 파일은 평가자 전용이다. 생성기에 공개하는 제품 입력은 `input.md` 하나다. E02 전체가 평가 단위이며 아래 세 여정은 독립 프롬프트가 아니다. 실행 코드, 정답 RSPDL, reference fixture는 제공하지 않는다. 평가 대상은 생성된 **문서가 보존한 제품 의미와 증거**다.

지원 기준 HEAD: `837290a83911e160f27835f246a5bd7df21107b1`.

## 판정 방법과 중요도

공통 `public-contract.md`와 `judge-instructions.md`를 함께 적용한다. 각 행을 `충족 / 위반 / 판정 보류`로 기록하고 산출물의 절·문장·stable ID 등 실제 근거 위치를 남긴다. 일부 조건·효과·출처가 누락되면 위반이며, 실제 해석 쟁점만 보류한다. 중요도 `핵심`은 업무 결과·출처·권한·완료를 바꾸는 누락, `주요`는 구조·복구·표시 계약의 누락이다. 중요도는 영향 설명용이며 가중 점수를 주지 않는다. 고정 실패율이나 특정 작성 스타일을 요구하지 않는다. 한 근거가 여러 요구사항을 충족해도 된다. 이름이 달라도 의미와 참조가 일관되면 인정한다. 전체 제품 흐름을 빠뜨린 조각 모음은 개별 구문의 정확성으로 보상하지 않는다.

컴파일 성공은 문법·일부 정적 의미의 증거다. 그 자체로 42개 요구사항의 충족, 외부 결정의 신뢰성, 실제 실행·조회·전달 성공을 증명하지 않는다. 아래 기대값은 합성 입력을 문서에 대입해 **문서가 명시한 결과**를 검토하는 인간 평가용이다.

양쪽 후보 모두 최종 한국어 제품 명세만 평가한다. 아래 `enum`, `ExistingModel`, `ExactlyOne Create/Skip`, `Required/Unique`, `producer`, `template`는 닫힌 선택값, 전달된 기존 기록, 하나 생성/생성하지 않음, 필수 대상 하나, 값의 출처, 문구 조합이라는 **업무 의미를 가리키는 약칭**이다. 같은 의미의 자연어·표를 인정하며 RSPDL 토큰이나 선언 형식을 요구하지 않는다. 중간 RSPDL·compiler 로그는 평가자에게 제공하지 않고 중간본으로 최종 명세의 누락을 구제하지 않는다. 소스 지도와 문법 설명은 사례의 표현 범위를 확인하는 자료다.

## 요구사항별 증거표

| ID | 중요도 | 충족에 필요한 문서 증거 |
|---|---|---|
| E02-R01 | 주요 | 최초 발급·정정·알림·전달 조회가 한 제품으로 연결되고 제외 범위가 보존됨. |
| E02-R02 | 핵심 | 여섯 별도 enum과 각각의 정확한 값 집합. 자유 문자열·boolean·한 enum 합치기로 결정 공간을 잃지 않음. |
| E02-R03 | 핵심 | 세 초기 모델의 외부 공급, 직접 typed 결정 입력, 입력 묶음 일치의 외부 전제. 출석 계산이나 내부 join 검증으로 대체하지 않음. |
| E02-R04 | 주요 | 교육회차 네 필드와 문자열/날짜 타입·필수성. |
| E02-R05 | 주요 | 수강자 세 필드의 문자열 타입·필수성. |
| E02-R06 | 주요 | 판정번호·enum 결과·날짜 판정일과 필수성. |
| E02-R07 | 핵심 | 발급문서 일곱 필드, 발급일 날짜·종류 enum·나머지 문자열, 모두 필수. |
| E02-R08 | 주요 | 요청 여섯 필드와 정확한 타입·필수성. |
| E02-R09 | 핵심 | 알림 다섯 문자열 필드와 필수성. |
| E02-R10 | 주요 | 전달접수 다섯 필드와 정확한 타입·필수성. |
| E02-R11 | 핵심 | 같은 모델의 세 inclusive 날짜 순서 제약. 동일 날짜 허용; 날짜 자동값이나 레코드 간 계산이 없음. |
| E02-R12 | 핵심 | output-first 여섯 binary 관계: 문서→판정·수강자, 요청→문서, 알림→문서·수강자, 접수→알림. 각각 Required와 per-output Unique 의미. 대상 레코드의 전역 uniqueness를 요구하지 않음. |
| E02-R13 | 핵심 | 두 역할, 교육담당 열 행동 허용, 감사열람자 여섯 조회 허용·네 생성 금지. 동일 action/resource 기준 정책 증거. |
| E02-R14 | 주요 | 여덟 화면, 역할 연결, 필수 조회·입력·버튼, 구별되는 화면/요소 ID와 action binding. 단순 환경 이름을 역할 연결로 대신하지 않음. |
| E02-R15 | 핵심 | 열 행동의 typed ExistingModel/enum/scalar 입력과 조회 필드. 호출에 없는 레코드를 관계 추적으로 확보하지 않음. |
| E02-R16 | 핵심 | 정정·알림 입력 묶음의 외부 일치 전제와 direct input 사용. 원문서번호 문자열 또는 관계만으로 입력을 조달하지 않음. |
| E02-R17 | 핵심 | 여섯 조회의 found/없음/timeout과 source-backed 조회 제공 필드. 없음·timeout·화면 READ 선언이 acquisition을 만들지 않음. |
| E02-R18 | 주요 | 각 originating lookup button의 세 결과별 도착/동일 화면 처리; 정확한 같은 action 재시도 버튼으로 복구 연결. |
| E02-R19 | 핵심 | 수료→ExactlyOne Create, 미수료→Skip, 검토중→Skip 전부 명시. 미수료·검토중 설명 구별. |
| E02-R20 | 핵심 | 최초 문서 다섯 non-template 필드의 정확한 producer: scalar 번호·날짜, 수강자 이름, 교육회차 교육명, 최초 enum constant. |
| E02-R21 | 핵심 | 두 발급 경로에서 같은 output String 필드만 참조하는 정확한 제목/본문 template. 필드 source order를 값 가용성으로 오인하지 않음. |
| E02-R22 | 핵심 | 최초 문서 관계 두 개에 해당 행동의 기존 판정·수강자 direct binding, 발급내역과 재조회 흐름. |
| E02-R23 | 핵심 | 접수선택의 접수→요청 한 개, 취소→미생성; 생성·취소의 서로 다른 화면 결과. |
| E02-R24 | 핵심 | 요청의 다섯 직접 값, 원문서번호의 existing input field, 대상 문서 direct relation producer. |
| E02-R25 | 핵심 | 요청 조회와 원문서 확인; 정정결정 승인→문서 한 개, 반려/보류→명시 Skip 및 별도 안내. |
| E02-R26 | 핵심 | 정정 출력 번호·날짜 직접 값, 이름은 요청 수정이름, 교육명은 원문서 필드, 정정 constant, 출력 template. 기존 수강자 이름으로 잘못 덮지 않음. |
| E02-R27 | 핵심 | 정정 문서 관계 두 개의 직접 입력 연결, 새 문서 재조회. 원문서·요청 수정/삭제/자동폐기 없음. |
| E02-R28 | 핵심 | typed immutable Event: 알림선택·문자열 제목·existing 문서·existing 수강자. 발급 action과 별도 외부 계기, 최초/정정 공통 계약. |
| E02-R29 | 핵심 | 사건 direct enum 통지→알림 한 개, 생략→명시 Skip. action 조건과 event source를 혼동하지 않음. |
| E02-R30 | 핵심 | event 문서 필드 두 개·수강자 연락처·직접 제목에서 무조건 field producers, 정확한 same-output String 내용 template. |
| E02-R31 | 핵심 | event existing 문서·수강자에서 required relation 직접 연결, payload 불변, 알림 생성과 전송 성공 구분. |
| E02-R32 | 핵심 | 전달선택 요청→접수 한 개, 보류→Skip; 접수됨/보류 메시지와 후속 조회. |
| E02-R33 | 핵심 | 접수번호·날짜 둘은 action value, 문서번호·연락처는 existing 알림 field, required relation은 direct 알림. |
| E02-R34 | 주요 | 조회 성공에서 접수 데이터 확보, 내부 접수 의미와 실제 전달/예약/중복제거의 범위 구분. |
| E02-R35 | 핵심 | 최초 발급 start와 전체 경로, 초기 learner 세 필드만 availability로 선언, 교육/판정 source-backed 조회, 완료 문서 네 필드. |
| E02-R36 | 핵심 | 정정 start의 원문서 모든 필드·learner 세 필드 외부 초기 전제, 요청 조회·정정·새 문서 조회 경로, 새 문서 네 필드 완료 계약. |
| E02-R37 | 핵심 | 알림함 start, 알림 필드의 조회 획득, 접수·접수조회 경로, 접수 세 필드 완료 계약. |
| E02-R38 | 주요 | 감사 열람 start와 source-backed 문서 조회·완료; write action 없이 완료 가능. |
| E02-R39 | 주요 | 네 생성 action의 네 결과가 각 originating button에 연결됨. 실패/timeout은 원래 action retry, skip는 variant별 안내, 실패만으로 데이터 획득 없음. |
| E02-R40 | 핵심 | 제출에 연결된 입력 획득 및 재조회 성공을 통한 완료 데이터 확보. 화면 방문/임의 완료 edge가 producer를 대신하지 않음. |
| E02-R41 | 주요 | 시작·완료·결과 handler·recovery를 함께 명시. 화면 경로의 조건 설명과 enum production 정책이 독립적으로 존재함. |
| E02-R42 | 핵심 | 네 action+한 event의 모든 output production에서 enum coverage, 모든 필수 field/relation 출처. 숨은 생성·전송·검증 요구 없음. |

## 사람 평가 여정 1 — 수료 확인부터 최초 문서와 전달 접수까지

**합성 입력.** 외부 교육회차 C-2609: 교육명 `보안 기초`, 시작일 `2026-09-21`, 종료일 `2026-09-22`. 수강자 L-104: 사번 `104`, 이름 `김다온`, 연락처 `daon@example.test`. 판정 J-104: 판정번호 `J-104`, 결과 수료, 판정일 `2026-09-22`. 이 묶음의 외부 일치를 전제한다. 초기 업무 필드는 수강자 세 필드이며 교육/판정 필드는 아직 업무에서 확보하지 않았다.

1. 교육담당이 교육 조회의 찾음, 판정 조회의 찾음을 거쳐 발급한다. 수료 발급 입력은 위 세 existing records, 직접 수료판정값 `수료`, 문서번호 `CERT-104-A`, 발급일 `2026-09-23`이다.
2. 문서는 하나이며 제목은 `보안 기초 수료 확인`, 본문은 `김다온님 / 보안 기초 / CERT-104-A`, 종류는 최초다. 판정 J-104와 수강자 L-104에 연결된다. 발급내역에서 그 문서를 문서 조회의 찾음으로 확보한 뒤 완료한다.
3. 외부 알림 요청 수신 payload: 알림선택 통지, 알림제목 `수료 문서 안내`, 위 문서, L-104. 알림 하나의 내용은 `[수료 문서 안내] 김다온님, 문서 CERT-104-A가 준비되었습니다.`이며 연락처는 `daon@example.test`, 문서와 수신자 관계가 각각 존재한다.
4. 알림함에서 알림 조회 찾음 후 전달 접수: 전달선택 요청, 접수번호 `OUT-104-A`, 요청일 `2026-09-23`, 전달희망일 `2026-09-24`, existing 알림. 접수 한 개가 같은 문서번호·연락처를 기록하고 그 알림에 연결된다. 전달 조회 찾음 후 완료한다.

**기대 문서 증거.** 전 과정을 연결하는 화면/action/outcome과 두 업무의 완료 계약; 위 literal를 설명할 정확한 producer·template·관계; 실제 발송 성공은 주장하지 않는 접수 의미. 초기 dataset이 존재한다는 이유만으로 lookup을 생략하는 산출물은 R17/R35/R37/R40의 충족 증거가 부족하다. 이 여정은 R01–R22, R28–R35, R37, R39–R42를 주로 확인하며 모델·역할·화면을 함께 점검한다.

## 사람 평가 여정 2 — 이름 정정, 조회 복구, 감사 열람

**합성 입력.** 여정 1의 최초 문서 `CERT-104-A`, 판정 J-104와 L-104를 기존 레코드로 제공한다. 정정 업무 시작 시 원문서 전체 필드와 learner 세 필드를 외부에서 이미 확보했다. 접수선택 접수, 요청번호 `FIX-104-1`, 수정이름 `김다온서`, 사유 `교육기관 등록명 정정`, 접수일 `2026-09-25`, 희망처리일 `2026-09-25`로 정정 요청을 제출한다.

1. 요청 한 개는 원문서번호 `CERT-104-A`를 기록하고 원문서에 연결된다. 정정 처리의 정정요청 조회에서 시간초과를 선택해 본다. 해당 조회 버튼의 팝업과 같은 조회 재시도 연결이 있어야 하며 timeout 자체는 요청 필드를 확보하지 않는다. 재시도의 찾음 후 요청 필드를 확보한다.
2. 정정결정 보류로 호출하면 문서를 만들지 않고 보류 안내가 나온다. 이후 외부에서 승인 결정을 별도로 제공한 호출은 existing 요청·원문서·판정·수강자, 직접 승인, 새 문서번호 `CERT-104-B`, 발급일 `2026-09-26`을 사용한다. 새 문서 하나의 표시이름은 `김다온서`, 교육명은 `보안 기초`, 종류는 정정, 본문은 `김다온서님 / 보안 기초 / CERT-104-B`다. 수강자 기존 이름 `김다온`이 출처가 되어서는 안 된다.
3. 새 문서의 판정·수신자 연결은 직접 받은 J-104·L-104다. 원문서와 요청은 그대로 남는다. 새 문서 조회 찾음 이후 정정 업무를 완료한다. 이어 새 문서와 L-104를 담은 통지 payload를 보내면 같은 알림 계약으로 처리할 수 있어야 한다.
4. 감사열람자가 발급내역에서 새 문서 조회의 없음 메시지·같은 조회 재시도·찾음을 거쳐 열람 완료한다. 감사열람자의 정정 발급·전달 접수는 금지다. 화면 역할과 버튼 정책을 별도로 확인한다.

**기대 문서 증거.** 정정 요청 producer/관계, 정정결정의 세 값, 정정 문서의 정확한 필드 출처, 양 문서 보존, 해당 조회 버튼에 묶인 timeout/없음 복구, 쓰기 없는 감사 완료. 문서가 실제로 이 시나리오를 실행했다고 주장할 필요는 없다. 주 coverage는 R08/R11–R18/R21/R23–R31/R36/R38–R42다.

## 사람 평가 여정 3 — 미발급과 생략을 끝까지 빠뜨리지 않기

**합성 입력.** 교육회차 C-ONE: 교육명 `당일 안전교육`, 시작일·종료일 모두 `2026-10-01`. 수강자 L-205: 사번 `205`, 이름 `오하린`, 연락처 `harin@example.test`. 판정 J-205의 결과와 직접 결정값을 각각 일치시켜 미수료인 호출과 검토중인 호출을 별도로 검토한다. 외부 계약을 의도적으로 어기는 입력은 사용하지 않는다.

1. 두 수료 발급 호출 모두 발급문서가 생성되지 않으며 미수료/검토중 안내가 구별된다. 화면의 완료 이동이나 action의 성공처럼 보이는 이름만으로 문서 완료 필드를 확보할 수 없다. 교육 날짜가 같다는 이유로 거절하지 않는다.
2. 별도 기존 원문서 `CERT-205-OLD`를 외부에서 적법하게 제공한다. 정정 요청 접수의 취소는 요청을 만들지 않는다. 유효한 existing 요청 `FIX-205-OLD`를 별도로 제공한 정정결정 반려와 보류의 호출도 각각 새 문서를 만들지 않는다. 요청의 접수일·희망처리일이 모두 `2026-10-02`라면 날짜 순서를 충족한다.
3. 별도 existing 문서와 L-205를 담은 알림선택 생략 사건은 알림을 만들지 않는다. 별도 existing 알림 `NOTICE-205-OLD`의 전달선택 보류는 접수를 만들지 않는다. 알림 목록 방문이나 보류 안내만으로 새 전달접수 완료 필드를 확보하지 않는다.
4. 같은 existing 알림을 사용해 요청일·전달희망일 모두 `2026-10-03`인 요청 호출은 날짜 순서를 충족하며 접수 하나를 만들 수 있다. 전달 조회 시간초과는 접수 필드를 새로 확보하지 않는다. 같은 조회를 재시도해 찾음인 경우에만 완료 필드를 확보한다.

**기대 문서 증거.** 정상 여정에서 사용하지 않은 enum variant를 포함한 명시 Skip, 동일 날짜 허용, 실패·skip·방문의 비획득, 호출별 외부 입력 전제. 과거 문서·요청·알림은 이 하위 검토를 위한 외부 기존 데이터이며 미생성 분기에서 새로 생긴 것으로 설명하면 안 된다. 핵심 coverage는 R02/R03/R11/R16–R19/R23/R25/R27–R29/R32/R34/R37/R39–R42다. inverted-date 예시는 필수 평가 입력으로 추가하지 않는다.

## 허용 가능한 다른 표현

- 한국어 표시명·ID·모듈 분리·절 순서, 동등한 화면 분류 구조는 자유다. 7 record models, 8 screens, 10 actions와 1 event의 의미를 유지하는 내부 이름 변경은 감점하지 않는다.
- 최종 한국어 명세의 표·문장·결과 흐름 등으로 같은 의미를 제공할 수 있다. 이 비교에서 RSPDL 자체는 최종 제출 형식이 아니며 compile pass로 이 rubric을 대체하지 않는다.
- 접수선택·전달선택 등 display label은 동의어로 바꾸어도 닫힌 domain과 전 값의 의도가 보존되면 인정한다. 각 output 별 Create/Skip는 명시적이어야 한다.
- 화면 안의 합리적 구역·폼 배치나 제목 문구 차이는 허용한다. R21/R30이 지정한 문서·알림 template은 정확한 문구 계약이므로 임의 변경하지 않는다.
- output field와 relation producer를 조건 없는 binding으로 공유하는 표현은 적절하다. Skip 경로에는 실제 output이 없다는 의미를 유지하면 그 경로의 값 생성을 별도로 요구하지 않는다.
- 정적 checker가 조건부 생성의 workflow availability를 unknown으로 남기는 것은 제품 요구 누락과 동일하지 않다. 이 제품은 완료 직전에 source-backed lookup을 요구하므로 generator가 임의 필드 획득을 만들어 unknown을 숨길 필요가 없다. 이 재조회 계약이 전체 workflow의 정적 증명을 보장한다는 뜻은 아니다.

## 현재 지원 근거와 한계

다음 파일을 실제로 읽어 범위를 확인했다. 오래된 실험·gold/reference fixture를 참고하지 않았다.

| 의미 | 지원 문서·conformance·구현 근거 |
|---|---|
| 현재 구현/비범위, 역할·화면·데이터·생산자 | `README.md`의 현재 구현된 범위 및 통합 statement 검증 범위; `docs/rfcs/0008-conditional-data-production.md` 구현된 문형 |
| direct enum conditional ExactlyOne Create/Skip와 coverage | `conformance/ko-KR/conditional-production/normal-exhaustive-optional/input.rspdl`; `crates/rspdl-domain/src/analysis.rs`의 production 분석 |
| typed Action input 및 scalar/ExistingModel field/constant producer | `conformance/ko-KR/field-producers/normal/input.rspdl`; `conformance/ko-KR/field-producers/boundary-explicit-values/input.rspdl`; `crates/rspdl-ko/src/lowering.rs` |
| immutable Event enum production·field·template·relation | `conformance/ko-KR/events/normal/input.rspdl`; `docs/rfcs/0008-conditional-data-production.md`의 TriggerPayload 계약; `crates/rspdl-domain/src/analysis.rs` 1644–1678행의 Event producer phase |
| same-output String template와 dependency | `conformance/ko-KR/message-templates/normal/input.rspdl`; `docs/rfcs/0008-conditional-data-production.md`; `crates/rspdl-domain/src/analysis.rs` 2539–2585행의 String placeholder 검사 |
| required+unique output-first binary relation producer | `conformance/ko-KR/relation-producers/normal/input.rspdl`; 같은 RFC의 ExactlyOne relation-slot 절; `crates/rspdl-domain/src/analysis.rs` 2050–2083행의 output slot 조건 |
| 같은 레코드 field 날짜 순서와 inclusive boundary | `conformance/ko-KR/field-ordering/normal/input.rspdl`; `conformance/ko-KR/field-ordering/boundary-equal-inclusive/input.rspdl` |
| 성공 lookup의 source-backed field acquisition, screen role | `conformance/ko-KR/action-outcomes/normal-lookup/input.rspdl`; `docs/rfcs/0012-typed-action-outcomes-and-recovery.md` |
| explicit initial field availability·업무 start/completion·submit 획득 | `conformance/ko-KR/workflow-data/boundary-explicit-initial-data/input.rspdl`; `conformance/ko-KR/workflow-data/normal-alternative-producers/input.rspdl`; `docs/rfcs/0010-workflow-completion-data-availability.md` |

**조건부 workflow의 중요한 지원 한계.** `crates/rspdl-domain/src/analysis.rs`의 `has_conditional` 및 `verification_unknown` 처리(지원 기준 HEAD의 약 3730–3790행)는 reachable Action production에 Skip branch 또는 conditional field producer가 있으면 `RSPDL-WORKFLOW-U001` 경고를 남기고 그 workflow의 완료 필드 가용성 검사를 건너뛴다. E02의 발급·정정·전달 업무가 여기에 해당할 수 있다. 재조회 경로를 쓴다고 이 전역 한계가 사라지는 것은 아니다. workflow start/completion·source-backed lookup·producer의 구조와 보존은 현재 문법으로 표현하며, 조건별 완료 데이터의 실제 사업 의미는 이 비공개 rubric과 사람 여정으로 검토한다. 이 경고를 제거할 목적으로 Skip·입력 출처·완료 의무를 삭제한 산출물은 정당화되지 않는다. 경고 자체는 요구 위반으로 채점하지 않는다.

**외부 초기 레코드와 전역 lifecycle의 별도 한계.** 같은 구현의 `analyze_data_usage`는 workflow outcomes 분석에 앞서 호출되며(약 412행), 전역 field availability를 화면 Input과 derivation으로 시작하고 model creator를 화면/행동의 생성 선언에서 찾는다. 약 3460–3497행에서 이 집합에 없는 화면 Read 필드·모델은 `RSPDL-DATA-001/002` 오류를 낸다. 명시적 ExistingModel 입력, workflow 초기 데이터와 source-backed 조회만으로 이 전역 검사가 충족되는 것은 아니다. 따라서 외부 read-only 세 모델을 보존한 E02 RSPDL은 이 지원 불일치 진단을 받을 수 있다. 이는 초기 데이터 계약을 문법으로 쓸 수 없다는 뜻은 아니지만, 현재 compiler가 그 계약을 전역 lifecycle에 통합해 증명하지 못한다는 한계다. `normal-lookup` 예제에는 별도 생성·입력 화면이 있으므로 그것을 외부 공급 지원의 증거로 확대하지 않는다. 숨은 수강자/교육/판정 생성 화면을 넣어 R03/R42를 바꾸지 않는다. 이 알려진 진단과 사업 요구 누락을 구분해 기록하고, clean compile을 E02의 합격 조건으로 추가하지 않는다.

**평가자만 확인할 경계.** Action field values는 PreMutation 입력, Event field values는 불변 TriggerPayload다. Event constant·조건부 field producer를 요구하지 않는다. relation producer는 direct ExistingModel 입력에서 output-first Required+Unique binary slot만 연결한다. 실제 relation 데이터 binding/join·snapshot·레코드 일치 검증은 지원한다고 주장할 수 없다. template는 같은 output의 String 필드만 참조하며 source-order priority는 없다. 일반 문자열 보간으로 날짜나 enum을 format하도록 요구하지 않는다.

무조건 role allow/deny는 지원되지만 임의 조건부 권한·소유권 추론은 요구하지 않는다. 화면 flow의 조건 설명은 실행 가능한 enum 정책이 아니다. source-backed lookup outcome은 성공 호출의 provenance/availability 계약이며 서버가 조회를 실행했다는 증명이 아니다. 초기 데이터 선언은 외부 전제다. immutable event가 실제로 외부에서 도착했는지, 결정값이 적법한지, 독립 입력들이 일치하는지, 실제 재시도가 중복을 방지하는지는 이 제품 평가에서 검사할 runtime 보장이 아니다.

통합 statement의 compiler 검사 범위와 실행 범위를 혼동하지 않는다. `자동 시도` 문장이나 compile pass는 발급·전송을 실행하지 않는다. 문서에서 요구한 사업 의미는 explicit production, provenance, relationship, permission, workflow, outcome 증거로 각각 검토한다. bounded model finder의 성공도 실제 고객 데이터·전송·업무 전 경로의 사업 완전성을 입증하지 않는다.
