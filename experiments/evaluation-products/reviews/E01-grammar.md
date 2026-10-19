# E01 grammar and implementation-boundary review

Verdict: **ready for independent fairness review; no unresolved grammar blocker found** in the completed `cases/E01-repair/input.md` and `evaluator.md` (E01-R01–R35). The product retains requirements that need human semantic review, and the evaluator identifies those limits rather than treating compilation as their proof.

This is a source-and-contract review. I read repository `AGENTS.md`, followed `.agents/skills/discover-rspdl-knowledge/SKILL.md`, inspected the relevant RFC sections, parser/analyzer source, and conformance/test definitions. I did **not** run the compiler, Cargo, tests, an application, or a runtime simulation. Referenced tests describe intended checks; their execution was not verified in this review. I did not read earlier experiment gold/reference answers, write an RSPDL solution, or modify the case files.

## Findings and resolutions

1. **Material implementation limit, now accurately disclosed — E01-R21–R23, R25–R33.** Explicit `업무.획득` is bound to a source button, not to success outcomes. `crates/rspdl-domain/src/analysis.rs:3694–3699` applies it to every target-bearing edge from that button, and `workflow_data_witness` repeats that behavior at `4437–4448`. Only source-backed outcome `제공 데이터` checks `OutcomeKind::Success` (`3700–3705`, `4450–4455`); non-success provided data is rejected at `4145–4156`. The completed evaluator's “특히 혼동하지 말아야 할 현재 구현 제한” states this distinction correctly. The prompt uses different submit/cancel buttons and same-screen failure/timeout handlers, with success alone advancing. This avoids relying on unsupported same-button success filtering. Preserve this caveat in later reporting; a compiler pass cannot by itself establish that failed submissions acquired nothing or saved nothing.

2. **Success-only creation remains required human-reviewed intent — E01-R21, R25–R28, R31, R35.** Model `생성` mutations register structural model producers (`analysis.rs:3245–3247`; `docs/rfcs/0005-field-provenance-and-sum-derivation.md:100–107`). They do not execute persistence or bind a mutation to a successful outcome. The evaluator explicitly retains successful-creation intent and assigns its execution/conditionality to human review. No requirement weakening is needed or recommended.

3. **External input, lookup, and producer are correctly separated — E01-R03–R04, R11–R12, R17–R19, R24, R33.** The external register supplies a selected existing record, while workflow starts with zero acquired fields. The manual route supplies an internal structural creation/input path. `조회 결과` links action/existing-model input/model/fields (`analysis.rs:3868–3949`); the success outcome must adopt matching lookup data (`3990–4037`). Screen READ is a consumer, not an acquisition or field producer (`3144–3153`). Initial workflow data would be only an external precondition (`docs/rfcs/0010-workflow-completion-data-availability.md:61–66`). The evaluator rejects treating record input, a READ declaration, or external existence as successful acquisition.

4. **Date requirements fit current expression support — E01-R04, R07–R10.** Both comparisons concern two fields of the same record and expressly allow equality. Parser support is in `crates/rspdl-ko/src/parser.rs:1719–1790` and the shadow grammar in `crates/rspdl-ko/src/grammar/constraint.ebnf`. `crates/rspdl-compiler/tests/field_ordering.rs:20–81` specifies strict/inclusive boundaries; `85–111` distinguishes optional absence and datetime instants; `247–279` specifies comparable types. Date value validity and concrete ordering need supplied values and the appropriate check API; compilation of a field declaration alone does not establish actual entered values or a working save-blocking UI. The evaluator correctly requires neither cross-record chronology nor invented SLA/calendar rules.

5. **Role declarations fit unconditional policy support — E01-R13–R15.** The two roles and eight actions have a fully stated allow/deny matrix without ownership/state conditions. Current screen permission analysis compares exact role/action/model/field tuples and returns Unknown without a matching field policy (`analysis.rs:4735–4795`). The evaluator documents this caveat and treats action-level matrix completeness as a separate document review. Screen role declarations do not prove authentication, session switching, or the physical handoff.

6. **Handlers and bounded recovery fit static support — E01-R18–R23, R29–R30, R32.** `validate_outcome_handlers` checks each originating screen/button/action and outcome, and validates recovery from that handler's reachable screens (`analysis.rs:5027–5178`). A same-action button elsewhere is insufficient. `crates/rspdl-compiler/tests/action_outcome_conformance.rs` contains corresponding named cases for exact handlers, non-success data rejection, and reachable retry targets. These checks establish declared reachability/ownership, not that the user retries or that the retry succeeds. The evaluator makes that distinction explicit.

7. **Minor prompt ambiguity fixed — E01-R15–R18.** The original R15 could be read as requiring a new domain action for every navigation selector. R15 now distinguishes data-handling buttons from pure navigation buttons. The evaluator agrees that the start selectors and manual-route switch need destinations without expanding the eight business actions. No hidden ninth action is required.

## Coverage classification

“Source-defined check” below identifies inspected implementation logic, not a compiler run on this product.

| Requirement area | Supported expression | Source-defined check | Human-only part retained |
| --- | --- | --- | --- |
| R02, R04–R08, R21, R24–R28, R34 | Nonempty record schemas, required typed fields, screen create/input/read, model create mutation | Model/field references, structural producers, layout/input consistency | Business meaning of fields, actual values, save/return occurrence |
| R09–R10 | Same-record date field ordering, inclusive boundary | Operand type compatibility; concrete values can be checked by separate API | Actual UI enforcement and absence of invented deadline policies |
| R11–R12, R17 | Existing-model action input, source-backed lookup, success-provided fields, explicit workflow start | Exact lookup ownership/model/field and success-data binding | External existence/selection and response success |
| R16–R33 | Explicit source buttons, outcomes/handlers, input acquisitions, workflow completion field obligations | References, reachability, placement, conservative field availability/witnesses subject to finding 1 | Success-only persistence, record continuity, state reset and actual execution |
| R13–R15 | Roles, unconditional field-scoped policies and screen bindings | Exact declared tuple checks, role/action exposure | All forbidden role/action combinations remain covered; authentication and real actor handoff |
| R18–R23, R29–R30 | Retry/return obligations, same-screen message/popup handlers | Exact originating-button outcome and reachable recovery target | Closing popups, preserving/editing inputs, retry invocation/success/idempotence |
| R01, R03, R14, R35 | Explicit scope and prose intent | No general compiler proof claimed | Single physical device/customer, current-record continuity, implementation exclusions |

## Standalone prompt and evaluator assessment

The five records, 18 fields, eight screens/actions, two routes, role matrix, external selected-record boundary, zero initial field availability, two inclusive date rules, per-action outcomes, and one completion obligation are explicit in the public input. The evaluator maps every R01–R35 once and adds three document walkthroughs without requiring a generated RSPDL answer. Its navigation/action distinction and source map match the completed input.

The core task does **not** require automatic joins, record search, cross-record aggregates or date comparisons, conditional role evaluation, runtime transaction execution, timeout duration calculation, or concurrency guarantees. R03 and R14 preserve same-record and actor-handoff intent as human-reviewed obligations; these must not be silently dropped from candidate evaluation. Actual persistence, retry, UI state retention, and physical return also remain human-reviewed intentions, with execution guarantees explicitly outside R35.

No unresolved fix is requested. Retain the implementation limits above and do not relabel this source audit, future compilation, or document walkthroughs as runtime verification or evidence of product efficacy.
