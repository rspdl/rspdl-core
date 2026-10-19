---
id: evaluation-products
title: 제품 전체 요구사항의 평가용 사례 3개
type: guide
status: draft
version: "1"
summary: Defines three evaluation-only product briefs with separate human and Sol rubrics, supported-grammar reviews, and an unexecuted paired comparison protocol.
topics:
  - evaluation
  - product-planning
  - data-lifecycle
related:
  - unified-statements
problem_refs:
  - data-lifecycle-modeling-gap
  - policy-consistency-blind-spots
last_updated: "2026-10-19"
---

# 제품 전체 요구사항의 평가용 사례 3개

[평가 자료 시작점](../../experiments/evaluation-products/README.md)에서 제품별 입력과 판정 기준을 읽는다.
기존 구매·재고 제품과 작은 과제는 개발용으로 남기고, 새로운 세 제품은 모델의 제품 명세 작성
품질을 평가하는 데 사용한다. 테스트 코드·정답 구현·명세 실행기를 추가하지 않는다.

| 제품 | 평가에서 읽을 연결 |
| --- | --- |
| 수리 접수부터 고객 인계까지 | 두 진입 경로, 필수 데이터 확보, 실패·시간 초과·재시도 뒤의 완료 |
| 교육 이수 증빙 발급과 정정 | 외부 판정, 조건별 생성·생략, 필드의 출처, 증빙과 안내의 연결 |
| 시설 점검 보고서 보완과 인계 | 주체·복합 조건·입력에 따른 허용·금지·시도, 변경과 후속 안내 |

제품마다 대표 여정 3개에 설명용 제목을 붙인다. 여정은 최종 명세를 따라 읽는 보조 판정이며,
독립 제품이나 실행 테스트가 아니다. 전체 요구사항을 빠짐없이 평가하는 것이 주 기준이다.

작성·문법 검토·평가 공정성 검토를 Sol 6.1의 별도 문맥으로 진행한다. 현재 문법과 소스에서
확인한 표현·검사 범위를 구분하고, 미지원 기능을 넣어 일부러 점수를 낮추지 않는다.
런타임에서 실행되지 않는 statement를 업무 수행의 증명으로 보지 않는다.

주 비교는 같은 모델의 자체 검토와 RSPDL 작성·실제 진단 활용이다. 두 군 모두 최종 한국어
제품 명세를 제출하며 Sol 6.1은 실험군을 숨긴 상태에서 요구사항별 근거를 판정한다.
사람 확인 전 결과는 잠정 평가로 표시한다. 상세 지시·분모·실패 처리·파일 분리는
[실험 운영](../../experiments/evaluation-products/generation-protocol.md)을 따른다.

현재 생성 모델 실행과 점수 측정은 하지 않았다. 100%·악화·미지원도 숨기지 않는다.
결과가 나쁘면 기준을 쉽게 바꾸는 대신 원인을 기록하고 코드나 품질을 개선한다.
사례 자체의 오류가 발견되면 수정 내역을 공개하고 새 버전으로 양쪽 군을 재평가한다.
개선에 사용한 사례에서의 점수와 새로운 제품에 대한 일반화는 구분한다.
