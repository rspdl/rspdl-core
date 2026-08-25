//! RSPDL compiler and execution facade.

#![forbid(unsafe_code)]

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use rspdl_domain::{
    ActionDataMutationProvenance, BoundedModelOptions, BoundedModelResult, CanonicalId,
    CanonicalType, CanonicalValue, ConstraintOperand, Diagnostic, Frontend, FrontendOutput,
    PolicyEffect, RelationOperator, SemanticModule, Severity, SolveOptions, SourceId, TextRange,
    analyze_with_source, find_bounded_relational_model,
};
use rspdl_ko::KoreanFrontend;
use rspdl_solver_z3::Z3Solver;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Maximum per-model scope accepted by bounded model finding.
pub const MAX_MODEL_SCOPE_PER_MODEL: usize = rspdl_domain::MAX_BOUNDED_SCOPE_PER_MODEL;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Compilation {
    pub module: Option<SemanticModule>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub action_data_mutation_provenance: Vec<ActionDataMutationProvenance>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModelFindingOptions {
    pub scope_per_model: usize,
    pub solver_timeout: Duration,
}

impl Default for ModelFindingOptions {
    fn default() -> Self {
        Self {
            scope_per_model: 3,
            solver_timeout: Duration::from_secs(5),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ModelFindingFailure {
    pub rule_id: String,
    pub message_key: String,
    pub reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ModelFindingReport {
    pub compilation: Compilation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<BoundedModelResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure: Option<ModelFindingFailure>,
}

impl ModelFindingReport {
    pub fn has_errors(&self) -> bool {
        self.compilation
            .diagnostics
            .iter()
            .any(Diagnostic::is_error)
            || self.failure.is_some()
            || matches!(
                self.result,
                Some(BoundedModelResult::Unknown { .. } | BoundedModelResult::Unsupported { .. })
            )
    }

    pub fn has_findings(&self) -> bool {
        matches!(
            self.result,
            Some(BoundedModelResult::UnsatWithinBound { .. })
        )
    }
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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub action_data_mutation_provenance: Vec<ActionDataMutationProvenance>,
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

/// Compiles one Korean source while preserving its caller-supplied identity.
pub fn compile_ko_source(source: Source) -> Compilation {
    compile_source_with_frontend(&KoreanFrontend, source)
}

/// Compiles a Korean source and asks the Solver for a virtual finite data
/// world. The command consumes declarations only; no runtime records are used.
pub fn find_ko_model(source: &str, options: ModelFindingOptions) -> ModelFindingReport {
    let compilation = compile_ko(source);
    if compilation.diagnostics.iter().any(Diagnostic::is_error) {
        return ModelFindingReport {
            compilation,
            result: None,
            failure: None,
        };
    }
    let Some(module) = compilation.module.as_ref() else {
        return ModelFindingReport {
            compilation,
            result: None,
            failure: None,
        };
    };
    let solve_options = match SolveOptions::with_timeout(options.solver_timeout) {
        Ok(options) => options,
        Err(error) => {
            return model_finding_failure(
                compilation,
                "RSPDL-MODEL-001",
                "model_finding.configuration_error",
                error,
            );
        }
    };
    let bounded_options = match BoundedModelOptions::new(options.scope_per_model, solve_options) {
        Ok(options) => options,
        Err(error) => {
            return model_finding_failure(
                compilation,
                "RSPDL-MODEL-001",
                "model_finding.configuration_error",
                error,
            );
        }
    };
    match find_bounded_relational_model(module, &Z3Solver::new(), bounded_options) {
        Ok(result) => ModelFindingReport {
            compilation,
            result: Some(result),
            failure: None,
        },
        Err(error) => model_finding_failure(
            compilation,
            "RSPDL-MODEL-002",
            "model_finding.backend_error",
            error,
        ),
    }
}

fn model_finding_failure(
    compilation: Compilation,
    rule_id: &str,
    message_key: &str,
    error: impl ToString,
) -> ModelFindingReport {
    ModelFindingReport {
        compilation,
        result: None,
        failure: Some(ModelFindingFailure {
            rule_id: rule_id.into(),
            message_key: message_key.into(),
            reason: error.to_string(),
        }),
    }
}

/// Compiles any surface language that implements the shared frontend contract.
pub fn compile_with_frontend(frontend: &dyn Frontend, source: &str) -> Compilation {
    compile_frontend_output(frontend.lower_source(source), SourceId::inline()).0
}

/// Compiles one identified source through any conforming frontend.
pub fn compile_source_with_frontend(frontend: &dyn Frontend, source: Source) -> Compilation {
    let Source { path, text } = source;
    compile_frontend_output(frontend.lower_source(&text), SourceId::new(path)).0
}

fn compile_frontend_output(
    output: FrontendOutput,
    source_id: SourceId,
) -> (Compilation, TextRange) {
    let declaration_span = output
        .module
        .as_ref()
        .map_or(TextRange::default(), |module| module.declaration.span);
    let mut diagnostics = output.diagnostics;
    let mut action_data_mutation_provenance = Vec::new();
    let module = if diagnostics.iter().any(Diagnostic::is_error) {
        None
    } else if let Some(module) = output.module {
        let analyzed = analyze_with_source(module, source_id);
        diagnostics.extend(analyzed.diagnostics);
        action_data_mutation_provenance = analyzed.action_data_mutation_provenance;
        analyzed.module
    } else {
        None
    };
    diagnostics.sort_by(Diagnostic::stable_cmp);
    (
        Compilation {
            module,
            action_data_mutation_provenance,
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
            let source_id = SourceId::new(source.path.clone());
            let (compilation, declaration_span) =
                compile_frontend_output(frontend.lower_source(&source.text), source_id);
            FileCompilation {
                path: source.path,
                module: compilation.module,
                action_data_mutation_provenance: compilation.action_data_mutation_provenance,
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
            files[index].diagnostics.push(
                Diagnostic::error(
                    "RSPDL-SOURCE-001",
                    "compiler.source.duplicate_path",
                    Default::default(),
                )
                .with_argument("path", &path),
            );
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
            files[index].diagnostics.push(
                Diagnostic::error("RSPDL-LINK-001", "compiler.module.duplicate_id", span)
                    .with_argument("module_id", &module_id),
            );
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
            .chain(module.relations.iter().map(|value| &value.id))
            .chain(module.relational_constraints.iter().map(|value| &value.id))
            .chain(module.screens.iter().map(|value| &value.id))
            .chain(module.constraints.iter().map(|value| &value.id))
            .chain(module.roles.iter().map(|value| &value.id))
            .chain(module.actions.iter().map(|value| &value.id))
            .chain(module.events.iter().map(|value| &value.id))
            .chain(
                module
                    .conditional_productions
                    .iter()
                    .flat_map(|production| {
                        std::iter::once(&production.id)
                            .chain(production.branches.iter().map(|branch| &branch.id))
                            .chain(
                                production
                                    .field_producers
                                    .iter()
                                    .map(|producer| &producer.id),
                            )
                            .chain(
                                production
                                    .relation_producers
                                    .iter()
                                    .map(|producer| &producer.id),
                            )
                    }),
            )
            .chain(module.policies.iter().map(|value| &value.id))
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
            files[index].diagnostics.push(
                Diagnostic::error("RSPDL-LINK-002", "compiler.symbol.duplicate_id", span)
                    .with_argument("symbol_id", &symbol_id),
            );
        }
    }

    for file in &mut files {
        file.diagnostics.sort_by(Diagnostic::stable_cmp);
    }

    WorkspaceCompilation { files }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CheckOptions {
    /// **`check` 의 제약 실행에는 쓰이지 않는다.** 제약은 canonical value 를 정확히
    /// 비교해 판정하므로 solver 가 개입하지 않고, 따라서 시간 제한도 없다. 이 필드는
    /// SDK 의 `timeout_ms` 계약을 깨지 않으려고 남아 있다. solver 시간 제한이 실제로
    /// 필요한 곳은 model finding 이며 그쪽은 [`ModelFindingOptions`] 를 쓴다.
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
    pub message_key: String,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub arguments: BTreeMap<String, String>,
}

impl RuntimeDiagnostic {
    pub fn with_argument(mut self, key: impl Into<String>, value: impl ToString) -> Self {
        self.arguments.insert(key.into(), value.to_string());
        self
    }

    pub fn argument(&self, key: &str) -> Option<&str> {
        self.arguments.get(key).map(String::as_str)
    }

    fn stable_cmp(left: &Self, right: &Self) -> std::cmp::Ordering {
        (
            &left.path,
            &left.rule_id,
            &left.message_key,
            &left.arguments,
        )
            .cmp(&(
                &right.path,
                &right.rule_id,
                &right.message_key,
                &right.arguments,
            ))
    }
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

pub fn check_ko(source: &str, runtime_json: &str, _options: CheckOptions) -> CheckReport {
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
            report.runtime_diagnostics.push(
                runtime_error("RSPDL-INPUT-001", "$", "runtime.json.invalid")
                    .with_argument("reason", error),
            );
            return report;
        }
    };
    let runtime = match bind_runtime(&[module], input) {
        Ok(runtime) => runtime,
        Err(mut diagnostics) => {
            diagnostics.sort_by(RuntimeDiagnostic::stable_cmp);
            report.runtime_diagnostics = diagnostics;
            return report;
        }
    };

    execute_constraints(
        &[module],
        &runtime,
        &mut report.constraint_violations,
        &mut report.runtime_diagnostics,
    );
    report.policy_results = execute_policies(&[module], &runtime);
    report.constraint_violations.sort_by(|left, right| {
        (&left.model_id, &left.record_id, &left.constraint_id).cmp(&(
            &right.model_id,
            &right.record_id,
            &right.constraint_id,
        ))
    });
    report
        .runtime_diagnostics
        .sort_by(RuntimeDiagnostic::stable_cmp);
    report
}

pub fn check_ko_files(
    sources: Vec<KoSource>,
    runtime_json: &str,
    _options: CheckOptions,
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
            report.runtime_diagnostics.push(
                runtime_error("RSPDL-INPUT-001", "$", "runtime.json.invalid")
                    .with_argument("reason", error),
            );
            return report;
        }
    };
    let runtime = match bind_runtime(&modules, input) {
        Ok(runtime) => runtime,
        Err(mut diagnostics) => {
            diagnostics.sort_by(RuntimeDiagnostic::stable_cmp);
            report.runtime_diagnostics = diagnostics;
            return report;
        }
    };

    execute_constraints(
        &modules,
        &runtime,
        &mut report.constraint_violations,
        &mut report.runtime_diagnostics,
    );
    report.policy_results = execute_policies(&modules, &runtime);
    report.constraint_violations.sort_by(|left, right| {
        (&left.model_id, &left.record_id, &left.constraint_id).cmp(&(
            &right.model_id,
            &right.record_id,
            &right.constraint_id,
        ))
    });
    report
        .runtime_diagnostics
        .sort_by(RuntimeDiagnostic::stable_cmp);
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
            diagnostics.push(
                runtime_error(
                    "RSPDL-INPUT-010",
                    format!("$.records.{model_text}"),
                    "runtime.model.not_found",
                )
                .with_argument("model_id", model_text),
            );
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
                        "runtime.record.id_required",
                    ));
                    continue;
                }
            };
            if !ids.insert(id.clone()) {
                diagnostics.push(
                    runtime_error(
                        "RSPDL-INPUT-012",
                        format!("{path}.$id"),
                        "runtime.record.duplicate_id",
                    )
                    .with_argument("record_id", &id),
                );
            }
            for key in object.keys() {
                if key != "$id" && !known_fields.contains_key(key.as_str()) {
                    diagnostics.push(
                        runtime_error(
                            "RSPDL-INPUT-013",
                            format!("{path}.{key}"),
                            "runtime.field.not_found",
                        )
                        .with_argument("field_id", key),
                    );
                }
            }
            let mut field_values = BTreeMap::new();
            for field in &model.fields {
                let value = object
                    .get(field.local_id.as_str())
                    .filter(|value| !value.is_null());
                let Some(value) = value else {
                    if field.required {
                        diagnostics.push(
                            runtime_error(
                                "RSPDL-INPUT-014",
                                format!("{path}.{}", field.local_id),
                                "runtime.field.required_missing",
                            )
                            .with_argument("field_id", &field.local_id),
                        );
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
            None => diagnostics.push(
                runtime_error(
                    "RSPDL-INPUT-020",
                    format!("$.role_assignments[{index}].role"),
                    "runtime.role.not_found",
                )
                .with_argument("role_id", assignment.role),
            ),
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
            diagnostics.push(
                runtime_error(
                    "RSPDL-INPUT-021",
                    format!("{path}.$id"),
                    "runtime.action_request.duplicate_id",
                )
                .with_argument("request_id", &action.id),
            );
        }
        let Some(model) = models.get(action.model.as_str()).copied() else {
            diagnostics.push(
                runtime_error(
                    "RSPDL-INPUT-022",
                    format!("{path}.model"),
                    "runtime.model.not_found",
                )
                .with_argument("model_id", &action.model),
            );
            continue;
        };
        let Some(field) = model
            .fields
            .iter()
            .find(|field| field.local_id.as_str() == action.field)
        else {
            diagnostics.push(
                runtime_error(
                    "RSPDL-INPUT-023",
                    format!("{path}.field"),
                    "runtime.field.not_found",
                )
                .with_argument("field_id", &action.field),
            );
            continue;
        };
        let Some(action_id) = action_ids.get(action.action.as_str()).cloned() else {
            diagnostics.push(
                runtime_error(
                    "RSPDL-INPUT-024",
                    format!("{path}.action"),
                    "runtime.action.not_found",
                )
                .with_argument("action_id", &action.action),
            );
            continue;
        };
        if !record_ids
            .get(&model.id)
            .is_some_and(|ids| ids.contains(&action.record))
        {
            diagnostics.push(
                runtime_error(
                    "RSPDL-INPUT-025",
                    format!("{path}.record"),
                    "runtime.record.not_found",
                )
                .with_argument("record_id", &action.record),
            );
            continue;
        }
        actions.push(BoundAction {
            id: action.id,
            actor: action.actor,
            model_id: model.id.clone(),
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
        runtime_error("RSPDL-INPUT-015", &path, "runtime.value.type_mismatch")
            .with_argument("field_id", &field.local_id)
            .with_argument("expected_type", &field.value_type)
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
        CanonicalType::Decimal => bind_decimal_text(value)
            .and_then(|value| CanonicalValue::decimal_from_str(value).ok())
            .ok_or_else(invalid),
        CanonicalType::Date => value
            .as_str()
            .and_then(|value| CanonicalValue::date_from_iso(value).ok())
            .ok_or_else(invalid),
        CanonicalType::Time => value
            .as_str()
            .and_then(|value| CanonicalValue::time_from_iso(value).ok())
            .ok_or_else(invalid),
        CanonicalType::DateTime => value
            .as_str()
            .and_then(|value| CanonicalValue::date_time_from_rfc3339(value).ok())
            .ok_or_else(invalid),
        CanonicalType::Duration => value
            .as_str()
            .and_then(|value| CanonicalValue::duration_from_iso(value).ok())
            .ok_or_else(invalid),
        CanonicalType::Latitude => bind_decimal_text(value)
            .and_then(|value| CanonicalValue::latitude_from_decimal(value).ok())
            .ok_or_else(invalid),
        CanonicalType::Longitude => bind_decimal_text(value)
            .and_then(|value| CanonicalValue::longitude_from_decimal(value).ok())
            .ok_or_else(invalid),
        CanonicalType::Money(_) => value
            .as_str()
            .and_then(|value| CanonicalValue::money_from_str(value).ok())
            .filter(|bound| bound.value_type() == &field.value_type)
            .ok_or_else(invalid),
        CanonicalType::Percentage => value
            .as_str()
            .and_then(|value| CanonicalValue::percentage_from_str(value).ok())
            .ok_or_else(invalid),
        CanonicalType::Quantity(_) => value
            .as_str()
            .and_then(|value| CanonicalValue::quantity_from_str(value).ok())
            .filter(|bound| bound.value_type() == &field.value_type)
            .ok_or_else(invalid),
        CanonicalType::Coordinate => value
            .as_str()
            .and_then(|value| CanonicalValue::coordinate_from_str(value).ok())
            .ok_or_else(invalid),
        CanonicalType::Uuid
        | CanonicalType::Email
        | CanonicalType::Url
        | CanonicalType::PhoneNumber
        | CanonicalType::IpAddress
        | CanonicalType::Cidr
        | CanonicalType::CountryCode
        | CanonicalType::LanguageCode
        | CanonicalType::CurrencyCode => value
            .as_str()
            .and_then(|value| {
                CanonicalValue::refinement_from_str(field.value_type.clone(), value).ok()
            })
            .ok_or_else(invalid),
        CanonicalType::List(element) => bind_collection(value, element, false).ok_or_else(invalid),
        CanonicalType::Set(element) => bind_collection(value, element, true).ok_or_else(invalid),
        CanonicalType::Map {
            key,
            value: value_type,
        } => bind_map(value, key, value_type).ok_or_else(invalid),
        CanonicalType::Reference(target) => value
            .as_str()
            .and_then(|value| CanonicalValue::reference(target.clone(), value).ok())
            .ok_or_else(invalid),
        CanonicalType::LocalDateTime => value
            .as_str()
            .and_then(|value| CanonicalValue::local_date_time_from_iso(value).ok())
            .ok_or_else(invalid),
        CanonicalType::ZonedDateTime => value
            .as_str()
            .and_then(|value| CanonicalValue::zoned_date_time_from_str(value).ok())
            .ok_or_else(invalid),
        CanonicalType::CalendarDuration => value
            .as_str()
            .and_then(|value| CanonicalValue::calendar_duration_from_iso(value).ok())
            .ok_or_else(invalid),
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

fn bind_collection(value: &Value, element: &CanonicalType, set: bool) -> Option<CanonicalValue> {
    let values = value.as_array()?;
    let mut canonical = values
        .iter()
        .map(|value| bind_runtime_type(value, element))
        .collect::<Option<Vec<_>>>()?;
    if set {
        canonical.sort();
        canonical.dedup();
        if canonical.len() != values.len() {
            return None;
        }
    }
    if set {
        CanonicalValue::set(element.clone(), canonical).ok()
    } else {
        CanonicalValue::list(element.clone(), canonical).ok()
    }
}

fn bind_map(
    input: &Value,
    key: &CanonicalType,
    value_type: &CanonicalType,
) -> Option<CanonicalValue> {
    // 어떤 key 타입이 결정적인지는 `CanonicalType::map` 이 정한다. 여기서 그 목록을
    // 베껴 두면 새 key 타입이 허용될 때 두 곳이 갈라진다.
    let object = input.as_object()?;
    let mut canonical = Vec::new();
    for (raw_key, raw_value) in object {
        canonical.push((
            bind_runtime_type(&Value::String(raw_key.clone()), key)?,
            bind_runtime_type(raw_value, value_type)?,
        ));
    }
    CanonicalValue::map(key.clone(), value_type.clone(), canonical).ok()
}

fn bind_runtime_type(value: &Value, value_type: &CanonicalType) -> Option<CanonicalValue> {
    let field = rspdl_domain::FieldDefinition {
        id: CanonicalId::new("internal.value").expect("constant ID"),
        local_id: CanonicalId::new("value").expect("constant ID"),
        name: String::new(),
        required: true,
        value_type: value_type.clone(),
        // 이 필드는 runtime 값을 검사하려고 만든 합성 선언이라 원문 문장이 없다.
        span: TextRange::default(),
    };
    bind_value(value, &field, "$").ok()
}

fn bind_decimal_text(value: &Value) -> Option<String> {
    if let Some(text) = value.as_str() {
        return Some(text.to_owned());
    }
    let number = value.as_number()?;
    // `serde_json` 은 `arbitrary_precision` 없이 쓰므로 소수 JSON 숫자는 이미 `f64` 로
    // 반올림된 뒤다. 그 자릿수를 정확한 소수 생성자에 넘기면 "정확하다" 는 약속이
    // 거짓이 된다. 정수는 그대로 보존되므로 그대로 받고, 소수는 문자열로 받는다.
    if number.is_f64() {
        return None;
    }
    Some(number.to_string())
}

fn execute_constraints(
    modules: &[&SemanticModule],
    runtime: &BoundRuntime,
    violations: &mut Vec<ConstraintViolation>,
    diagnostics: &mut Vec<RuntimeDiagnostic>,
) {
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
            let holds = match relation_holds(constraint.operator, &left, &right) {
                Ok(holds) => holds,
                Err(message) => {
                    diagnostics.push(
                        runtime_error(
                            "RSPDL-RUNTIME-001",
                            format!("$.records.{}[{}]", constraint.model_id, record.id),
                            "runtime.constraint.expression_error",
                        )
                        .with_argument("reason", message),
                    );
                    continue;
                }
            };
            if !holds {
                violations.push(ConstraintViolation {
                    constraint_id: constraint.id.clone(),
                    model_id: constraint.model_id.clone(),
                    record_id: record.id.clone(),
                    left,
                    right,
                });
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

fn relation_holds(
    operator: RelationOperator,
    left: &CanonicalValue,
    right: &CanonicalValue,
) -> Result<bool, String> {
    match operator {
        RelationOperator::Equal => Ok(left == right),
        RelationOperator::NotEqual => Ok(left != right),
        RelationOperator::LessThan => ordered_relation(Ordering::is_lt, left, right),
        RelationOperator::LessThanOrEqual => {
            ordered_relation(|ordering| ordering.is_le(), left, right)
        }
        RelationOperator::GreaterThan => ordered_relation(Ordering::is_gt, left, right),
        RelationOperator::GreaterThanOrEqual => {
            ordered_relation(|ordering| ordering.is_ge(), left, right)
        }
    }
}

fn ordered_relation(
    predicate: impl FnOnce(Ordering) -> bool,
    left: &CanonicalValue,
    right: &CanonicalValue,
) -> Result<bool, String> {
    left.compare_ordered(right)
        .map(predicate)
        .map_err(|error| error.to_string())
}

fn execute_policies(modules: &[&SemanticModule], runtime: &BoundRuntime) -> Vec<PolicyResult> {
    let policies = modules
        .iter()
        .flat_map(|module| module.policies.iter())
        .collect::<Vec<_>>();
    let mut results = runtime
        .actions
        .iter()
        .map(|action| {
            let mut allows = Vec::new();
            let mut denies = Vec::new();
            for policy in &policies {
                if action.model_id != policy.model_id
                    || action.field_id != policy.field_id
                    || action.action_id != policy.action_id
                    || !runtime
                        .roles
                        .contains(&(action.actor.clone(), policy.role_id.clone()))
                {
                    continue;
                }
                match policy.effect {
                    PolicyEffect::Allow => allows.push(policy.id.clone()),
                    PolicyEffect::Deny => denies.push(policy.id.clone()),
                }
            }
            allows.sort();
            allows.dedup();
            denies.sort();
            denies.dedup();
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
    results
}

fn runtime_error(
    rule_id: &str,
    path: impl Into<String>,
    message_key: impl Into<String>,
) -> RuntimeDiagnostic {
    RuntimeDiagnostic {
        rule_id: rule_id.into(),
        severity: Severity::Error,
        path: path.into(),
        message_key: message_key.into(),
        arguments: BTreeMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str = r#"@모듈 비용 승인(expense)

비용 상태(status)는 다음 값 중 하나다.
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

회계 관리자(accounting_manager)는 역할이다.
사용자(user)는 역할이다.
변경(change)은 행동이다.

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
    fn direct_policy_matching_preserves_every_runtime_status_and_order() {
        let data = r#"{
          "records": {
            "expense.request": [
              {
                "$id": "request-1",
                "id": "request-1",
                "applicant": "alice",
                "approver": "bob",
                "amount": 1,
                "status": "submitted"
              }
            ]
          },
          "role_assignments": [
            {"actor": "alice", "role": "expense.accounting_manager"},
            {"actor": "bob", "role": "expense.user"},
            {"actor": "carol", "role": "expense.accounting_manager"},
            {"actor": "carol", "role": "expense.user"}
          ],
          "action_requests": [
            {
              "$id": "request-unmatched",
              "actor": "dana",
              "model": "expense.request",
              "record": "request-1",
              "field": "status",
              "action": "expense.change"
            },
            {
              "$id": "request-denied",
              "actor": "bob",
              "model": "expense.request",
              "record": "request-1",
              "field": "status",
              "action": "expense.change"
            },
            {
              "$id": "request-conflict",
              "actor": "carol",
              "model": "expense.request",
              "record": "request-1",
              "field": "status",
              "action": "expense.change"
            },
            {
              "$id": "request-allowed",
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
        assert!(report.constraint_violations.is_empty());
        assert_eq!(
            report
                .policy_results
                .iter()
                .map(|result| (result.request_id.as_str(), result.status))
                .collect::<Vec<_>>(),
            vec![
                ("request-allowed", PolicyStatus::Allowed),
                ("request-conflict", PolicyStatus::Conflict),
                ("request-denied", PolicyStatus::Denied),
                ("request-unmatched", PolicyStatus::Unmatched),
            ]
        );
        assert_eq!(report.policy_results[0].allow_policies.len(), 1);
        assert!(report.policy_results[0].deny_policies.is_empty());
        assert_eq!(report.policy_results[1].allow_policies.len(), 1);
        assert_eq!(report.policy_results[1].deny_policies.len(), 1);
        assert!(report.policy_results[2].allow_policies.is_empty());
        assert_eq!(report.policy_results[2].deny_policies.len(), 1);
        assert!(report.policy_results[3].allow_policies.is_empty());
        assert!(report.policy_results[3].deny_policies.is_empty());
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
                .any(|diagnostic| diagnostic.rule_id == "RSPDL-INPUT-014"
                    && diagnostic.message_key == "runtime.field.required_missing"
                    && diagnostic.argument("field_id") == Some("id"))
        );
        let json = serde_json::to_string(&report.runtime_diagnostics).unwrap();
        assert!(!json.contains("필수 필드"));
        assert!(!json.contains("\"message\":"));
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
상태(state)는 다음 값 중 하나다.
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

    const EXTENDED_SCALAR_SOURCE: &str = r#"@모듈 확장 값(extended_runtime)
이벤트(event)는 다음 필드들로 구성되어 있다.
    금액(amount): 필수 소수
    시작일(start_date): 필수 날짜
    시작 시각(start_time): 필수 시간
    발생 시점(occurred_at): 필수 날짜시간
    대기 기간(wait_duration): 필수 기간
    위도(latitude): 필수 위도
    경도(longitude): 필수 경도
이벤트의 금액은 "10.5" 이상이어야 한다.
이벤트의 시작일은 "2026-08-13" 이상이어야 한다.
이벤트의 시작 시각은 "09:00:00"보다 커야 한다.
이벤트의 발생 시점은 "2026-08-13T05:30:00Z" 이상이어야 한다.
이벤트의 대기 기간은 "PT2S" 이하여야 한다.
이벤트의 위도는 "37.5"이어야 한다.
이벤트의 경도는 "127"이어야 한다.
"#;

    #[test]
    fn binds_and_executes_extended_scalar_constraints_exactly() {
        let data = r#"{
          "records": {
            "extended_runtime.event": [
              {
                "$id": "valid",
                "amount": "10.5",
                "start_date": "2026-08-13",
                "start_time": "09:00:00.000000001",
                "occurred_at": "2026-08-13T14:30:00+09:00",
                "wait_duration": "PT1.5S",
                "latitude": "37.5",
                "longitude": "127.0"
              },
              {
                "$id": "invalid",
                "amount": "10.499",
                "start_date": "2026-08-12",
                "start_time": "09:00:00",
                "occurred_at": "2026-08-13T05:29:59Z",
                "wait_duration": "PT2.000000001S",
                "latitude": "37.6",
                "longitude": "126.9"
              }
            ]
          }
        }"#;
        let report = check_ko(EXTENDED_SCALAR_SOURCE, data, CheckOptions::default());
        assert!(!report.has_errors(), "{report:?}");
        assert_eq!(report.constraint_violations.len(), 7);
        assert!(
            report
                .constraint_violations
                .iter()
                .all(|violation| violation.record_id == "invalid")
        );
    }

    #[test]
    fn rejects_out_of_range_runtime_coordinates_before_constraints() {
        let data = r#"{
          "records": {
            "extended_runtime.event": [{
              "$id": "invalid-coordinate",
              "amount": "10.5",
              "start_date": "2026-08-13",
              "start_time": "09:00:01",
              "occurred_at": "2026-08-13T05:30:00Z",
              "wait_duration": "PT1S",
              "latitude": "90.0001",
              "longitude": 127
            }]
          }
        }"#;
        let report = check_ko(EXTENDED_SCALAR_SOURCE, data, CheckOptions::default());
        assert!(report.has_errors());
        assert_eq!(report.runtime_diagnostics.len(), 1);
        assert_eq!(report.runtime_diagnostics[0].rule_id, "RSPDL-INPUT-015");
        assert_eq!(
            report.runtime_diagnostics[0].argument("expected_type"),
            Some("latitude")
        );
        assert!(report.constraint_violations.is_empty());
    }

    #[test]
    fn fractional_json_numbers_are_rejected_for_exact_decimals() {
        // `serde_json` 은 소수 JSON 숫자를 `f64` 로 담는다. 그 자릿수를 정확한 소수로
        // 받아들이면 "정확하다" 는 약속이 입력 모양에 따라 조용히 깨진다.
        let data = r#"{
          "records": {
            "extended_runtime.event": [
              {
                "$id": "rounded",
                "amount": 10.5,
                "start_date": "2026-08-13",
                "start_time": "09:00:01",
                "occurred_at": "2026-08-13T05:30:00Z",
                "wait_duration": "PT1S",
                "latitude": "37.5",
                "longitude": 127
              }
            ]
          }
        }"#;
        let report = check_ko(EXTENDED_SCALAR_SOURCE, data, CheckOptions::default());
        assert!(report.has_errors(), "{report:?}");
        assert_eq!(
            report.runtime_diagnostics[0].argument("expected_type"),
            Some("decimal")
        );
    }

    #[test]
    fn extended_scalar_symbolic_model_finding_is_explicitly_unsupported() {
        let report = find_ko_model(EXTENDED_SCALAR_SOURCE, ModelFindingOptions::default());
        assert!(matches!(
            report.result,
            Some(BoundedModelResult::Unsupported { ref constructs, .. })
                if constructs.iter().any(|construct| construct == "symbolic_field:decimal:extended_runtime.event.amount")
        ));
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
            KoSource::new(
                "one.rspdl",
                "@모듈 하나(one)\n관리자(shared.admin)는 역할이다.\n",
            ),
            KoSource::new(
                "two.rspdl",
                "@모듈 둘(two)\n운영자(shared.admin)는 역할이다.\n",
            ),
        ]);

        assert!(compilation.has_errors());
        assert!(compilation.files.iter().all(|file| {
            file.diagnostics
                .iter()
                .any(|diagnostic| diagnostic.rule_id == "RSPDL-LINK-002")
        }));
    }

    #[test]
    fn duplicate_qualified_event_ids_across_modules_are_link_errors() {
        let compilation = compile_ko_files(vec![
            KoSource::new(
                "one.rspdl",
                "@모듈 하나(one)\n요청 접수됨(shared.request_received)은 사건이다.\n",
            ),
            KoSource::new(
                "two.rspdl",
                "@모듈 둘(two)\n접수됨(shared.request_received)은 사건이다.\n",
            ),
        ]);

        assert!(compilation.has_errors());
        assert!(compilation.files.iter().all(|file| {
            file.diagnostics.iter().any(|diagnostic| {
                diagnostic.rule_id == "RSPDL-LINK-002"
                    && diagnostic.argument("symbol_id") == Some("shared.request_received")
            })
        }));
    }

    #[test]
    fn duplicate_qualified_screen_ids_across_modules_are_link_errors() {
        let compilation = compile_ko_files(vec![
            KoSource::new(
                "one.rspdl",
                "@모듈 하나(one)\n항목(item)은 다음 필드들로 구성되어 있다.\n    값(value): 필수 정수\n공통 화면(shared.screen)에서는 항목을 생성할 수 있다.\n",
            ),
            KoSource::new(
                "two.rspdl",
                "@모듈 둘(two)\n항목(item)은 다음 필드들로 구성되어 있다.\n    값(value): 필수 정수\n공통 화면(shared.screen)에서는 항목을 생성할 수 있다.\n",
            ),
        ]);

        assert!(compilation.has_errors());
        assert!(compilation.files.iter().all(|file| {
            file.diagnostics.iter().any(|diagnostic| {
                diagnostic.rule_id == "RSPDL-LINK-002"
                    && diagnostic.argument("symbol_id") == Some("shared.screen")
            })
        }));
    }

    #[test]
    fn duplicate_qualified_creation_branch_ids_across_modules_are_link_errors() {
        let source = |module_name: &str, module_id: &str| {
            format!(
                "@모듈 {module_name}({module_id})\n상태(status)는 다음 값 중 하나다.\n    접수됨(received)\n알림(notice)은 다음 필드들로 구성되어 있다.\n    내용(body): 선택 문자열\n전달(assign)은 행동이다.\n전달은 상태를 요청 상태(request_status)로 입력받는다.\n접수 생성(shared.received_create)은 전달의 요청 상태가 접수됨이면 알림을 하나 생성한다.\n"
            )
        };
        let compilation = compile_ko_files(vec![
            KoSource::new("one.rspdl", source("하나", "one")),
            KoSource::new("two.rspdl", source("둘", "two")),
        ]);

        assert!(compilation.has_errors());
        assert!(compilation.files.iter().all(|file| {
            file.diagnostics.iter().any(|diagnostic| {
                diagnostic.rule_id == "RSPDL-LINK-002"
                    && diagnostic.argument("symbol_id") == Some("shared.received_create")
            })
        }));
    }

    #[test]
    fn duplicate_qualified_field_producer_ids_across_modules_are_link_errors() {
        let source = |module_name: &str, module_id: &str| {
            format!(
                "@모듈 {module_name}({module_id})\n상태(status)는 다음 값 중 하나다.\n    접수됨(received)\n알림(notice)은 다음 필드들로 구성되어 있다.\n    내용(body): 필수 문자열\n전달(assign)은 행동이다.\n전달은 상태를 요청 상태(request_status)로 입력받는다.\n전달은 문자열을 알림 내용(body_input)으로 입력받는다.\n접수 생성(received_create)은 전달의 요청 상태가 접수됨이면 알림을 하나 생성한다.\n내용 기록(shared.body_binding)은 전달이 실행될 때 알림 내용을 알림의 내용으로 기록한다.\n"
            )
        };
        let compilation = compile_ko_files(vec![
            KoSource::new("one.rspdl", source("하나", "one")),
            KoSource::new("two.rspdl", source("둘", "two")),
        ]);

        assert!(compilation.has_errors());
        assert!(compilation.files.iter().all(|file| {
            file.diagnostics.iter().any(|diagnostic| {
                diagnostic.rule_id == "RSPDL-LINK-002"
                    && diagnostic.argument("symbol_id") == Some("shared.body_binding")
            })
        }));
    }

    #[test]
    fn duplicate_qualified_relation_producer_ids_across_modules_are_link_errors() {
        let source = |module_name: &str, module_id: &str| {
            format!(
                "@모듈 {module_name}({module_id})\n상태(status)는 다음 값 중 하나다.\n    접수됨(received)\n기술자(technician)는 다음 필드들로 구성되어 있다.\n    이름(name): 필수 문자열\n알림(notice)은 다음 필드들로 구성되어 있다.\n    내용(body): 선택 문자열\n알림은 기술자를 수신자(recipient)로 가질 수 있다.\n모든 알림은 수신자를 하나 이상 가져야 한다.\n각 알림은 수신자를 최대 하나만 가질 수 있다.\n전달(assign)은 행동이다.\n전달은 상태를 요청 상태(request_status)로 입력받는다.\n전달은 기존 기술자를 수신 기술자(recipient_technician)로 입력받는다.\n접수 생성(received_create)은 전달의 요청 상태가 접수됨이면 알림을 하나 생성한다.\n수신자 연결(shared.recipient_binding)은 전달이 실행될 때 수신 기술자를 알림의 수신자로 연결한다.\n"
            )
        };
        let compilation = compile_ko_files(vec![
            KoSource::new("one.rspdl", source("하나", "one")),
            KoSource::new("two.rspdl", source("둘", "two")),
        ]);

        assert!(compilation.has_errors());
        assert!(compilation.files.iter().all(|file| {
            file.diagnostics.iter().any(|diagnostic| {
                diagnostic.rule_id == "RSPDL-LINK-002"
                    && diagnostic.argument("symbol_id") == Some("shared.recipient_binding")
            })
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

    const RELATIONAL_SOURCE: &str = r#"@모듈 관계 테스트(relational)
프로젝트(project)는 다음 필드들로 구성되어 있다.
    이름(name): 필수 문자열
사용자(user)는 다음 필드들로 구성되어 있다.
    이름(name): 필수 문자열
프로젝트는 사용자를 소유자(owner)로 가질 수 있다.
프로젝트는 사용자를 검토자(reviewer)로 가질 수 있다.
프로젝트는 하나 이상 존재해야 한다.
모든 프로젝트는 소유자를 하나 이상 가져야 한다.
각 프로젝트는 소유자를 최대 하나만 가질 수 있다.
모든 프로젝트는 검토자를 하나 이상 가져야 한다.
"#;

    #[test]
    fn finds_virtual_entities_for_required_unique_relation() {
        let report = find_ko_model(
            RELATIONAL_SOURCE,
            ModelFindingOptions {
                scope_per_model: 1,
                ..ModelFindingOptions::default()
            },
        );

        assert!(!report.has_errors(), "{:?}", report.failure);
        let Some(BoundedModelResult::Sat { witness, .. }) = report.result else {
            panic!("expected SAT model: {:?}", report.result);
        };
        assert!(
            witness
                .entities
                .iter()
                .any(|entity| { entity.model_id.as_str() == "relational.project" })
        );
        assert_eq!(
            witness
                .relation_tuples
                .iter()
                .filter(|tuple| tuple.relation_id.as_str() == "relational.owner")
                .count(),
            1
        );
    }

    #[test]
    fn reports_unsat_only_within_the_requested_bound() {
        let source =
            format!("{RELATIONAL_SOURCE}소유자, 검토자 중 둘 이상은 동시에 성립할 수 없다.\n");
        let scope_one = find_ko_model(
            &source,
            ModelFindingOptions {
                scope_per_model: 1,
                ..ModelFindingOptions::default()
            },
        );
        let scope_two = find_ko_model(
            &source,
            ModelFindingOptions {
                scope_per_model: 2,
                ..ModelFindingOptions::default()
            },
        );

        let Some(BoundedModelResult::UnsatWithinBound {
            scope_per_model,
            core_rule_ids,
        }) = scope_one.result
        else {
            panic!("expected bound-1 UNSAT: {:?}", scope_one.result);
        };
        assert_eq!(scope_per_model, 1);
        assert!(core_rule_ids.len() >= 4);
        assert!(matches!(
            scope_two.result,
            Some(BoundedModelResult::Sat { .. })
        ));
    }

    #[test]
    fn compatible_coexistence_does_not_create_a_false_conflict() {
        let source = format!("{RELATIONAL_SOURCE}소유자, 검토자는 동시에 성립할 수 있다.\n");
        let report = find_ko_model(
            &source,
            ModelFindingOptions {
                scope_per_model: 1,
                ..ModelFindingOptions::default()
            },
        );

        assert!(matches!(
            report.result,
            Some(BoundedModelResult::Sat { .. })
        ));
    }

    #[test]
    fn exclusive_exhaustive_unary_relations_classify_each_existing_entity_once() {
        let source = r#"@모듈 사용자 분류(classification)
사용자(user)는 다음 필드들로 구성되어 있다.
    이름(name): 필수 문자열
사용자는 내부 사용자(internal)에 해당할 수 있다.
사용자는 외부 사용자(external)에 해당할 수 있다.
사용자는 하나 이상 존재해야 한다.
내부 사용자, 외부 사용자 중 둘 이상은 동시에 성립할 수 없다.
내부 사용자, 외부 사용자 중 하나 이상은 항상 성립해야 한다.
"#;
        let report = find_ko_model(
            source,
            ModelFindingOptions {
                scope_per_model: 1,
                ..ModelFindingOptions::default()
            },
        );
        let Some(BoundedModelResult::Sat { witness, .. }) = report.result else {
            panic!("expected SAT classification: {:?}", report.result);
        };

        assert_eq!(witness.entities.len(), 1);
        assert_eq!(witness.relation_tuples.len(), 1);
    }

    #[test]
    fn contradictory_compatibility_metadata_is_a_structured_error() {
        let source = format!(
            "{RELATIONAL_SOURCE}소유자, 검토자 중 둘 이상은 동시에 성립할 수 없다.\n검토자, 소유자는 동시에 성립할 수 있다.\n"
        );
        let compilation = compile_ko(&source);

        assert!(compilation.module.is_none());
        assert!(compilation.diagnostics.iter().any(|diagnostic| {
            diagnostic.rule_id == "RSPDL-REL-004"
                && diagnostic.message_key == "semantic.relation.compatibility_conflict"
                && diagnostic.argument("relation_ids")
                    == Some("relational.owner,relational.reviewer")
        }));
    }

    #[test]
    fn compatibility_conflict_is_detected_inside_larger_relation_groups() {
        let source = r#"@모듈 분류(classification)
사용자(user)는 다음 필드들로 구성되어 있다.
    이름(name): 필수 문자열
사용자는 내부(internal)에 해당할 수 있다.
사용자는 외부(external)에 해당할 수 있다.
사용자는 파트너(partner)에 해당할 수 있다.
내부, 외부, 파트너 중 둘 이상은 동시에 성립할 수 없다.
외부, 파트너는 동시에 성립할 수 있다.
"#;
        let compilation = compile_ko(source);

        assert!(compilation.diagnostics.iter().any(|diagnostic| {
            diagnostic.rule_id == "RSPDL-REL-004"
                && diagnostic.argument("relation_ids")
                    == Some("classification.external,classification.partner")
        }));
    }

    #[test]
    fn relation_group_input_order_does_not_change_canonical_semantics() {
        let prefix = r#"@모듈 분류(classification)
사용자(user)는 다음 필드들로 구성되어 있다.
    이름(name): 필수 문자열
사용자는 내부(internal)에 해당할 수 있다.
사용자는 외부(external)에 해당할 수 있다.
"#;
        let forward = compile_ko(&format!(
            "{prefix}내부, 외부 중 둘 이상은 동시에 성립할 수 없다.\n"
        ));
        let reverse = compile_ko(&format!(
            "{prefix}외부, 내부 중 둘 이상은 동시에 성립할 수 없다.\n"
        ));

        assert_eq!(forward.module, reverse.module);
        assert!(forward.diagnostics.is_empty());
        assert!(reverse.diagnostics.is_empty());
    }

    #[test]
    fn required_attribute_constraints_participate_in_the_virtual_theory() {
        let source = r#"@모듈 속성(attribute)
항목(item)은 다음 필드들로 구성되어 있다.
    값(value): 필수 정수
항목의 값은 0보다 커야 한다.
항목의 값은 0보다 작아야 한다.
항목은 하나 이상 존재해야 한다.
"#;
        let report = find_ko_model(
            source,
            ModelFindingOptions {
                scope_per_model: 1,
                ..ModelFindingOptions::default()
            },
        );

        let Some(BoundedModelResult::UnsatWithinBound { core_rule_ids, .. }) = report.result else {
            panic!("expected attribute contradiction: {:?}", report.result);
        };
        assert_eq!(core_rule_ids.len(), 3);
    }

    #[test]
    fn absent_optional_attribute_prevents_a_false_contradiction() {
        let source = r#"@모듈 선택 속성(optional_attribute)
항목(item)은 다음 필드들로 구성되어 있다.
    값(value): 선택 정수
항목의 값은 0보다 커야 한다.
항목의 값은 0보다 작아야 한다.
항목은 하나 이상 존재해야 한다.
"#;
        let report = find_ko_model(
            source,
            ModelFindingOptions {
                scope_per_model: 1,
                ..ModelFindingOptions::default()
            },
        );

        let Some(BoundedModelResult::Sat { witness, .. }) = report.result else {
            panic!("optional field may be absent: {:?}", report.result);
        };
        assert!(witness.field_values.is_empty());
    }

    #[test]
    fn unsupported_derivation_is_not_approximated_as_sat() {
        let report = find_ko_model(
            include_str!("../../../examples/field-provenance.rspdl"),
            ModelFindingOptions::default(),
        );

        assert!(matches!(
            report.result,
            Some(BoundedModelResult::Unsupported { ref constructs, .. })
                if constructs == &["derivation"]
        ));
        assert!(report.has_errors());
    }
}
