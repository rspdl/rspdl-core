export type JsonPrimitive = string | number | boolean | null
export type JsonValue = JsonPrimitive | JsonObject | JsonValue[]
export interface JsonObject {
  [key: string]: JsonValue
}

export interface Source {
  path: string
  text: string
}

export interface TextRange {
  start: number
  end: number
}

export interface Diagnostic {
  rule_id: string
  severity: 'error' | 'warning' | 'info'
  message_key: string
  arguments?: Record<string, string>
  span: TextRange
}

export interface FileCompilation {
  path: string
  module: JsonObject | null
  action_data_mutation_provenance?: JsonObject[]
  diagnostics: Diagnostic[]
}

export interface WorkspaceCompilation {
  files: FileCompilation[]
}

export interface RuntimeDiagnostic {
  rule_id: string
  severity: 'error' | 'warning' | 'info'
  path: string
  message_key: string
  arguments?: Record<string, string>
}

export interface ConstraintViolation extends JsonObject {
  constraint_id: string
  model_id: string
  record_id: string
}

export interface PolicyResult {
  request_id: string
  status: 'allowed' | 'denied' | 'conflict' | 'unmatched'
  allow_policies: string[]
  deny_policies: string[]
}

export interface WorkspaceCheckReport {
  compilation: WorkspaceCompilation
  constraint_violations: ConstraintViolation[]
  policy_results: PolicyResult[]
  runtime_diagnostics: RuntimeDiagnostic[]
}

export interface ModelFindingReport {
  compilation: JsonObject
  result?: JsonObject
  failure?: JsonObject
}

export interface SdkResponse<T> {
  schema_version: 1
  result: T
}

export interface CommonOptions {
  locale?: string
}

export interface CheckOptions extends CommonOptions {
  timeoutMs?: number
}

export interface FindModelOptions extends CheckOptions {
  scopePerModel?: number
}

export declare const WIRE_SCHEMA_VERSION: 1
export declare const SUPPORTED_LOCALE: 'ko-KR'

export declare function compile(
  sources: readonly Source[],
  options?: CommonOptions,
): Promise<SdkResponse<WorkspaceCompilation>>

export declare function check(
  sources: readonly Source[],
  data: JsonObject,
  options?: CheckOptions,
): Promise<SdkResponse<WorkspaceCheckReport>>

export declare function findModel(
  source: Source,
  options?: FindModelOptions,
): Promise<SdkResponse<ModelFindingReport>>

declare const sdk: {
  readonly WIRE_SCHEMA_VERSION: typeof WIRE_SCHEMA_VERSION
  readonly SUPPORTED_LOCALE: typeof SUPPORTED_LOCALE
  readonly compile: typeof compile
  readonly check: typeof check
  readonly findModel: typeof findModel
}

export default sdk
