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
void findModel(source, { scopePerModel: 1, timeoutMs: 1_000 })
void rspdl.compile([source])
