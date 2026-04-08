import { readFile } from 'node:fs/promises'
import { spawnSync } from 'node:child_process'

import { generatedSchemaPath, openApiSpecPath, repoRoot } from './governance-lib.mjs'

const previousGenerated = await readFile(generatedSchemaPath, 'utf8').catch(() => '')
const generation = spawnSync(process.execPath, ['scripts/generate-schema.mjs'], {
  cwd: repoRoot,
  encoding: 'utf8',
})

if (generation.status !== 0) {
  process.stdout.write(generation.stdout)
  process.stderr.write(generation.stderr)
  process.exit(generation.status ?? 1)
}

const regenerated = await readFile(generatedSchemaPath, 'utf8')
const spec = await readFile(openApiSpecPath, 'utf8')

if (!spec.includes('openapi: 3.1.0')) {
  console.error('Schema governance check failed:\n- contracts/openapi/octopus.openapi.yaml must declare OpenAPI 3.1.0')
  process.exit(1)
}

if (previousGenerated !== regenerated) {
  console.error('Schema governance check failed:\n- packages/schema/src/generated.ts is not fresh; rerun pnpm schema:generate')
  process.exit(1)
}

console.log('Schema governance check passed.')
