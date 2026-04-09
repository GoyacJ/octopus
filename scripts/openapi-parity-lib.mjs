import { readFile } from 'node:fs/promises'
import path from 'node:path'

import YAML from 'yaml'

import { openApiSpecPath, repoRoot } from './governance-lib.mjs'

export const routeParityAllowlistPath = path.join(repoRoot, 'contracts', 'openapi', 'route-parity-allowlist.json')
export const adapterParityAllowlistPath = path.join(repoRoot, 'contracts', 'openapi', 'adapter-parity-allowlist.json')

const serverRoutesSourcePath = path.join(repoRoot, 'crates', 'octopus-server', 'src', 'lib.rs')
const adapterSourcePaths = [
  path.join(repoRoot, 'apps', 'desktop', 'src', 'tauri', 'shell.ts'),
  path.join(repoRoot, 'apps', 'desktop', 'src', 'tauri', 'workspace-client.ts'),
]

function unique(values) {
  return [...new Set(values)]
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

export function normalizeComparableApiPath(value) {
  let normalized = String(value).trim()
  const queryIndex = normalized.indexOf('?')
  if (queryIndex >= 0) {
    normalized = normalized.slice(0, queryIndex)
  }

  normalized = normalized.replace(/\$\{[^}]*\}/g, '{param}')
  normalized = normalized.replace(/\$\{.*$/, '')
  normalized = normalized.replace(/:[A-Za-z0-9_]+/g, '{param}')
  normalized = normalized.replace(/\*[A-Za-z0-9_]+/g, '{param}')
  normalized = normalized.replace(/\{[^}]+\}/g, '{param}')
  normalized = normalized.replace(/(?<!\/)\{param\}/g, '')
  normalized = normalized.replace(/\/+/g, '/')

  if (normalized.length > 1 && normalized.endsWith('/')) {
    normalized = normalized.slice(0, -1)
  }

  return normalized
}

export async function readOpenApiPaths() {
  const document = YAML.parse(await readFile(openApiSpecPath, 'utf8'))
  return unique(Object.keys(document.paths ?? {}).map(normalizeComparableApiPath)).sort()
}

function sliceBetween(source, startMarker, endMarker) {
  const startIndex = source.indexOf(startMarker)
  const endIndex = source.indexOf(endMarker)
  if (startIndex < 0 || endIndex < 0 || endIndex <= startIndex) {
    return ''
  }

  return source.slice(startIndex, endIndex)
}

function extractRouteLiterals(source) {
  return Array.from(source.matchAll(/\.route\("([^"]+)"/g)).map((match) => match[1])
}

export async function collectServerRoutes() {
  const source = await readFile(serverRoutesSourcePath, 'utf8')
  const buildSection = sliceBetween(
    source,
    'pub fn build_router(state: ServerState) -> Router {',
    'fn runtime_routes() -> Router<ServerState> {',
  )
  const runtimeSection = sliceBetween(
    source,
    'fn runtime_routes() -> Router<ServerState> {',
    'async fn runtime_bootstrap(',
  )

  const rootRoutes = extractRouteLiterals(buildSection).filter((route) => route.startsWith('/api/v1/'))
  const runtimeRoutes = extractRouteLiterals(runtimeSection).map((route) => `/api/v1/runtime${route}`)

  return unique([...rootRoutes, ...runtimeRoutes].map(normalizeComparableApiPath)).sort()
}

function extractDirectApiPaths(source) {
  const paths = []
  for (const match of source.matchAll(/['"`](\/api\/v1\/[^'"`\n]+)['"`]/g)) {
    paths.push(match[1])
  }

  return paths
}

function extractTemplateApiPaths(source, marker, prefix) {
  const pattern = new RegExp('\\$\\{' + escapeRegExp(marker) + '\\}([^\\n`]+)', 'g')
  const paths = []
  for (const match of source.matchAll(pattern)) {
    paths.push(`${prefix}${match[1]}`)
  }

  return paths
}

export async function collectAdapterRoutes() {
  const discovered = []
  for (const filePath of adapterSourcePaths) {
    const source = await readFile(filePath, 'utf8')
    discovered.push(...extractDirectApiPaths(source))
    discovered.push(...extractTemplateApiPaths(source, 'API_BASE', '/api/v1'))
    discovered.push(...extractTemplateApiPaths(source, 'RUNTIME_API_BASE', '/api/v1/runtime'))
  }

  return unique(discovered.map(normalizeComparableApiPath)).sort()
}

export async function readParityAllowlist(filePath) {
  const payload = JSON.parse(await readFile(filePath, 'utf8'))
  return unique((payload.paths ?? []).map(normalizeComparableApiPath)).sort()
}

export function compareOpenApiCoverage(actualPaths, openApiPaths, allowlist) {
  const actual = unique(actualPaths.map(normalizeComparableApiPath)).sort()
  const openapi = new Set(openApiPaths.map(normalizeComparableApiPath))
  const allowed = new Set(allowlist.map(normalizeComparableApiPath))

  const missing = actual.filter((apiPath) => !openapi.has(apiPath) && !allowed.has(apiPath))
  const staleAllowlist = [...allowed].filter((apiPath) => !actual.includes(apiPath) || openapi.has(apiPath))

  return {
    actual,
    missing,
    staleAllowlist,
  }
}

export function formatParityFailure(title, missing, staleAllowlist) {
  const lines = [`${title} failed:`]

  if (missing.length) {
    lines.push('- Missing from OpenAPI and allowlist:')
    for (const apiPath of missing) {
      lines.push(`  - ${apiPath}`)
    }
  }

  if (staleAllowlist.length) {
    lines.push('- Stale allowlist entries:')
    for (const apiPath of staleAllowlist) {
      lines.push(`  - ${apiPath}`)
    }
  }

  return `${lines.join('\n')}\n`
}
