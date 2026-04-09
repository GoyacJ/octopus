import { readFile } from 'node:fs/promises'

import { generatedSchemaPath, openApiSourceEntryPath, openApiSpecPath } from './governance-lib.mjs'
import { bundleOpenApiDocument, bundleOpenApiYaml } from './openapi-bundle-lib.mjs'
import { renderGeneratedSchema } from './schema-generator-lib.mjs'

const spec = await readFile(openApiSpecPath, 'utf8')
const generated = await readFile(generatedSchemaPath, 'utf8').catch(() => '')
const bundledDocument = await bundleOpenApiDocument({ rootPath: openApiSourceEntryPath })
const bundledSpec = await bundleOpenApiYaml({ rootPath: openApiSourceEntryPath })
const regenerated = renderGeneratedSchema(bundledDocument, bundledSpec)
const failures = []

if (bundledDocument.openapi !== '3.1.0') {
  failures.push('contracts/openapi/octopus.openapi.yaml must declare OpenAPI 3.1.0')
}

if (spec !== bundledSpec) {
  failures.push('contracts/openapi/octopus.openapi.yaml is not fresh; rerun pnpm openapi:bundle')
}

if (generated !== regenerated) {
  failures.push('packages/schema/src/generated.ts is not fresh; rerun pnpm schema:generate')
}

if (failures.length) {
  console.error(`Schema governance check failed:\n- ${failures.join('\n- ')}`)
  process.exit(1)
}

console.log('Schema governance check passed.')
