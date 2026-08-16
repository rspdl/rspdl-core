'use strict'

const supportedBindings = {
  'darwin-arm64': {
    local: './rspdl.darwin-arm64.node',
    package: 'rspdl-darwin-arm64',
  },
  'darwin-x64': {
    local: './rspdl.darwin-x64.node',
    package: 'rspdl-darwin-x64',
  },
  'linux-x64': {
    local: './rspdl.linux-x64-gnu.node',
    package: 'rspdl-linux-x64-gnu',
  },
  'win32-x64': {
    local: './rspdl.win32-x64-msvc.node',
    package: 'rspdl-win32-x64-msvc',
  },
}

const target = `${process.platform}-${process.arch}`
const binding = supportedBindings[target]

if (!binding) {
  throw new Error(
    `RSPDL-SDK-005: unsupported Node.js platform ${process.platform}/${process.arch}`,
  )
}

const candidates = process.env.NAPI_RS_NATIVE_LIBRARY_PATH
  ? [process.env.NAPI_RS_NATIVE_LIBRARY_PATH]
  : [binding.local, binding.package]
const loadErrors = []

for (const candidate of candidates) {
  try {
    module.exports = require(candidate)
    return
  } catch (error) {
    loadErrors.push(error)
  }
}

throw new Error(
  `RSPDL-SDK-005: native addon unavailable for ${process.platform}/${process.arch}; reinstall rspdl-core on a supported glibc Linux, macOS 14+, or Windows system`,
  { cause: new AggregateError(loadErrors) },
)
