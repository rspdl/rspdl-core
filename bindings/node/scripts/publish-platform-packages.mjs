import { spawnSync } from 'node:child_process'
import {
  readFileSync,
  readdirSync,
  writeFileSync,
} from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'

const scriptDirectory = dirname(fileURLToPath(import.meta.url))
const defaultPackageRoot = resolve(scriptDirectory, '..')
const npmCommand = process.platform === 'win32' ? 'npm.cmd' : 'npm'

export const platformPackageAliases = [
  {
    directory: 'win32-x64-msvc',
    dependencyName: 'rspdl-win32-x64-msvc',
    publishedName: 'rspdl-native-windows-x64',
  },
]

function readJson(path) {
  return JSON.parse(readFileSync(path, 'utf8'))
}

function writeJson(path, value) {
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`)
}

function assertNonEmptyString(value, label) {
  if (typeof value !== 'string' || value.length === 0) {
    throw new Error(`${label} must be a non-empty string`)
  }
}

export function applyPlatformPackageAliases(packageRoot = defaultPackageRoot) {
  const rootManifestPath = join(packageRoot, 'package.json')
  const rootManifest = readJson(rootManifestPath)
  assertNonEmptyString(rootManifest.name, 'Root package name')
  assertNonEmptyString(rootManifest.version, 'Root package version')

  if (
    rootManifest.optionalDependencies === undefined
    || rootManifest.optionalDependencies === null
  ) {
    rootManifest.optionalDependencies = {}
  }
  if (
    typeof rootManifest.optionalDependencies !== 'object'
    || Array.isArray(rootManifest.optionalDependencies)
  ) {
    throw new Error('Root optionalDependencies must be an object')
  }

  for (const alias of platformPackageAliases) {
    const platformManifestPath = join(
      packageRoot,
      'npm',
      alias.directory,
      'package.json',
    )
    const platformManifest = readJson(platformManifestPath)
    const allowedNames = new Set([
      alias.dependencyName,
      alias.publishedName,
    ])

    if (!allowedNames.has(platformManifest.name)) {
      throw new Error(
        `Unexpected package name in ${platformManifestPath}: ${platformManifest.name}`,
      )
    }
    if (platformManifest.version !== rootManifest.version) {
      throw new Error(
        `Version mismatch for ${alias.dependencyName}: expected ${rootManifest.version}, got ${platformManifest.version}`,
      )
    }

    platformManifest.name = alias.publishedName
    rootManifest.optionalDependencies[alias.dependencyName] =
      `npm:${alias.publishedName}@${rootManifest.version}`
    writeJson(platformManifestPath, platformManifest)
  }

  writeJson(rootManifestPath, rootManifest)
  return rootManifest
}

function runNpm(args, options = {}) {
  return spawnSync(npmCommand, args, {
    encoding: 'utf8',
    ...options,
  })
}

function commandOutput(result) {
  return [result.stdout, result.stderr].filter(Boolean).join('\n')
}

function packageVersionExists(packageName, version, packageRoot, npmRunner) {
  const result = npmRunner(
    ['view', `${packageName}@${version}`, 'version', '--json'],
    { cwd: packageRoot },
  )

  if (result.error) {
    throw result.error
  }
  if (result.status === 0) {
    let publishedVersion
    try {
      publishedVersion = JSON.parse(result.stdout.trim())
    } catch (error) {
      throw new Error(
        `npm returned invalid JSON for ${packageName}@${version}: ${result.stdout}`,
        { cause: error },
      )
    }
    if (publishedVersion !== version) {
      throw new Error(
        `npm returned unexpected version for ${packageName}@${version}: ${publishedVersion}`,
      )
    }
    return true
  }

  const output = commandOutput(result)
  if (/E404|404 Not Found/i.test(output)) {
    return false
  }
  throw new Error(
    `Unable to check ${packageName}@${version} on npm (exit ${result.status}):\n${output}`,
  )
}

export function publishPlatformPackages(
  packageRoot = defaultPackageRoot,
  npmRunner = runNpm,
) {
  applyPlatformPackageAliases(packageRoot)

  const npmDirectory = join(packageRoot, 'npm')
  const platformDirectories = readdirSync(npmDirectory, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => entry.name)
    .sort()

  for (const directory of platformDirectories) {
    const platformDirectory = join(npmDirectory, directory)
    const manifest = readJson(join(platformDirectory, 'package.json'))
    assertNonEmptyString(manifest.name, `${directory} package name`)
    assertNonEmptyString(manifest.version, `${directory} package version`)
    const packageSpec = `${manifest.name}@${manifest.version}`

    if (
      packageVersionExists(
        manifest.name,
        manifest.version,
        packageRoot,
        npmRunner,
      )
    ) {
      console.log(`Skipping existing ${packageSpec}`)
      continue
    }

    console.log(`Publishing ${packageSpec}`)
    const result = npmRunner(
      ['publish', '--access', 'public'],
      { cwd: platformDirectory, stdio: 'inherit' },
    )
    if (result.error) {
      throw result.error
    }
    if (result.status !== 0) {
      throw new Error(`npm publish failed for ${packageSpec} (exit ${result.status})`)
    }
  }
}

if (
  process.argv[1]
  && import.meta.url === pathToFileURL(resolve(process.argv[1])).href
) {
  publishPlatformPackages()
}
