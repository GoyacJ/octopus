import { mkdir, readFile, writeFile } from 'node:fs/promises'
import path from 'node:path'

import YAML from 'yaml'

const MERGE_DIRECTIVE = 'x-octopus-merge'
const ROOT_KEY_ORDER = ['openapi', 'info', 'servers', 'tags', 'paths', 'components']
const COMPONENT_KEY_ORDER = ['schemas', 'parameters', 'responses', 'requestBodies', 'headers', 'securitySchemes']
const HTTP_METHOD_ORDER = ['get', 'post', 'put', 'patch', 'delete', 'options', 'head', 'trace']

function isPlainObject(value) {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value)
}

function decodePointerSegment(segment) {
  return segment.replaceAll('~1', '/').replaceAll('~0', '~')
}

function splitRef(ref) {
  const hashIndex = ref.indexOf('#')
  if (hashIndex < 0) {
    return { filePath: ref, pointer: '' }
  }

  return {
    filePath: ref.slice(0, hashIndex),
    pointer: ref.slice(hashIndex + 1),
  }
}

function resolvePointer(document, pointer, sourceLabel) {
  if (!pointer || pointer === '/') {
    return document
  }

  if (!pointer.startsWith('/')) {
    throw new Error(`Invalid ref pointer "${pointer}" in ${sourceLabel}`)
  }

  let current = document
  for (const rawSegment of pointer.slice(1).split('/')) {
    const segment = decodePointerSegment(rawSegment)
    if (Array.isArray(current)) {
      const index = Number.parseInt(segment, 10)
      if (Number.isNaN(index) || index < 0 || index >= current.length) {
        throw new Error(`Unresolved ref pointer "${pointer}" in ${sourceLabel}`)
      }
      current = current[index]
      continue
    }

    if (!isPlainObject(current) || !(segment in current)) {
      throw new Error(`Unresolved ref pointer "${pointer}" in ${sourceLabel}`)
    }
    current = current[segment]
  }

  return current
}

function compareKeys(left, right, scope) {
  if (scope.at(-1) === 'paths') {
    return left.localeCompare(right)
  }

  if (scope.at(-1) === 'schemas' || scope.at(-1) === 'parameters' || scope.at(-1) === 'responses') {
    return left.localeCompare(right)
  }

  if (scope.at(-1)?.startsWith('/api/')) {
    const leftIndex = HTTP_METHOD_ORDER.indexOf(left)
    const rightIndex = HTTP_METHOD_ORDER.indexOf(right)
    if (leftIndex >= 0 && rightIndex >= 0) {
      return leftIndex - rightIndex
    }
    if (leftIndex >= 0) {
      return -1
    }
    if (rightIndex >= 0) {
      return 1
    }
  }

  if (scope.length === 0) {
    const leftIndex = ROOT_KEY_ORDER.indexOf(left)
    const rightIndex = ROOT_KEY_ORDER.indexOf(right)
    if (leftIndex >= 0 && rightIndex >= 0) {
      return leftIndex - rightIndex
    }
    if (leftIndex >= 0) {
      return -1
    }
    if (rightIndex >= 0) {
      return 1
    }
  }

  if (scope.at(-1) === 'components') {
    const leftIndex = COMPONENT_KEY_ORDER.indexOf(left)
    const rightIndex = COMPONENT_KEY_ORDER.indexOf(right)
    if (leftIndex >= 0 && rightIndex >= 0) {
      return leftIndex - rightIndex
    }
    if (leftIndex >= 0) {
      return -1
    }
    if (rightIndex >= 0) {
      return 1
    }
  }

  return left.localeCompare(right)
}

function sortValue(value, scope = []) {
  if (Array.isArray(value)) {
    return value.map((entry) => sortValue(entry, scope))
  }

  if (!isPlainObject(value)) {
    return value
  }

  const sortedEntries = Object.entries(value)
    .sort(([left], [right]) => compareKeys(left, right, scope))
    .map(([key, entry]) => [key, sortValue(entry, [...scope, key])])

  return Object.fromEntries(sortedEntries)
}

function parseCliArgs(argv) {
  const parsed = {}
  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index]
    if (!token?.startsWith('--')) {
      continue
    }

    const key = token.slice(2)
    const next = argv[index + 1]
    if (!next || next.startsWith('--')) {
      parsed[key] = true
      continue
    }

    parsed[key] = next
    index += 1
  }

  return parsed
}

function formatSourceLocation(filePath, pointer = '') {
  return pointer ? `${filePath}#${pointer}` : filePath
}

async function readYamlDocument(filePath) {
  const source = await readFile(filePath, 'utf8')
  return YAML.parse(source) ?? {}
}

