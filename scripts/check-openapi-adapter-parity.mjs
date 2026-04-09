import {
  adapterParityAllowlistPath,
  collectAdapterRoutes,
  compareOpenApiCoverage,
  formatParityFailure,
  readOpenApiPaths,
  readParityAllowlist,
} from './openapi-parity-lib.mjs'

const actual = await collectAdapterRoutes()
const openApiPaths = await readOpenApiPaths()
const allowlist = await readParityAllowlist(adapterParityAllowlistPath)
const result = compareOpenApiCoverage(actual, openApiPaths, allowlist)

if (result.missing.length || result.staleAllowlist.length) {
  process.stderr.write(
    formatParityFailure('OpenAPI adapter parity check', result.missing, result.staleAllowlist),
  )
  process.exit(1)
}

console.log(`OpenAPI adapter parity check passed for ${result.actual.length} normalized routes.`)
