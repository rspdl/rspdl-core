import { copyFileSync, existsSync, readFileSync, readdirSync, writeFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const repositoryRoot = resolve(packageRoot, '..', '..')

const licenseFiles = ['LICENSE', 'README.md', 'THIRD_PARTY_LICENSES.html']

for (const name of licenseFiles) {
  const source = resolve(repositoryRoot, name)
  if (!existsSync(source)) {
    const recovery =
      name === 'THIRD_PARTY_LICENSES.html'
        ? 'From the repository root, run cargo about generate --workspace --all-features --locked --fail -o THIRD_PARTY_LICENSES.html about.hbs before packaging.'
        : `Restore ${name} from the source checkout before packaging.`
    throw new Error(`Missing ${name}. ${recovery}`)
  }
  copyFileSync(source, resolve(packageRoot, name))
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
    manifest.files = [...new Set([...(manifest.files ?? []), ...licenseFiles])]
    writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`)
  }
}
