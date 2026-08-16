'use strict'

const native = require('./native.cjs')

const WIRE_SCHEMA_VERSION = 1
const SUPPORTED_LOCALE = 'ko-KR'

const decode = async (response) => JSON.parse(await response)

const invalidOption = (option, reason) =>
  new TypeError(`RSPDL-SDK-004: invalid SDK option \`${option}\`: ${reason}`)

const normalizeOptions = (options) => {
  if (options === undefined) return {}
  if (options === null || typeof options !== 'object' || Array.isArray(options)) {
    throw invalidOption('options', 'must be an object')
  }
  return options
}

const optionOrDefault = (options, name, fallback) => {
  const value = options[name]
  if (value === undefined) return fallback
  if (value === null) throw invalidOption(name, 'must not be null')
  return value
}

const compile = async (sources, rawOptions) => {
  const options = normalizeOptions(rawOptions)
  return decode(
    native.compileJson(
      JSON.stringify({
        schema_version: WIRE_SCHEMA_VERSION,
        locale: optionOrDefault(options, 'locale', SUPPORTED_LOCALE),
        sources,
      }),
    ),
  )
}

const check = async (sources, data, rawOptions) => {
  const options = normalizeOptions(rawOptions)
  return decode(
    native.checkJson(
      JSON.stringify({
        schema_version: WIRE_SCHEMA_VERSION,
        locale: optionOrDefault(options, 'locale', SUPPORTED_LOCALE),
        sources,
        data,
        timeout_ms: optionOrDefault(options, 'timeoutMs', 5_000),
      }),
    ),
  )
}

const findModel = async (source, rawOptions) => {
  const options = normalizeOptions(rawOptions)
  return decode(
    native.findModelJson(
      JSON.stringify({
        schema_version: WIRE_SCHEMA_VERSION,
        locale: optionOrDefault(options, 'locale', SUPPORTED_LOCALE),
        source,
        scope_per_model: optionOrDefault(options, 'scopePerModel', 3),
        timeout_ms: optionOrDefault(options, 'timeoutMs', 5_000),
      }),
    ),
  )
}

module.exports = {
  SUPPORTED_LOCALE,
  WIRE_SCHEMA_VERSION,
  check,
  compile,
  findModel,
}
