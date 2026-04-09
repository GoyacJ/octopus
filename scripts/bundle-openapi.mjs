import { openApiSourceEntryPath, openApiSpecPath } from './governance-lib.mjs'
import { readBundleCliArgs, writeBundledOpenApi } from './openapi-bundle-lib.mjs'

const args = readBundleCliArgs()
const rootPath = args.root ? String(args.root) : openApiSourceEntryPath
const outputPath = args.output ? String(args.output) : openApiSpecPath

await writeBundledOpenApi({ rootPath, outputPath })
console.log(`Bundled OpenAPI artifact to ${outputPath}.`)
