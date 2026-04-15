import { readFile } from 'node:fs/promises'
import path from 'node:path'

import YAML from 'yaml'

import { openApiSpecPath, repoRoot } from './governance-lib.mjs'

export const routeParityAllowlistPath = path.join(repoRoot, 'contracts', 'openapi', 'route-parity-allowlist.json')
export const adapterParityAllowlistPath = path.join(repoRoot, 'contracts', 'openapi', 'adapter-parity-allowlist.json')

const serverRoutesSourcePath = path.join(repoRoot, 'crates', 'octopus-server', 'src', 'routes.rs')
const adapterSourcePaths = [
  path.join(repoRoot, 'apps', 'desktop', 'src', 'tauri', 'workspace_api.ts'),
  path.join(repoRoot, 'apps', 'desktop', 'src', 'tauri', 'runtime_api.ts'),
  path.join(repoRoot, 'apps', 'desktop', 'src', 'tauri', 'runtime_events.ts'),
  path.join(repoRoot, 'apps', 'desktop', 'src', 'tauri', 'shell_browser.ts'),
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

function extractFunctionBlock(source, signature) {
  const signatureIndex = source.indexOf(signature)
  if (signatureIndex < 0) {
    return ''
  }

  const blockStart = source.indexOf('{', signatureIndex)
  if (blockStart < 0) {
    return ''
  }

  let depth = 0
  for (let index = blockStart; index < source.length; index += 1) {
    const character = source[index]
    if (character === '{') {
      depth += 1
    } else if (character === '}') {
      depth -= 1
      if (depth === 0) {
        return source.slice(signatureIndex, index + 1)
      }
    }
  }

  return ''
}

function extractRouteLiterals(source) {
  return Array.from(source.matchAll(/\.route\(\s*"([^"]+)"/g)).map((match) => match[1])
}

export async function collectServerRoutes() {
  const source = await readFile(serverRoutesSourcePath, 'utf8')
  const buildSection = extractFunctionBlock(source, 'pub fn build_router(state: ServerState) -> Router {')
  const runtimeSection = extractFunctionBlock(source, 'pub(crate) fn runtime_routes() -> Router<ServerState> {')

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
