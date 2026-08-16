import rspdl, {
  check,
  compile,
  findModel,
  type Source,
  type WorkspaceCompilation,
} from 'rspdl'

const source: Source = { path: 'inventory.rspdl', text: '@모듈 재고(inventory)' }
const compilation: Promise<{ schema_version: 1; result: WorkspaceCompilation }> = compile([source])

void compilation
void check([source], { records: {} })
void findModel(source, { scopePerModel: 1, timeoutMs: 1_000 }).then(({ result }) => {
  if (result.result?.status === 'unsupported') {
    result.result.constructs.forEach((construct) => construct.toUpperCase())
  }
  if (result.result?.status === 'unsat_within_bound') {
    result.result.core_rule_ids.forEach((ruleId) => ruleId.toUpperCase())
  }
})
void rspdl.compile([source])
