import { readFile, writeFile } from 'node:fs/promises'

import { generatedSchemaPath, openApiSpecPath, readOpenApiDocument } from './governance-lib.mjs'
import { renderGeneratedSchema } from './schema-generator-lib.mjs'

const document = await readOpenApiDocument()
const source = await readFile(openApiSpecPath, 'utf8')
const output = renderGeneratedSchema(document, source)

await writeFile(generatedSchemaPath, output)
console.log(`Generated ${generatedSchemaPath}.`)
