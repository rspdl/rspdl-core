'use strict'

const native = require('./index.js')

const WIRE_SCHEMA_VERSION = 1
const SUPPORTED_LOCALE = 'ko-KR'

const decode = async (response) => JSON.parse(await response)

const compile = (sources, options = {}) =>
  decode(
    native.compileJson(
      JSON.stringify({
        schema_version: WIRE_SCHEMA_VERSION,
        locale: options.locale ?? SUPPORTED_LOCALE,
        sources,
      }),
    ),
  )

const check = (sources, data, options = {}) =>
  decode(
    native.checkJson(
      JSON.stringify({
        schema_version: WIRE_SCHEMA_VERSION,
        locale: options.locale ?? SUPPORTED_LOCALE,
        sources,
        data,
        timeout_ms: options.timeoutMs ?? 5_000,
      }),
    ),
  )

const findModel = (source, options = {}) =>
  decode(
    native.findModelJson(
      JSON.stringify({
        schema_version: WIRE_SCHEMA_VERSION,
        locale: options.locale ?? SUPPORTED_LOCALE,
        source,
        scope_per_model: options.scopePerModel ?? 3,
        timeout_ms: options.timeoutMs ?? 5_000,
      }),
    ),
  )

module.exports = {
  SUPPORTED_LOCALE,
  WIRE_SCHEMA_VERSION,
  check,
  compile,
  findModel,
}
