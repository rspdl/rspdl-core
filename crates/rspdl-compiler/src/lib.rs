//! RSPDL compiler and execution facade.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use rspdl_datalog::DatalogEvaluator;
use rspdl_domain::{
    Atom, BooleanExpression, CanonicalId, CanonicalType, CanonicalValue, ConstraintOperand,
    ConstraintProblem, ConstraintSolver, DerivationRule, Diagnostic, Fact, Frontend,
    FrontendOutput, LogicProgram, PolicyEffect, PredicateApplication, PredicateSignature,
    RelationOperator, RuleLiteral, SemanticModule, Severity, SolveOptions, SolveResult, Term,
    TextRange, Variable, analyze,
};
use rspdl_ko::KoreanFrontend;
use rspdl_solver_z3::Z3Solver;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Compilation {
    pub module: Option<SemanticModule>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Source {
    pub path: String,
    pub text: String,
}

impl Source {
    pub fn new(path: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            text: text.into(),
        }
    }
}

/// Backwards-compatible name for Korean-only callers.
pub type KoSource = Source;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FileCompilation {
    pub path: String,
    pub module: Option<SemanticModule>,
    pub diagnostics: Vec<Diagnostic>,
    #[serde(skip)]
    declaration_span: TextRange,
}

impl FileCompilation {
    fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(Diagnostic::is_error)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct WorkspaceCompilation {
    pub files: Vec<FileCompilation>,
}

impl WorkspaceCompilation {
    pub fn has_errors(&self) -> bool {
        self.files.iter().any(FileCompilation::has_errors)
    }