async function resolveExternalRef(ref, filePath, state) {
  const { filePath: relativePath, pointer } = splitRef(ref)
  const targetPath = path.resolve(path.dirname(filePath), relativePath)
  const cacheKey = formatSourceLocation(targetPath, pointer)

  if (state.refStack.includes(cacheKey)) {
    throw new Error(`Detected circular OpenAPI source ref: ${[...state.refStack, cacheKey].join(' -> ')}`)
  }

  let document = state.documentCache.get(targetPath)
  if (document === undefined) {
    try {
      document = await readYamlDocument(targetPath)
    } catch (error) {
      throw new Error(`Unable to resolve OpenAPI ref "${ref}" from ${filePath}: ${error.message}`)
    }
    state.documentCache.set(targetPath, document)
  }

  const target = resolvePointer(document, pointer, cacheKey)
  return await resolveValue(target, {
    ...state,
    currentFilePath: targetPath,
    refStack: [...state.refStack, cacheKey],
  })
}

function assertMergeSource(value, sourceLabel) {
  if (!isPlainObject(value)) {
    throw new Error(`OpenAPI merge source ${sourceLabel} must resolve to an object map`)
  }
}

function mergeUniqueMaps(target, incoming, sourceLabel) {
  for (const [key, value] of Object.entries(incoming)) {
    if (key in target) {
      throw new Error(`Duplicate OpenAPI key "${key}" while merging ${sourceLabel}`)
    }
    target[key] = value
  }
}

async function resolveValue(value, state) {
  if (Array.isArray(value)) {
    return await Promise.all(value.map((entry) => resolveValue(entry, state)))
  }

  if (!isPlainObject(value)) {
    return value
  }

  if (MERGE_DIRECTIVE in value) {
    const mergeTargets = value[MERGE_DIRECTIVE]
    if (!Array.isArray(mergeTargets)) {
      throw new Error(`${MERGE_DIRECTIVE} must be an array in ${state.currentFilePath}`)
    }

    const merged = {}
    for (const mergeTarget of mergeTargets) {
      if (typeof mergeTarget !== 'string') {
        throw new Error(`${MERGE_DIRECTIVE} entries must be string refs in ${state.currentFilePath}`)
      }
      const resolved = await resolveExternalRef(mergeTarget, state.currentFilePath, state)
      assertMergeSource(resolved, formatSourceLocation(state.currentFilePath, mergeTarget))
      mergeUniqueMaps(merged, resolved, formatSourceLocation(state.currentFilePath, mergeTarget))
    }
    return merged
  }

  if (typeof value.$ref === 'string' && Object.keys(value).length === 1 && !value.$ref.startsWith('#')) {
    return await resolveExternalRef(value.$ref, state.currentFilePath, state)
  }

  const entries = await Promise.all(
    Object.entries(value).map(async ([key, entry]) => [key, await resolveValue(entry, state)]),
  )

  return Object.fromEntries(entries)
}

function validatePathFiles(document) {
  const errors = []

  function visit(node, trail = []) {
    if (Array.isArray(node)) {
      node.forEach((entry, index) => visit(entry, [...trail, String(index)]))
      return
    }

    if (!isPlainObject(node)) {
      return
    }

    if ('schema' in node && isPlainObject(node.schema) && !node.schema.$ref) {
      const schema = node.schema
      const hasComplexShape =
        Boolean(schema.properties)
        || Boolean(schema.oneOf)
        || Boolean(schema.anyOf)
        || Boolean(schema.allOf)
        || Boolean(schema.enum)
        || (schema.type === 'object')

      if (hasComplexShape) {
        errors.push(`paths source contains inline schema at ${trail.join('.') || '<root>'}`)
      }
    }

    for (const [key, value] of Object.entries(node)) {
      visit(value, [...trail, key])
    }
  }

  visit(document)
  return errors
}

export async function bundleOpenApiDocument({ rootPath }) {
  const absoluteRootPath = path.resolve(rootPath)
  const rootDocument = await readYamlDocument(absoluteRootPath)
  const bundled = await resolveValue(rootDocument, {
    currentFilePath: absoluteRootPath,
    documentCache: new Map([[absoluteRootPath, rootDocument]]),
    refStack: [absoluteRootPath],
  })

  const pathInlineSchemaErrors = []
  if (Array.isArray(rootDocument?.paths?.[MERGE_DIRECTIVE])) {
    for (const mergeTarget of rootDocument.paths[MERGE_DIRECTIVE]) {
      const pathDocument = await resolveExternalRef(mergeTarget, absoluteRootPath, {
        currentFilePath: absoluteRootPath,
        documentCache: new Map([[absoluteRootPath, rootDocument]]),
        refStack: [absoluteRootPath],
      })
      pathInlineSchemaErrors.push(...validatePathFiles(pathDocument))
    }
  }

  if (pathInlineSchemaErrors.length) {
    throw new Error(pathInlineSchemaErrors.join('\n'))
  }

  return sortValue(bundled)
}

export async function bundleOpenApiYaml({ rootPath }) {
  const bundledDocument = await bundleOpenApiDocument({ rootPath })
  return YAML.stringify(bundledDocument, {
    lineWidth: 0,
    sortMapEntries: false,
  })
}

export async function writeBundledOpenApi({ rootPath, outputPath }) {
  const bundledYaml = await bundleOpenApiYaml({ rootPath })
  await mkdir(path.dirname(outputPath), { recursive: true })
  await writeFile(outputPath, bundledYaml)
  return bundledYaml
}

export function readBundleCliArgs(argv = process.argv.slice(2)) {
  return parseCliArgs(argv)
}
