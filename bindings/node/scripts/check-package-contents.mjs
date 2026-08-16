import { execFileSync } from 'node:child_process'

const npm = process.platform === 'win32' ? 'npm.cmd' : 'npm'
const reports = JSON.parse(
  execFileSync(npm, ['pack', '--dry-run', '--ignore-scripts', '--json'], {
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'inherit'],
  }),
)
if (!Array.isArray(reports) || reports.length !== 1) {
  throw new Error('npm pack must report exactly one root package')
}

const paths = new Set(reports[0].files.map(({ path }) => path))
const required = [
  'LICENSE',
  'README.md',
  'THIRD_PARTY_LICENSES.html',
  'index.js',
  'package.json',
  'sdk.cjs',
  'sdk.d.ts',
  'sdk.mjs',
]

for (const path of required) {
  if (!paths.has(path)) throw new Error(`npm package is missing ${path}`)
}

if (![...paths].some((path) => /^rspdl\..+\.node$/.test(path))) {
  throw new Error('npm package is missing the native rspdl addon')
}