    fn modules(&self) -> Vec<&SemanticModule> {
        self.files
            .iter()
            .filter_map(|file| file.module.as_ref())
            .collect()
    }
}

pub fn compile_ko(source: &str) -> Compilation {
    compile_with_frontend(&KoreanFrontend, source)
}

/// Compiles any surface language that implements the shared frontend contract.
pub fn compile_with_frontend(frontend: &dyn Frontend, source: &str) -> Compilation {
    compile_frontend_output(frontend.lower_source(source)).0
}

fn compile_frontend_output(output: FrontendOutput) -> (Compilation, TextRange) {
    let declaration_span = output
        .module
        .as_ref()
        .map_or(TextRange::default(), |module| module.declaration.span);
    let mut diagnostics = output.diagnostics;
    let module = if diagnostics.iter().any(Diagnostic::is_error) {
        None
    } else if let Some(module) = output.module {
        let analyzed = analyze(module);
        diagnostics.extend(analyzed.diagnostics);
        analyzed.module
    } else {
        None
    };
    diagnostics.sort_by(|left, right| {
        (left.span.start, left.span.end, &left.rule_id, &left.message).cmp(&(
            right.span.start,
            right.span.end,
            &right.rule_id,
            &right.message,
        ))
    });
    (
        Compilation {
            module,
            diagnostics,
        },
        declaration_span,
    )
}

pub fn compile_ko_files(sources: Vec<KoSource>) -> WorkspaceCompilation {
    compile_files_with_frontend(&KoreanFrontend, sources)
}

/// Compiles a workspace with any conforming surface-language frontend.
pub fn compile_files_with_frontend(
    frontend: &dyn Frontend,
    mut sources: Vec<Source>,
) -> WorkspaceCompilation {
    sources.sort_by(|left, right| (&left.path, &left.text).cmp(&(&right.path, &right.text)));
    let mut files = sources
        .into_iter()
        .map(|source| {
            let (compilation, declaration_span) =
                compile_frontend_output(frontend.lower_source(&source.text));
            FileCompilation {
                path: source.path,
                module: compilation.module,
                diagnostics: compilation.diagnostics,
                declaration_span,
            }
        })
        .collect::<Vec<_>>();

    let mut path_sources = BTreeMap::<String, Vec<usize>>::new();
    for (index, file) in files.iter().enumerate() {
        path_sources
            .entry(file.path.clone())
            .or_default()
            .push(index);
    }
    for (path, source_indexes) in path_sources {
        if source_indexes.len() < 2 {
            continue;
        }
        for index in source_indexes {
            files[index].diagnostics.push(Diagnostic {
                rule_id: "RSPDL-SOURCE-001".into(),
                severity: Severity::Error,
                message: format!("source 경로 `{path}`가 중복 지정되었습니다."),
                span: Default::default(),
            });
        }
    }

    let mut module_sources = BTreeMap::<CanonicalId, Vec<usize>>::new();
    for (index, file) in files.iter().enumerate() {
        if let Some(module) = &file.module {
            module_sources
                .entry(module.id.clone())
                .or_default()
                .push(index);
        }
    }
    let mut duplicate_module_sources = BTreeSet::new();
    for (module_id, source_indexes) in module_sources {
        if source_indexes.len() < 2 {
            continue;
        }
        for index in source_indexes {
            duplicate_module_sources.insert(index);
            let span = files[index].declaration_span;
            files[index].diagnostics.push(Diagnostic {
                rule_id: "RSPDL-LINK-001".into(),
                severity: Severity::Error,
                message: format!("모듈 ID `{module_id}`가 여러 파일에 선언되었습니다."),
                span,
            });
        }
    }

    let mut symbol_sources = BTreeMap::<CanonicalId, Vec<usize>>::new();
    for (index, file) in files.iter().enumerate() {
        if duplicate_module_sources.contains(&index) {
            continue;
        }
        let Some(module) = &file.module else {
            continue;
        };
        for id in module
            .enums
            .iter()
            .map(|value| &value.id)
            .chain(module.models.iter().map(|value| &value.id))
            .chain(module.roles.iter().map(|value| &value.id))
            .chain(module.actions.iter().map(|value| &value.id))
        {
            symbol_sources.entry(id.clone()).or_default().push(index);
        }
    }
    for (symbol_id, mut source_indexes) in symbol_sources {
        source_indexes.sort_unstable();
        source_indexes.dedup();
        if source_indexes.len() < 2 {
            continue;
        }
        for index in source_indexes {
            let span = files[index].declaration_span;
            files[index].diagnostics.push(Diagnostic {
                rule_id: "RSPDL-LINK-002".into(),
                severity: Severity::Error,
                message: format!("stable ID `{symbol_id}`가 여러 파일에 선언되었습니다."),
                span,
            });
        }
    }

    for file in &mut files {
        file.diagnostics.sort_by(|left, right| {
            (left.span.start, left.span.end, &left.rule_id, &left.message).cmp(&(
                right.span.start,
                right.span.end,
                &right.rule_id,
                &right.message,
            ))
        });
    }

    WorkspaceCompilation { files }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CheckOptions {
    pub solver_timeout: Duration,
}

impl Default for CheckOptions {
    fn default() -> Self {
        Self {
            solver_timeout: Duration::from_secs(5),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RuntimeDiagnostic {
    pub rule_id: String,
    pub severity: Severity,
    pub path: String,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ConstraintViolation {
    pub constraint_id: CanonicalId,
    pub model_id: CanonicalId,
    pub record_id: String,
    pub left: CanonicalValue,
    pub right: CanonicalValue,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyStatus {
    Allowed,
    Denied,
    Conflict,
    Unmatched,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PolicyResult {
    pub request_id: String,
    pub status: PolicyStatus,
    pub allow_policies: Vec<CanonicalId>,
    pub deny_policies: Vec<CanonicalId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CheckReport {
    pub compilation: Compilation,
    pub constraint_violations: Vec<ConstraintViolation>,
    pub policy_results: Vec<PolicyResult>,
    pub runtime_diagnostics: Vec<RuntimeDiagnostic>,
}

impl CheckReport {
    pub fn has_errors(&self) -> bool {
        self.compilation
            .diagnostics
            .iter()
            .any(Diagnostic::is_error)
            || self
                .runtime_diagnostics
                .iter()
                .any(|diagnostic| diagnostic.severity == Severity::Error)
    }

    pub fn has_findings(&self) -> bool {
        !self.constraint_violations.is_empty()
            || self
                .policy_results
                .iter()
                .any(|result| !matches!(result.status, PolicyStatus::Allowed))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct WorkspaceCheckReport {
    pub compilation: WorkspaceCompilation,
    pub constraint_violations: Vec<ConstraintViolation>,
    pub policy_results: Vec<PolicyResult>,
    pub runtime_diagnostics: Vec<RuntimeDiagnostic>,
}

impl WorkspaceCheckReport {
    pub fn has_errors(&self) -> bool {
        self.compilation.has_errors()
            || self
                .runtime_diagnostics
                .iter()
                .any(|diagnostic| diagnostic.severity == Severity::Error)
    }

    pub fn has_findings(&self) -> bool {
        !self.constraint_violations.is_empty()
            || self
                .policy_results
                .iter()
                .any(|result| !matches!(result.status, PolicyStatus::Allowed))
    }
}

pub fn check_ko(source: &str, runtime_json: &str, options: CheckOptions) -> CheckReport {
    let compilation = compile_ko(source);
    let mut report = CheckReport {
        compilation,
        constraint_violations: Vec::new(),
        policy_results: Vec::new(),
        runtime_diagnostics: Vec::new(),
    };
    if report
        .compilation
        .diagnostics
        .iter()
        .any(Diagnostic::is_error)
    {
        return report;
    }
    let Some(module) = report.compilation.module.as_ref() else {
        return report;
    };
    let input = match serde_json::from_str::<RuntimeInput>(runtime_json) {
        Ok(input) => input,
        Err(error) => {
            report.runtime_diagnostics.push(runtime_error(
                "RSPDL-INPUT-001",
                "$",
                format!("JSON 형식이 올바르지 않습니다: {error}"),
            ));
            return report;
        }
    };
    let runtime = match bind_runtime(&[module], input) {
        Ok(runtime) => runtime,
        Err(mut diagnostics) => {
            diagnostics.sort_by(|left, right| {
                (&left.path, &left.rule_id, &left.message).cmp(&(
                    &right.path,
                    &right.rule_id,
                    &right.message,
                ))
            });
            report.runtime_diagnostics = diagnostics;
            return report;
        }
    };

    execute_constraints(
        &[module],
        &runtime,
        options,
        &mut report.constraint_violations,
        &mut report.runtime_diagnostics,
    );
    match execute_policies(&[module], &runtime) {
        Ok(results) => report.policy_results = results,
        Err(message) => report.runtime_diagnostics.push(runtime_error(
            "RSPDL-BACKEND-DATALOG-001",
            "$.action_requests",
            message,
        )),
    }
    report.constraint_violations.sort_by(|left, right| {
        (&left.model_id, &left.record_id, &left.constraint_id).cmp(&(
            &right.model_id,
            &right.record_id,
            &right.constraint_id,
        ))
    });
    report.runtime_diagnostics.sort_by(|left, right| {
        (&left.path, &left.rule_id, &left.message).cmp(&(
            &right.path,
            &right.rule_id,
            &right.message,
        ))
    });
    report
}

pub fn check_ko_files(
    sources: Vec<KoSource>,
    runtime_json: &str,
    options: CheckOptions,
) -> WorkspaceCheckReport {
    let compilation = compile_ko_files(sources);
    let mut report = WorkspaceCheckReport {
        compilation,
        constraint_violations: Vec::new(),
        policy_results: Vec::new(),
        runtime_diagnostics: Vec::new(),
    };
    if report.compilation.has_errors() {
        return report;
    }
    let modules = report.compilation.modules();
    let input = match serde_json::from_str::<RuntimeInput>(runtime_json) {
        Ok(input) => input,
        Err(error) => {
            report.runtime_diagnostics.push(runtime_error(
                "RSPDL-INPUT-001",
                "$",
                format!("JSON 형식이 올바르지 않습니다: {error}"),
            ));
            return report;
        }
    };
    let runtime = match bind_runtime(&modules, input) {
        Ok(runtime) => runtime,
        Err(mut diagnostics) => {
            diagnostics.sort_by(|left, right| {
                (&left.path, &left.rule_id, &left.message).cmp(&(
                    &right.path,
                    &right.rule_id,
                    &right.message,
                ))
            });
            report.runtime_diagnostics = diagnostics;
            return report;
        }
    };

    execute_constraints(
        &modules,
        &runtime,
        options,
        &mut report.constraint_violations,
        &mut report.runtime_diagnostics,
    );
    match execute_policies(&modules, &runtime) {
        Ok(results) => report.policy_results = results,
        Err(message) => report.runtime_diagnostics.push(runtime_error(
            "RSPDL-BACKEND-DATALOG-001",
            "$.action_requests",
            message,
        )),
    }
    report.constraint_violations.sort_by(|left, right| {
        (&left.model_id, &left.record_id, &left.constraint_id).cmp(&(
            &right.model_id,
            &right.record_id,
            &right.constraint_id,
        ))
    });
    report.runtime_diagnostics.sort_by(|left, right| {
        (&left.path, &left.rule_id, &left.message).cmp(&(
            &right.path,
            &right.rule_id,
            &right.message,
        ))
    });
    report
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RuntimeInput {
    #[serde(default)]
    records: BTreeMap<String, Vec<Map<String, Value>>>,
    #[serde(default)]
    role_assignments: Vec<RoleAssignmentInput>,
    #[serde(default)]
    action_requests: Vec<ActionRequestInput>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RoleAssignmentInput {
    actor: String,
    role: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ActionRequestInput {
    #[serde(rename = "$id")]
    id: String,
    actor: String,
    model: String,
    record: String,
    field: String,
    action: String,
}

#[derive(Clone, Debug)]
struct BoundRecord {
    id: String,
    values: BTreeMap<CanonicalId, CanonicalValue>,
}

#[derive(Clone, Debug)]
struct BoundAction {
    id: String,
    actor: String,
    model_id: CanonicalId,
    record_id: String,
    field_id: CanonicalId,
    action_id: CanonicalId,
}

#[derive(Clone, Debug)]
struct BoundRuntime {
    records: BTreeMap<CanonicalId, Vec<BoundRecord>>,
    roles: Vec<(String, CanonicalId)>,
    actions: Vec<BoundAction>,
}

fn bind_runtime(
    modules: &[&SemanticModule],
    input: RuntimeInput,
) -> Result<BoundRuntime, Vec<RuntimeDiagnostic>> {
    let mut diagnostics = Vec::new();
    let models = modules
        .iter()
        .flat_map(|module| module.models.iter())
        .map(|model| (model.id.as_str(), model))
        .collect::<BTreeMap<_, _>>();
    let mut records = BTreeMap::new();
    let mut record_ids = BTreeMap::<CanonicalId, BTreeSet<String>>::new();

    for (model_text, values) in input.records {
        let Some(model) = models.get(model_text.as_str()).copied() else {
            diagnostics.push(runtime_error(
                "RSPDL-INPUT-010",
                format!("$.records.{model_text}"),
                format!("데이터 모델 `{model_text}`이 선언되지 않았습니다."),
            ));
            continue;
        };
        let mut bound = Vec::new();
        let mut ids = BTreeSet::new();
        let known_fields = model
            .fields
            .iter()
            .map(|field| (field.local_id.as_str(), field))
            .collect::<BTreeMap<_, _>>();
        for (index, object) in values.into_iter().enumerate() {
            let path = format!("$.records.{model_text}[{index}]");
            let id = match object.get("$id").and_then(Value::as_str) {
                Some(id) => id.to_owned(),
                None => {
                    diagnostics.push(runtime_error(
                        "RSPDL-INPUT-011",
                        format!("{path}.$id"),
                        "레코드에는 문자열 `$id`가 필요합니다.",
                    ));
                    continue;
                }
            };
            if !ids.insert(id.clone()) {
                diagnostics.push(runtime_error(
                    "RSPDL-INPUT-012",
                    format!("{path}.$id"),
                    format!("레코드 ID `{id}`가 중복되었습니다."),
                ));
            }
            for key in object.keys() {
                if key != "$id" && !known_fields.contains_key(key.as_str()) {
                    diagnostics.push(runtime_error(
                        "RSPDL-INPUT-013",
                        format!("{path}.{key}"),
                        format!("필드 `{key}`가 모델에 선언되지 않았습니다."),
                    ));
                }
            }
            let mut field_values = BTreeMap::new();
            for field in &model.fields {
                let value = object
                    .get(field.local_id.as_str())
                    .filter(|value| !value.is_null());
                let Some(value) = value else {
                    if field.required {
                        diagnostics.push(runtime_error(
                            "RSPDL-INPUT-014",
                            format!("{path}.{}", field.local_id),
                            format!("필수 필드 `{}`가 누락되었습니다.", field.local_id),
                        ));
                    }
                    continue;
                };
                match bind_value(value, field, &path) {
                    Ok(value) => {
                        field_values.insert(field.id.clone(), value);
                    }
                    Err(diagnostic) => diagnostics.push(diagnostic),
                }
            }
            bound.push(BoundRecord {
                id,
                values: field_values,
            });
        }
        record_ids.insert(model.id.clone(), ids);
        records.insert(model.id.clone(), bound);
    }

    let role_ids = modules
        .iter()
        .flat_map(|module| module.roles.iter())
        .map(|role| (role.id.as_str(), role.id.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut roles = Vec::new();
    for (index, assignment) in input.role_assignments.into_iter().enumerate() {
        match role_ids.get(assignment.role.as_str()) {
            Some(role) => roles.push((assignment.actor, role.clone())),
            None => diagnostics.push(runtime_error(
                "RSPDL-INPUT-020",
                format!("$.role_assignments[{index}].role"),
                format!("역할 `{}`이 선언되지 않았습니다.", assignment.role),
            )),
        }
    }

    let action_ids = modules
        .iter()
        .flat_map(|module| module.actions.iter())
        .map(|action| (action.id.as_str(), action.id.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut action_request_ids = BTreeSet::new();
    let mut actions = Vec::new();
    for (index, action) in input.action_requests.into_iter().enumerate() {
        let path = format!("$.action_requests[{index}]");
        if !action_request_ids.insert(action.id.clone()) {
            diagnostics.push(runtime_error(
                "RSPDL-INPUT-021",
                format!("{path}.$id"),
                format!("행동 요청 ID `{}`가 중복되었습니다.", action.id),
            ));
        }
        let Some(model) = models.get(action.model.as_str()).copied() else {
            diagnostics.push(runtime_error(
                "RSPDL-INPUT-022",
                format!("{path}.model"),
                format!("데이터 모델 `{}`이 선언되지 않았습니다.", action.model),
            ));
            continue;
        };
        let Some(field) = model
            .fields
            .iter()
            .find(|field| field.local_id.as_str() == action.field)
        else {
            diagnostics.push(runtime_error(
                "RSPDL-INPUT-023",
                format!("{path}.field"),
                format!("필드 `{}`이 모델에 선언되지 않았습니다.", action.field),
            ));
            continue;
        };
        let Some(action_id) = action_ids.get(action.action.as_str()).cloned() else {
            diagnostics.push(runtime_error(
                "RSPDL-INPUT-024",
                format!("{path}.action"),
                format!("행동 `{}`이 선언되지 않았습니다.", action.action),
            ));
            continue;
        };
        if !record_ids
            .get(&model.id)
            .is_some_and(|ids| ids.contains(&action.record))
        {
            diagnostics.push(runtime_error(
                "RSPDL-INPUT-025",
                format!("{path}.record"),
                format!("레코드 `{}`을 찾을 수 없습니다.", action.record),
            ));
            continue;
        }
        actions.push(BoundAction {
            id: action.id,
            actor: action.actor,
            model_id: model.id.clone(),
            record_id: action.record,
            field_id: field.id.clone(),
            action_id,
        });
    }

    if diagnostics.is_empty() {
        Ok(BoundRuntime {
            records,
            roles,
            actions,
        })
    } else {
        Err(diagnostics)
    }
}

fn bind_value(
    value: &Value,
    field: &rspdl_domain::FieldDefinition,
    record_path: &str,
) -> Result<CanonicalValue, RuntimeDiagnostic> {
    let path = format!("{record_path}.{}", field.local_id);
    let invalid = || {
        runtime_error(
            "RSPDL-INPUT-015",
            &path,
            format!(
                "필드 `{}` 값이 타입 `{}`과 맞지 않습니다.",
                field.local_id, field.value_type
            ),
        )
    };
    match &field.value_type {
        CanonicalType::String => value
            .as_str()
            .map(CanonicalValue::string)
            .ok_or_else(invalid),
        CanonicalType::Boolean => value
            .as_bool()
            .map(CanonicalValue::boolean)
            .ok_or_else(invalid),
        CanonicalType::Integer => {
            let Some(number) = value.as_number() else {
                return Err(invalid());
            };
            CanonicalValue::integer_from_decimal(number.to_string()).map_err(|_| invalid())
        }
        CanonicalType::Enum(enum_type) => {
            let Some(local_id) = value.as_str() else {
                return Err(invalid());
            };
            let full_id = CanonicalId::new(format!("{}.{}", enum_type.id(), local_id))
                .map_err(|_| invalid())?;
            CanonicalValue::enum_variant(enum_type.clone(), full_id).map_err(|_| invalid())
        }
        CanonicalType::Refinement(_) => Err(invalid()),
    }
}

fn execute_constraints(
    modules: &[&SemanticModule],
    runtime: &BoundRuntime,
    options: CheckOptions,
    violations: &mut Vec<ConstraintViolation>,
    diagnostics: &mut Vec<RuntimeDiagnostic>,
) {
    let solve_options = match SolveOptions::with_timeout(options.solver_timeout) {
        Ok(options) => options,
        Err(error) => {
            diagnostics.push(runtime_error(
                "RSPDL-BACKEND-Z3-001",
                "$.records",
                error.to_string(),
            ));
            return;
        }
    };
    let solver = Z3Solver::new();
    for constraint in modules.iter().flat_map(|module| module.constraints.iter()) {
        let Some(records) = runtime.records.get(&constraint.model_id) else {
            continue;
        };
        for record in records {
            let Some(left) = operand_value(&constraint.left, record) else {
                continue;
            };
            let Some(right) = operand_value(&constraint.right, record) else {
                continue;
            };
            let expression = match relation_expression(constraint.operator, &left, &right) {
                Ok(expression) => expression,
                Err(message) => {
                    diagnostics.push(runtime_error(
                        "RSPDL-BACKEND-Z3-002",
                        format!("$.records.{}[{}]", constraint.model_id, record.id),
                        message,
                    ));
                    continue;
                }
            };
            let problem =
                match ConstraintProblem::new(Vec::new(), BooleanExpression::negate(expression)) {
                    Ok(problem) => problem,
                    Err(error) => {
                        diagnostics.push(runtime_error(
                            "RSPDL-BACKEND-Z3-002",
                            "$.records",
                            error.to_string(),
                        ));
                        continue;
                    }
                };
            match solver.solve(&problem, solve_options) {
                Ok(SolveResult::Sat(_)) => violations.push(ConstraintViolation {
                    constraint_id: constraint.id.clone(),
                    model_id: constraint.model_id.clone(),
                    record_id: record.id.clone(),
                    left,
                    right,
                }),
                Ok(SolveResult::Unsat) => {}
                Ok(SolveResult::Unknown { reason }) => diagnostics.push(runtime_error(
                    "RSPDL-BACKEND-Z3-003",
                    format!("$.records.{}[{}]", constraint.model_id, record.id),
                    format!("solver가 결과를 결정하지 못했습니다: {reason}"),
                )),
                Err(error) => diagnostics.push(runtime_error(
                    "RSPDL-BACKEND-Z3-004",
                    format!("$.records.{}[{}]", constraint.model_id, record.id),
                    error.to_string(),
                )),
            }
        }
    }
}

fn operand_value(operand: &ConstraintOperand, record: &BoundRecord) -> Option<CanonicalValue> {
    match operand {
        ConstraintOperand::Field(id) => record.values.get(id).cloned(),
        ConstraintOperand::Constant(value) => Some(value.clone()),
    }
}

fn relation_expression(
    operator: RelationOperator,
    left: &CanonicalValue,
    right: &CanonicalValue,
) -> Result<BooleanExpression, String> {
    let left = Term::Constant(left.clone());
    let right = Term::Constant(right.clone());
    match operator {
        RelationOperator::Equal => Atom::equal(left, right)
            .map(BooleanExpression::atom)
            .map_err(|error| error.to_string()),
        RelationOperator::NotEqual => Atom::equal(left, right)
            .map(|atom| BooleanExpression::negate(BooleanExpression::atom(atom)))
            .map_err(|error| error.to_string()),
        RelationOperator::LessThan => {
            integer_relation(rspdl_domain::ComparisonOperator::Lt, left, right)
        }
        RelationOperator::LessThanOrEqual => {
            integer_relation(rspdl_domain::ComparisonOperator::Le, left, right)
        }
        RelationOperator::GreaterThan => {
            integer_relation(rspdl_domain::ComparisonOperator::Gt, left, right)
        }
        RelationOperator::GreaterThanOrEqual => {
            integer_relation(rspdl_domain::ComparisonOperator::Ge, left, right)
        }
    }
}

fn integer_relation(
    operator: rspdl_domain::ComparisonOperator,
    left: Term,
    right: Term,
) -> Result<BooleanExpression, String> {
    Atom::integer_comparison(operator, left, right)
        .map(BooleanExpression::atom)
        .map_err(|error| error.to_string())
}

fn execute_policies(
    modules: &[&SemanticModule],
    runtime: &BoundRuntime,
) -> Result<Vec<PolicyResult>, String> {
    let string = CanonicalType::String;
    let role = PredicateSignature::new(
        canonical_id("runtime.role")?,
        vec![string.clone(), string.clone()],
    );
    let request = PredicateSignature::new(
        canonical_id("runtime.action_request")?,
        vec![
            string.clone(),
            string.clone(),
            string.clone(),
            string.clone(),
            string.clone(),
            string.clone(),
        ],
    );
    let allow = PredicateSignature::new(
        canonical_id("runtime.allow_match")?,
        vec![string.clone(), string.clone()],
    );
    let deny = PredicateSignature::new(
        canonical_id("runtime.deny_match")?,
        vec![string.clone(), string.clone()],
    );
    let application = |signature: &PredicateSignature, arguments: Vec<Term>| {
        PredicateApplication::new(signature.clone(), arguments).map_err(|error| error.to_string())
    };
    let constant = |value: &str| Term::Constant(CanonicalValue::string(value));

    let mut facts = Vec::new();
    for (actor, role_id) in &runtime.roles {
        facts.push(
            Fact::new(application(
                &role,
                vec![constant(actor), constant(role_id.as_str())],
            )?)
            .map_err(|error| error.to_string())?,
        );
    }
    for action in &runtime.actions {
        facts.push(
            Fact::new(application(
                &request,
                vec![
                    constant(&action.id),
                    constant(&action.actor),
                    constant(action.model_id.as_str()),
                    constant(&action.record_id),
                    constant(action.field_id.as_str()),
                    constant(action.action_id.as_str()),
                ],
            )?)
            .map_err(|error| error.to_string())?,
        );
    }

    let variable = |name: &str| -> Result<Term, String> {
        Ok(Term::Variable(Variable::new(
            canonical_id(name)?,
            string.clone(),
        )))
    };
    let request_id = variable("request_id")?;
    let actor = variable("actor")?;
    let model_id = variable("model_id")?;
    let record_id = variable("record_id")?;
    let field_id = variable("field_id")?;
    let action_id = variable("action_id")?;
    let mut rules = Vec::new();
    for (index, policy) in modules
        .iter()
        .flat_map(|module| module.policies.iter())
        .enumerate()
    {
        let head_signature = match policy.effect {
            PolicyEffect::Allow => &allow,
            PolicyEffect::Deny => &deny,
        };
        let head = application(
            head_signature,
            vec![request_id.clone(), constant(policy.id.as_str())],
        )?;
        let body = vec![
            RuleLiteral::Positive(application(
                &request,
                vec![
                    request_id.clone(),
                    actor.clone(),
                    model_id.clone(),
                    record_id.clone(),
                    field_id.clone(),
                    action_id.clone(),
                ],
            )?),
            RuleLiteral::Positive(application(
                &role,
                vec![actor.clone(), constant(policy.role_id.as_str())],
            )?),
            RuleLiteral::Constraint(
                Atom::equal(model_id.clone(), constant(policy.model_id.as_str()))
                    .map_err(|error| error.to_string())?,
            ),
            RuleLiteral::Constraint(
                Atom::equal(field_id.clone(), constant(policy.field_id.as_str()))
                    .map_err(|error| error.to_string())?,
            ),
            RuleLiteral::Constraint(
                Atom::equal(action_id.clone(), constant(policy.action_id.as_str()))
                    .map_err(|error| error.to_string())?,
            ),
        ];
        rules.push(DerivationRule::new(
            canonical_id(&format!("policy.rule.p{index}"))?,
            head,
            body,
        ));
    }
    let program = LogicProgram::new(
        vec![role, request, allow.clone(), deny.clone()],
        facts,
        rules,
    )
    .map_err(|error| error.to_string())?;
    let database = DatalogEvaluator::evaluate(&program).map_err(|error| error.to_string())?;
    let allow_matches = match_map(&database, &allow);
    let deny_matches = match_map(&database, &deny);
    let mut results = runtime
        .actions
        .iter()
        .map(|action| {
            let allows = allow_matches.get(&action.id).cloned().unwrap_or_default();
            let denies = deny_matches.get(&action.id).cloned().unwrap_or_default();
            let status = match (allows.is_empty(), denies.is_empty()) {
                (false, true) => PolicyStatus::Allowed,
                (true, false) => PolicyStatus::Denied,
                (false, false) => PolicyStatus::Conflict,
                (true, true) => PolicyStatus::Unmatched,
            };
            PolicyResult {
                request_id: action.id.clone(),
                status,
                allow_policies: allows,
                deny_policies: denies,
            }
        })
        .collect::<Vec<_>>();
    results.sort_by(|left, right| left.request_id.cmp(&right.request_id));
    Ok(results)
}

fn match_map(
    database: &rspdl_datalog::MaterializedDatabase,
    signature: &PredicateSignature,
) -> BTreeMap<String, Vec<CanonicalId>> {
    let mut matches = BTreeMap::<String, Vec<CanonicalId>>::new();
    for tuple in database.tuples(signature.id()).into_iter().flatten() {
        let [request, policy] = tuple.as_slice() else {
            continue;
        };
        let (Some(request), Some(policy)) = (request.as_string(), policy.as_string()) else {
            continue;
        };
        if let Ok(policy) = CanonicalId::new(policy) {
            matches.entry(request.to_owned()).or_default().push(policy);
        }
    }
    for policies in matches.values_mut() {
        policies.sort();
        policies.dedup();
    }
    matches
}

fn canonical_id(value: &str) -> Result<CanonicalId, String> {
    CanonicalId::new(value).map_err(|error| error.to_string())
}

fn runtime_error(
    rule_id: &str,
    path: impl Into<String>,
    message: impl Into<String>,
) -> RuntimeDiagnostic {
    RuntimeDiagnostic {
        rule_id: rule_id.into(),
        severity: Severity::Error,
        path: path.into(),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str = r#"@모듈 비용 승인(expense)

@열거형 비용 상태(status)는 다음 값 중 하나다.
    제출됨(submitted)
    승인됨(approved)

비용 신청(request)은 다음 필드들로 구성되어 있다.
    식별자(id): 필수 문자열
    신청자(applicant): 필수 문자열
    승인자(approver): 선택 문자열
    금액(amount): 필수 정수
    승인 상태(status): 필수 비용 상태

비용 신청의 금액은 0보다 커야 한다.

비용 신청의 신청자와 승인자는 달라야 한다.

@역할 회계 관리자(accounting_manager)
@역할 사용자(user)
@행동 변경(change)

회계 관리자는 비용 신청의 승인 상태를 변경할 수 있다.

사용자는 비용 신청의 승인 상태를 변경할 수 없다.
"#;

    #[test]
    fn compiles_and_executes_constraints_and_policies() {
        let data = r#"{
          "records": {
            "expense.request": [
              {
                "$id": "request-1",
                "id": "request-1",
                "applicant": "alice",
                "approver": "alice",
                "amount": -1,
                "status": "submitted"
              }
            ]
          },
          "role_assignments": [
            {"actor": "alice", "role": "expense.accounting_manager"},
            {"actor": "alice", "role": "expense.user"}
          ],
          "action_requests": [
            {
              "$id": "action-1",
              "actor": "alice",
              "model": "expense.request",
              "record": "request-1",
              "field": "status",
              "action": "expense.change"
            }
          ]
        }"#;
        let report = check_ko(SOURCE, data, CheckOptions::default());
        assert!(!report.has_errors(), "{:?}", report.runtime_diagnostics);
        assert_eq!(report.constraint_violations.len(), 2);
        assert_eq!(report.policy_results[0].status, PolicyStatus::Conflict);
        assert_eq!(report.policy_results[0].allow_policies.len(), 1);
        assert_eq!(report.policy_results[0].deny_policies.len(), 1);
    }

    #[test]
    fn input_errors_stop_backend_execution() {
        let report = check_ko(
            SOURCE,
            r#"{"records":{"expense.request":[{"$id":"x"}]}}"#,
            CheckOptions::default(),
        );
        assert!(report.has_errors());
        assert!(
            report
                .runtime_diagnostics
                .iter()
                .any(|diagnostic| diagnostic.rule_id == "RSPDL-INPUT-014")
        );
        assert!(report.policy_results.is_empty());
    }

    #[test]
    fn semantic_errors_stop_single_file_backend_execution() {
        let source = SOURCE.replace("0보다 커야 한다.", "\"zero\"이어야 한다.");
        let report = check_ko(&source, "not json", CheckOptions::default());

        assert!(report.has_errors());
        assert!(report.runtime_diagnostics.is_empty());
        assert!(report.constraint_violations.is_empty());
        assert!(report.policy_results.is_empty());
    }

    #[test]
    fn executes_every_v01_literal_comparison() {
        let source = r#"@모듈 비교(comparison)
@열거형 상태(state)는 다음 값 중 하나다.
    작성 중(draft)
    완료(done)
항목(item)은 다음 필드들로 구성되어 있다.
    값(value): 필수 정수
    이름(name): 필수 문자열
    활성(active): 필수 불리언
    상태(state): 필수 상태
    별칭(alias): 선택 문자열
항목의 값은 0보다 커야 한다.
항목의 값은 1 이상이어야 한다.
항목의 값은 10보다 작아야 한다.
항목의 값은 9 이하여야 한다.
항목의 이름은 "sample"이어야 한다.
항목의 활성은 참이어야 한다.
항목의 상태는 작성 중이어야 한다.
항목의 이름과 별칭은 같아야 한다.
"#;
        let data = r#"{
          "records": {
            "comparison.item": [
              {
                "$id": "one",
                "value": 5,
                "name": "sample",
                "active": true,
                "state": "draft"
              }
            ]
          }
        }"#;
        let report = check_ko(source, data, CheckOptions::default());
        assert!(!report.has_errors(), "{:?}", report);
        assert!(report.constraint_violations.is_empty());
    }

    #[test]
    fn multi_file_compilation_is_independent_of_input_order() {
        let alpha = KoSource::new(
            "alpha.rspdl",
            "@모듈 알파(alpha)\n항목(item)은 다음 필드들로 구성되어 있다.\n    값(value): 필수 정수\n",
        );
        let beta = KoSource::new(
            "beta.rspdl",
            "@모듈 베타(beta)\n항목(item)은 다음 필드들로 구성되어 있다.\n    값(value): 필수 정수\n",
        );

        let forward = compile_ko_files(vec![alpha.clone(), beta.clone()]);
        let reverse = compile_ko_files(vec![beta, alpha]);

        assert_eq!(forward, reverse);
        assert_eq!(forward.files.len(), 2);
        assert_eq!(forward.files[0].path, "alpha.rspdl");
        assert_eq!(
            forward.files[1].module.as_ref().unwrap().id.as_str(),
            "beta"
        );
    }

    #[test]
    fn duplicate_module_ids_are_workspace_errors() {
        let compilation = compile_ko_files(vec![
            KoSource::new("one.rspdl", "@모듈 하나(shared)\n"),
            KoSource::new("two.rspdl", "@모듈 둘(shared)\n"),
        ]);

        assert!(compilation.has_errors());
        assert!(compilation.files.iter().all(|file| {
            file.diagnostics
                .iter()
                .any(|diagnostic| diagnostic.rule_id == "RSPDL-LINK-001")
        }));
    }

    #[test]
    fn duplicate_source_paths_are_deterministic_workspace_errors() {
        let forward = compile_ko_files(vec![
            KoSource::new("same.rspdl", "@모듈 알파(alpha)\n"),
            KoSource::new("same.rspdl", "@모듈 베타(beta)\n"),
        ]);
        let reverse = compile_ko_files(vec![
            KoSource::new("same.rspdl", "@모듈 베타(beta)\n"),
            KoSource::new("same.rspdl", "@모듈 알파(alpha)\n"),
        ]);

        assert_eq!(forward, reverse);
        assert!(forward.files.iter().all(|file| {
            file.diagnostics
                .iter()
                .any(|diagnostic| diagnostic.rule_id == "RSPDL-SOURCE-001")
        }));
    }

    #[test]
    fn duplicate_qualified_symbols_across_modules_are_link_errors() {
        let compilation = compile_ko_files(vec![
            KoSource::new("one.rspdl", "@모듈 하나(one)\n@역할 관리자(shared.admin)\n"),
            KoSource::new("two.rspdl", "@모듈 둘(two)\n@역할 운영자(shared.admin)\n"),
        ]);

        assert!(compilation.has_errors());
        assert!(compilation.files.iter().all(|file| {
            file.diagnostics
                .iter()
                .any(|diagnostic| diagnostic.rule_id == "RSPDL-LINK-002")
        }));
    }

    #[test]
    fn checks_runtime_data_against_all_source_modules() {
        let sources = vec![
            KoSource::new(
                "alpha.rspdl",
                "@모듈 알파(alpha)\n항목(item)은 다음 필드들로 구성되어 있다.\n    값(value): 필수 정수\n항목의 값은 0보다 커야 한다.\n",
            ),
            KoSource::new(
                "beta.rspdl",
                "@모듈 베타(beta)\n항목(item)은 다음 필드들로 구성되어 있다.\n    값(value): 필수 정수\n항목의 값은 10보다 작아야 한다.\n",
            ),
        ];
        let data = r#"{
          "records": {
            "alpha.item": [{"$id": "alpha-1", "value": -1}],
            "beta.item": [{"$id": "beta-1", "value": 11}]
          }
        }"#;

        let report = check_ko_files(sources, data, CheckOptions::default());

        assert!(!report.has_errors(), "{:?}", report.runtime_diagnostics);
        assert_eq!(report.constraint_violations.len(), 2);
        assert_eq!(
            report
                .constraint_violations
                .iter()
                .map(|violation| violation.model_id.as_str())
                .collect::<Vec<_>>(),
            vec!["alpha.item", "beta.item"]
        );
    }
}
