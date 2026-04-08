import { createHash } from 'node:crypto'
import { readFile, writeFile } from 'node:fs/promises'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

import { parse, stringify } from 'yaml'

const __dirname = path.dirname(fileURLToPath(import.meta.url))

export const repoRoot = path.resolve(__dirname, '..')
export const versionFilePath = path.join(repoRoot, 'VERSION')
export const cargoManifestPath = path.join(repoRoot, 'Cargo.toml')
export const openApiSpecPath = path.join(repoRoot, 'contracts', 'openapi', 'octopus.openapi.yaml')
export const generatedSchemaPath = path.join(repoRoot, 'packages', 'schema', 'src', 'generated.ts')
export const releasePlatformArtifactRules = {
  macos: {
    artifactExtensions: ['.dmg', '.zip'],
    requiredExtensions: ['.dmg', '.zip'],
  },
  windows: {
    artifactExtensions: ['.msi', '.exe'],
    requiredExtensions: ['.msi', '.exe'],
  },
}

export const mirroredVersionJsonFiles = [
  'package.json',
  'apps/desktop/package.json',
  'packages/schema/package.json',
  'packages/ui/package.json',
  'apps/desktop/src-tauri/tauri.conf.json',
]

export async function readVersion() {
  return (await readFile(versionFilePath, 'utf8')).trim()
}

export async function readJson(relativePath) {
  const filePath = path.join(repoRoot, relativePath)
  return JSON.parse(await readFile(filePath, 'utf8'))
}

export async function writeJson(relativePath, value) {
  const filePath = path.join(repoRoot, relativePath)
  await writeFile(filePath, `${JSON.stringify(value, null, 2)}\n`)
}

export async function readText(relativePath) {
  return await readFile(path.join(repoRoot, relativePath), 'utf8')
}

export async function writeText(relativePath, value) {
  await writeFile(path.join(repoRoot, relativePath), value)
}

export async function readOpenApiDocument() {
  return parse(await readFile(openApiSpecPath, 'utf8'))
}

export async function writeOpenApiDocument(document) {
  await writeFile(openApiSpecPath, stringify(document, { lineWidth: 0 }))
}

export async function syncMirroredVersions() {
  const version = await readVersion()

  for (const relativePath of mirroredVersionJsonFiles) {
    const json = await readJson(relativePath)
    json.version = version
    await writeJson(relativePath, json)
  }

  const cargoManifest = await readFile(cargoManifestPath, 'utf8')
  const nextCargoManifest = cargoManifest.replace(
    /(\[workspace\.package\][\s\S]*?^version\s*=\s*")[^"]+(")/m,
    `$1${version}$2`,
  )
  await writeFile(cargoManifestPath, nextCargoManifest)

  const openApiDocument = await readOpenApiDocument()
  openApiDocument.info ??= {}
  openApiDocument.info.version = version
  await writeOpenApiDocument(openApiDocument)

  return version
}

export async function collectVersionMismatches() {
  const version = await readVersion()
  const mismatches = []

  for (const relativePath of mirroredVersionJsonFiles) {
    const json = await readJson(relativePath)
    if (json.version !== version) {
      mismatches.push(`${relativePath}: expected ${version}, received ${json.version ?? '<missing>'}`)
    }
  }

  const cargoManifest = await readFile(cargoManifestPath, 'utf8')
  const cargoVersion = cargoManifest.match(/\[workspace\.package\][\s\S]*?^version\s*=\s*"([^"]+)"/m)?.[1]
  if (cargoVersion !== version) {
    mismatches.push(`Cargo.toml: expected ${version}, received ${cargoVersion ?? '<missing>'}`)
  }

  const openApiDocument = await readOpenApiDocument()
  const openApiVersion = openApiDocument?.info?.version
  if (openApiVersion !== version) {
    mismatches.push(`contracts/openapi/octopus.openapi.yaml: expected ${version}, received ${openApiVersion ?? '<missing>'}`)
  }

  return { mismatches, version }
}

export function sha256(value) {
  return createHash('sha256').update(value).digest('hex')
}
