import { sha256 } from './governance-lib.mjs'

function toTypeName(ref) {
  return ref.split('/').at(-1)
}

function isIdentifier(value) {
  return /^[A-Za-z_$][A-Za-z0-9_$]*$/.test(value)
}

function quote(value) {
  return JSON.stringify(value)
}

function joinUnionMembers(members) {
  return [...new Set(
    members
      .flatMap(member => member.split('|'))
      .map(member => member.trim())
      .filter(Boolean),
  )].join(' | ')
}

function withNullable(type, schema) {
  if (!schema?.nullable) {
    return type
  }

  return joinUnionMembers([type, 'null'])
}

function renderType(schema) {
  if (!schema) {
    return 'unknown'
  }

  if (schema.$ref) {
    return withNullable(toTypeName(schema.$ref), schema)
  }

  if (schema.enum) {
    return withNullable(schema.enum.map((entry) => quote(entry)).join(' | '), schema)
  }

  if (schema.oneOf) {
    return withNullable(joinUnionMembers(schema.oneOf.map((entry) => renderType(entry))), schema)
  }

  if (schema.anyOf) {
    return withNullable(joinUnionMembers(schema.anyOf.map((entry) => renderType(entry))), schema)
  }

  if (schema.allOf) {
    return withNullable(schema.allOf.map((entry) => renderType(entry)).join(' & '), schema)
  }

  if (Array.isArray(schema.type)) {
    const members = schema.type.map((entry) => renderType({ ...schema, nullable: false, type: entry }))
    return withNullable(joinUnionMembers(members), schema)
  }

  let type
  switch (schema.type) {
    case 'string':
      type = 'string'
      break
    case 'integer':
    case 'number':
      type = 'number'
      break
    case 'boolean':
      type = 'boolean'
      break
    case 'null':
      type = 'null'
      break
    case 'array':
      type = `${renderType(schema.items)}[]`
      break
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
        type = `{\n${lines.join('\n')}\n}`
        break
      }
      if (schema.additionalProperties) {
        type = `Record<string, ${schema.additionalProperties === true ? 'unknown' : renderType(schema.additionalProperties)}>`
        break
      }
      type = 'Record<string, unknown>'
      break
    default:
      type = 'unknown'
      break
  }

  return withNullable(type, schema)
}

function renderDeclaration(name, schema) {
  if (
    schema.enum
    || schema.oneOf
    || schema.anyOf
    || schema.allOf
    || schema.nullable
    || schema.type !== 'object'
    || schema.additionalProperties === true
  ) {
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

function resolveSuccessResponse(operation) {
  const entries = Object.entries(operation.responses ?? {})
    .filter(([statusCode]) => /^2\d\d$/.test(statusCode))
    .sort(([left], [right]) => Number(left) - Number(right))

  if (!entries.length) {
    return null
  }

  return entries[0]?.[1] ?? null
}

function renderResponseType(response) {
  const schema = response?.content?.['application/json']?.schema
  if (!schema) {
    return 'void'
  }

  return renderType(schema)
}

function renderPaths(paths) {
  const lines = []

  for (const [route, operations] of Object.entries(paths ?? {})) {
    const methodLines = []
    for (const [method, operation] of Object.entries(operations)) {
      const successResponse = resolveSuccessResponse(operation)
      const defaultErrorSchema = operation.responses?.default?.content?.['application/json']?.schema
      const responseType = renderResponseType(successResponse)
      const errorType = renderType(defaultErrorSchema)
      methodLines.push(`    ${method}: { operationId: ${quote(operation.operationId ?? `${method}${route}`)}; response: ${responseType}; error: ${errorType} }`)
    }
    lines.push(`  ${quote(route)}: {\n${methodLines.join('\n')}\n  }`)
  }

  return `export interface OctopusApiPaths {\n${lines.join('\n')}\n}\n`
}

export function renderGeneratedSchema(document, sourceText) {
  const schemaHash = sha256(sourceText)
  const declarations = Object.entries(document.components?.schemas ?? {})
    .map(([name, schema]) => renderDeclaration(name, schema))
    .join('\n')

  return `/* eslint-disable */
// Generated from contracts/openapi/octopus.openapi.yaml by scripts/generate-schema.mjs.
// Source hash: ${schemaHash}

export const OCTOPUS_OPENAPI_VERSION = ${quote(document.openapi)}
export const OCTOPUS_API_VERSION = ${quote(document.info?.version ?? '0.0.0')}
export const OCTOPUS_OPENAPI_SOURCE_HASH = ${quote(schemaHash)}

${declarations}

${renderPaths(document.paths)}
`
}
