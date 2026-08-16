import { copyFileSync, existsSync, readFileSync, readdirSync, writeFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const repositoryRoot = resolve(packageRoot, '..', '..')

const licenseFiles = ['LICENSE', 'README.md', 'THIRD_PARTY_LICENSES.html']

for (const name of licenseFiles) {
  copyFileSync(resolve(repositoryRoot, name), resolve(packageRoot, name))
}

const platformPackages = resolve(packageRoot, 'npm')
if (existsSync(platformPackages)) {
  for (const directory of readdirSync(platformPackages, { withFileTypes: true })) {
    if (!directory.isDirectory()) continue

    const platformRoot = resolve(platformPackages, directory.name)
    for (const name of licenseFiles) {
      copyFileSync(resolve(repositoryRoot, name), resolve(platformRoot, name))
    }

    const manifestPath = resolve(platformRoot, 'package.json')
    const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'))
    manifest.files = [...new Set([...manifest.files, ...licenseFiles])]
    writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`)
  }
}
