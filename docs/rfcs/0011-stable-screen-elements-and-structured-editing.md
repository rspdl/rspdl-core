---
id: stable-screen-elements-and-structured-editing
title: 안정적 화면 요소 식별과 컴파일러 소유 구조화 편집
type: rfc
status: implemented
version: "1"
summary: Defines optional explicit IDs for every semantic layout element and a source-hash guarded compiler edit API that returns compiled candidates without saving.
topics:
  - screen-layout
  - stable-id
  - structured-editing
  - source-hash
  - sdk
  - canonical-ir
related:
  - document-frontmatter-structure
  - frontend-semantic-analysis-contract
  - workflow-completion-data-availability
problem_refs:
  - screen-structure-spec-divergence
  - semantic-source-provenance-loss
last_updated: "2026-09-23"
owners:
  - rspdl-maintainers
target_spec: "0.7.0"
---

# 안정적 화면 요소 식별과 컴파일러 소유 구조화 편집

## 실패 시나리오와 최소 범위

현재 버튼만 명시적 ID가 있다. 입력·목록·제목·폼·구역을 배열 위치나 source span으로 기억하면 앞에
요소 하나를 추가했을 때 저장한 배치와 편집 대상이 다른 요소로 옮겨간다. application이 YAML을 직접
고치면 compiler frontend와 두 번째 parser가 생긴다.

모든 layout element는 선택적인 명시적 `id`를 가질 수 있다. 기존 shorthand는 계속 유효하지만 ID가
없는 요소는 구조화 편집과 영구 디자인 metadata의 대상으로 주소 지정할 수 없다. 이름·배열 index·span
으로 ID를 합성하지 않는다. ID는 한 화면의 전체 중첩 트리에서 유일하다.

## 표면 문법과 Canonical IR

기존 scalar/container shorthand와 함께 mapping form을 받는다.

```rspdl
- 구역:
    id: passenger_section
    자식:
      - 폼:
          id: passenger_form
          입력:
            - 입력: { id: contact_input, 필드: 연락처 }
- 제목: { id: title, 글: "승객 정보" }
- 목록: { id: flights, 모델: 항공편, 필드: [편명] }
- 자리: { id: seat_map, 이름: "좌석 배치" }
- 버튼: { id: submit, 이름: "다음" }
```

머리말·구역은 `자식`, 폼은 `입력`을 쓴다. 제목은 `글`, 입력은 `필드`, 자리는 `이름`을 쓴다.
버튼의 기존 필수 `id` 계약은 유지한다. 각 `LayoutElement` IR variant는 `id?: String`과 element span을
보존한다. ID가 없는 기존 요소는 serialization에서 필드를 생략한다.

## 구조화 편집 SDK

edit envelope의 `schema_version`은 compile wire schema와 독립적으로 1에서 시작한다. 대상 문서는
`source: { path, text }`, 대상 요소는 canonical `screen_id`와 explicit `element_id`로 전달하며
`expected_source_hash`로 원문을 잠근다. `source_hash`는 source
text의 UTF-8 byte sequence를 그대로 SHA-256한 lowercase hex다. project/workspace hash와 같다고 가정하지
않는다.

첫 operation은 `insert`, `delete`, `move`, `update`, screen path `connect`와 `disconnect`다. insert/move는
`parent_element_id?`, `slot = root|children|inputs`, `before_element_id?`를 명시한다. update는 element kind가
허용하는 label/field/model/action 속성만 바꾼다. field/model/action 참조는 frontend가 쓰는 stable reference로
검증한다.

Python과 Node SDK는 각각 `source_hash(text)`와 `sourceHash(text)`를 제공하므로 호출자가 알고리즘을 다시
구현할 필요가 없다. 응답은 입력·후보 source hash, candidate text, compile result와 diagnostics,
stable-ID `id_remap`/`tombstones`,
locale, compiler version과 wire schema version을 반환한다. compiler는 저장하지 않는다. stale hash,
중복/모호 target, ID 없는 target과 잘못된 slot은 compiler diagnostic과 구분되는 structured edit outcome이다.
변환 뒤 compile error가 생기면 candidate와 diagnostics를 함께 반환해 사람이 판단할 수 있게 한다.
첫 버전은 ID를 합성하거나 바꾸지 않으므로 `id_remap`은 빈 객체이고, container 삭제는 모든 명시적
하위 ID를 tombstone으로 함께 반환한다.

첫 구현은 AST를 바꾼 뒤 formatter로 전체 문서를 정규화할 수 있다. 이 경우 호출자는 원문과 candidate의
diff를 반드시 검토할 수 있어야 한다. 장기적으로 source-preserving patch를 넓히되 application parser를
추가하지 않는다.

## 호환성과 비범위

기존 compile/format schema version 1 소비자는 새 optional `id`와 `workflows` 필드를 무시할 수 있다.
필드 제거나 기존 필드 의미 변경은 없다. edit envelope는 별도 API라 compile wire 응답을 바꾸지 않는다.
좌표·크기·색은 application이 `{screen_id, element_id}`로 보관한다. C3 typed outcome, 정책 실행, 여러 파일을
원자적으로 편집하는 workspace edit는 이 RFC 범위 밖이다.

## References

- [Screen Structure and Flow Spec Divergence](../problems/0006-screen-structure-spec-divergence.md)
- [Semantic Source Provenance Loss](../problems/0005-semantic-source-provenance-loss.md)
- [Document Frontmatter Structure](0009-document-frontmatter-structure.md)
