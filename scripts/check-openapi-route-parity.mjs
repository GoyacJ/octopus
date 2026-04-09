import {
  collectServerRoutes,
  compareOpenApiCoverage,
  formatParityFailure,
  readOpenApiPaths,
  readParityAllowlist,
  routeParityAllowlistPath,
} from './openapi-parity-lib.mjs'

const actual = await collectServerRoutes()
const openApiPaths = await readOpenApiPaths()
const allowlist = await readParityAllowlist(routeParityAllowlistPath)
const result = compareOpenApiCoverage(actual, openApiPaths, allowlist)

if (result.missing.length || result.staleAllowlist.length) {
  process.stderr.write(
    formatParityFailure('OpenAPI route parity check', result.missing, result.staleAllowlist),
  )
  process.exit(1)
}

console.log(`OpenAPI route parity check passed for ${result.actual.length} normalized routes.`)
