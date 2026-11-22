import { realpath, rm } from 'node:fs/promises'
import { join } from 'node:path'
import { parseArgs } from 'node:util'

const { values } = parseArgs({ options: { 'dry-run': { type: 'boolean' } } })
const dryRun = values['dry-run']
const root = await realpath(new URL('../', import.meta.url))
const generatedPaths = [
  'target',
  'node_modules',
  'dist',
  'apps/web/node_modules',
  'apps/web/dist',
  'apps/web/tsconfig.tsbuildinfo'
]

const targets = []
for (const name of generatedPaths) {
  const path = join(root, name)
  let resolvedPath
  try {
    resolvedPath = await realpath(path)
  } catch (error) {
    if (error.code === 'ENOENT') continue
    throw error
  }
  if (resolvedPath !== path) {
    throw new Error(`Refusing to clean a redirected path: ${name}`)
  }
  targets.push({ name, path })
}

if (targets.length === 0) {
  console.log('No generated files to clean.')
}

for (const { name, path } of targets) {
  if (!dryRun) await rm(path, { recursive: true, force: true })
  console.log(`${dryRun ? 'Would remove' : 'Removed'}: ${name}`)
}
