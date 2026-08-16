import assert from 'node:assert/strict'
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import test from 'node:test'

import {
  applyPlatformPackageAliases,
  publishPlatformPackages,
} from '../scripts/publish-platform-packages.mjs'

function writeJson(path, value) {
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`)
}

function readJson(path) {
  return JSON.parse(readFileSync(path, 'utf8'))
}

function createPackageFixture() {
  const packageRoot = mkdtempSync(join(tmpdir(), 'rspdl-npm-publish-'))
  const platforms = [
    {
      directory: 'darwin-arm64',
      name: 'rspdl-core-darwin-arm64',
      os: ['darwin'],
      cpu: ['arm64'],
      main: 'rspdl.darwin-arm64.node',
    },
    {
      directory: 'darwin-x64',
      name: 'rspdl-core-darwin-x64',
      os: ['darwin'],
      cpu: ['x64'],
      main: 'rspdl.darwin-x64.node',
    },
    {
      directory: 'linux-x64-gnu',
      name: 'rspdl-core-linux-x64-gnu',
      os: ['linux'],
      cpu: ['x64'],
      main: 'rspdl.linux-x64-gnu.node',
    },
    {
      directory: 'win32-x64-msvc',
      name: 'rspdl-core-win32-x64-msvc',
      os: ['win32'],
      cpu: ['x64'],
      main: 'rspdl.win32-x64-msvc.node',
    },
  ]
  const optionalDependencies = Object.fromEntries(
    platforms.map((platform) => [platform.name, '0.1.0']),
  )

  writeJson(join(packageRoot, 'package.json'), {
    name: 'rspdl-core',
    version: '0.1.0',
    optionalDependencies,
  })
  for (const platform of platforms) {
    const platformDirectory = join(packageRoot, 'npm', platform.directory)
    mkdirSync(platformDirectory, { recursive: true })
    writeJson(join(platformDirectory, 'package.json'), {
      name: platform.name,
      version: '0.1.0',
      main: platform.main,
      files: [platform.main],
      os: platform.os,
      cpu: platform.cpu,
    })
  }

  return packageRoot
}

test('maps generated packages to stable public names and aliases Windows', (context) => {
  const packageRoot = createPackageFixture()
  context.after(() => rmSync(packageRoot, { recursive: true, force: true }))

  applyPlatformPackageAliases(packageRoot)

  const rootManifest = readJson(join(packageRoot, 'package.json'))
  const windowsManifest = readJson(
    join(packageRoot, 'npm', 'win32-x64-msvc', 'package.json'),
  )
  assert.equal(
    rootManifest.optionalDependencies['rspdl-win32-x64-msvc'],
    'npm:rspdl-native-windows-x64@0.1.0',
  )
  assert.equal(
    rootManifest.optionalDependencies['rspdl-darwin-arm64'],
    '0.1.0',
  )
  assert.equal(
    rootManifest.optionalDependencies['rspdl-core-darwin-arm64'],
    undefined,
  )
  assert.equal(
    readJson(join(packageRoot, 'npm', 'darwin-arm64', 'package.json')).name,
    'rspdl-darwin-arm64',
  )
  assert.equal(windowsManifest.name, 'rspdl-native-windows-x64')
  assert.equal(windowsManifest.main, 'rspdl.win32-x64-msvc.node')
  assert.deepEqual(windowsManifest.os, ['win32'])
  assert.deepEqual(windowsManifest.cpu, ['x64'])

  applyPlatformPackageAliases(packageRoot)
  assert.equal(
    readJson(join(packageRoot, 'npm', 'win32-x64-msvc', 'package.json')).name,
    'rspdl-native-windows-x64',
  )
})

test('skips published versions and publishes only missing platform packages', (context) => {
  const packageRoot = createPackageFixture()
  context.after(() => rmSync(packageRoot, { recursive: true, force: true }))
  const commands = []

  publishPlatformPackages(packageRoot, (args, options) => {
    commands.push({ args, cwd: options.cwd })
    if (args[0] === 'view') {
      if (args[1] !== 'rspdl-native-windows-x64@0.1.0') {
        return { status: 0, stdout: '"0.1.0"\n', stderr: '' }
      }
      return { status: 1, stdout: '', stderr: 'npm error code E404' }
    }
    return { status: 0, stdout: '', stderr: '' }
  })

  const publishCommands = commands.filter(({ args }) => args[0] === 'publish')
  assert.equal(publishCommands.length, 1)
  assert.equal(
    publishCommands[0].cwd,
    join(packageRoot, 'npm', 'win32-x64-msvc'),
  )
})

test('fails closed when npm registry availability cannot be determined', (context) => {
  const packageRoot = createPackageFixture()
  context.after(() => rmSync(packageRoot, { recursive: true, force: true }))

  assert.throws(
    () => publishPlatformPackages(packageRoot, () => ({
      status: 1,
      stdout: '',
      stderr: 'npm error code E503',
    })),
    /Unable to check rspdl-darwin-arm64@0\.1\.0 on npm/,
  )
})
