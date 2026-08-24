'use strict'

const assert = require('node:assert/strict')
const test = require('node:test')

const sdk = require('../sdk.cjs')

const validSource = `@모듈 재고(inventory)

재고 항목(item)은 다음 필드들로 구성되어 있다.
    이름(name): 필수 문자열
`

test('CommonJS compile returns a versioned workspace result', async () => {
  const response = await sdk.compile([
    { path: 'inventory.rspdl', text: validSource },
  ])

  assert.equal(response.schema_version, 1)
  assert.equal(response.result.files[0].module.id, 'inventory')
  assert.deepEqual(response.result.files[0].module.span, {
    start: 0,
    end: validSource.indexOf('\n'),
  })
  assert.ok(response.result.files[0].module.models[0].span.end > 0)
})

test('compiler errors remain in the result', async () => {
  const response = await sdk.compile([
    { path: 'invalid.rspdl', text: 'invalid' },
  ])

  assert.equal(response.result.files[0].module, null)
  assert.ok(response.result.files[0].diagnostics.length > 0)
})

test('invalid SDK configuration rejects with a stable code', async () => {
  await assert.rejects(
    sdk.compile([{ path: 'inventory.rspdl', text: validSource }], {
      locale: 'en-US',
    }),
    /RSPDL-SDK-003/,
  )
})

test('explicit null options reject with stable SDK option errors', async () => {
  const source = { path: 'inventory.rspdl', text: validSource }

  await assert.rejects(sdk.compile([source], null), /RSPDL-SDK-004.*options/)
  await assert.rejects(sdk.compile([source], []), /RSPDL-SDK-004.*options/)
  await assert.rejects(sdk.compile([source], 'invalid'), /RSPDL-SDK-004.*options/)
  await assert.rejects(
    sdk.compile([source], { locale: null }),
    /RSPDL-SDK-004.*locale/,
  )
  await assert.rejects(
    sdk.check([source], { records: {} }, { timeoutMs: null }),
    /RSPDL-SDK-004.*timeoutMs/,
  )
  await assert.rejects(
    sdk.findModel(source, { scopePerModel: null }),
    /RSPDL-SDK-004.*scopePerModel/,
  )
})

test('ESM, check and model entry points work', async () => {
  const esm = await import('../sdk.mjs')
  const source = { path: 'inventory.rspdl', text: validSource }

  const compiled = await esm.compile([source])
  const checked = await esm.check([source], { records: {} })
  const modeled = await esm.findModel(source, { scopePerModel: 1 })

  assert.equal(compiled.schema_version, 1)
  assert.equal(checked.schema_version, 1)
  assert.equal(modeled.schema_version, 1)
})
