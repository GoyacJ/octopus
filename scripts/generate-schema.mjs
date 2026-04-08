import { readFile, writeFile } from 'node:fs/promises'

import { generatedSchemaPath, openApiSpecPath, readOpenApiDocument, sha256 } from './governance-lib.mjs'

function toTypeName(ref) {
  return ref.split('/').at(-1)
}

function isIdentifier(value) {
  return /^[A-Za-z_$][A-Za-z0-9_$]*$/.test(value)
}

function quote(value) {
  return JSON.stringify(value)
}

function renderType(schema) {
  if (!schema) {
    return 'unknown'
  }

  if (schema.$ref) {
    return toTypeName(schema.$ref)
  }

  if (schema.enum) {
    return schema.enum.map((entry) => quote(entry)).join(' | ')
  }

  if (schema.oneOf) {
    return schema.oneOf.map((entry) => renderType(entry)).join(' | ')
  }

  if (schema.anyOf) {
    return schema.anyOf.map((entry) => renderType(entry)).join(' | ')
  }

  if (schema.allOf) {
    return schema.allOf.map((entry) => renderType(entry)).join(' & ')
  }

  if (Array.isArray(schema.type)) {
    const members = schema.type.map((entry) => renderType({ ...schema, type: entry }))
    return [...new Set(members)].join(' | ')
  }

  switch (schema.type) {
    case 'string':
      return 'string'
    case 'integer':
    case 'number':
      return 'number'
    case 'boolean':
      return 'boolean'
    case 'null':
      return 'null'
    case 'array':
      return `${renderType(schema.items)}[]`
    case 'object':
      if (schema.properties) {
        const required = new Set(schema.required ?? [])
        const lines = Object.entries(schema.properties).map(([name, value]) => {
          const key = isIdentifier(name) ? name : quote(name)
          const optional = required.has(name) ? '' : '?'
          return `  ${key}${optional}: ${renderType(value)}`
        })
        if (schema.additionalProperties) {
          lines.push(`  [key: string]: ${schema.additionalProperties === true ? 'unknown' : renderType(schema.additionalProperties)}`)
        }
        return `{\n${lines.join('\n')}\n}`
      }
      if (schema.additionalProperties) {
        return `Record<string, ${schema.additionalProperties === true ? 'unknown' : renderType(schema.additionalProperties)}>`
      }
      return 'Record<string, unknown>'
    default:
      return 'unknown'
  }
}

function renderDeclaration(name, schema) {
  if (schema.enum || schema.oneOf || schema.anyOf || schema.allOf || schema.type !== 'object' || schema.additionalProperties === true) {
    return `export type ${name} = ${renderType(schema)}\n`
  }

  if (!schema.properties) {
    return `export type ${name} = ${renderType(schema)}\n`
  }

  const required = new Set(schema.required ?? [])
  const lines = Object.entries(schema.properties).map(([propertyName, propertySchema]) => {
    const key = isIdentifier(propertyName) ? propertyName : quote(propertyName)
    const optional = required.has(propertyName) ? '' : '?'
    return `  ${key}${optional}: ${renderType(propertySchema)}`
  })

  if (schema.additionalProperties) {
    lines.push(`  [key: string]: ${schema.additionalProperties === true ? 'unknown' : renderType(schema.additionalProperties)}`)
  }

  return `export interface ${name} {\n${lines.join('\n')}\n}\n`
}

function renderPaths(paths) {
  const lines = []

  for (const [route, operations] of Object.entries(paths ?? {})) {
    const methodLines = []
    for (const [method, operation] of Object.entries(operations)) {
      const responseSchema = operation.responses?.['200']?.content?.['application/json']?.schema
      const defaultErrorSchema = operation.responses?.default?.content?.['application/json']?.schema
      const responseType = renderType(responseSchema)
      const errorType = renderType(defaultErrorSchema)
      methodLines.push(`    ${method}: { operationId: ${quote(operation.operationId ?? `${method}${route}`)}; response: ${responseType}; error: ${errorType} }`)
    }
    lines.push(`  ${quote(route)}: {\n${methodLines.join('\n')}\n  }`)
  }

  return `export interface OctopusApiPaths {\n${lines.join('\n')}\n}\n`
}

const document = await readOpenApiDocument()
const source = await readFile(openApiSpecPath, 'utf8')
const schemaHash = sha256(source)

const declarations = Object.entries(document.components?.schemas ?? {})
  .map(([name, schema]) => renderDeclaration(name, schema))
  .join('\n')

const output = `/* eslint-disable */
// Generated from contracts/openapi/octopus.openapi.yaml by scripts/generate-schema.mjs.
// Source hash: ${schemaHash}

export const OCTOPUS_OPENAPI_VERSION = ${quote(document.openapi)}
export const OCTOPUS_API_VERSION = ${quote(document.info?.version ?? '0.0.0')}
export const OCTOPUS_OPENAPI_SOURCE_HASH = ${quote(schemaHash)}

${declarations}

${renderPaths(document.paths)}
`

await writeFile(generatedSchemaPath, output)
console.log(`Generated ${generatedSchemaPath}.`)
