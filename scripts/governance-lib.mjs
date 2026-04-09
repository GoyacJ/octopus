import { createHash } from 'node:crypto'
import { readFile, writeFile } from 'node:fs/promises'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

import { parse, stringify } from 'yaml'

const __dirname = path.dirname(fileURLToPath(import.meta.url))

export const repoRoot = path.resolve(__dirname, '..')
export const versionFilePath = path.join(repoRoot, 'VERSION')
export const cargoManifestPath = path.join(repoRoot, 'Cargo.toml')
export const openApiSourceRootPath = path.join(repoRoot, 'contracts', 'openapi', 'src')
export const openApiSourceEntryPath = path.join(openApiSourceRootPath, 'root.yaml')
export const openApiInfoPath = path.join(openApiSourceRootPath, 'info.yaml')
export const openApiSpecPath = path.join(repoRoot, 'contracts', 'openapi', 'octopus.openapi.yaml')
export const generatedSchemaPath = path.join(repoRoot, 'packages', 'schema', 'src', 'generated.ts')
export const releasePlatformArtifactRules = {
  macos: {
    artifactExtensions: ['.dmg', '.zip'],
    requiredArtifacts: [
      {
        label: 'Apple Silicon installer (.dmg or .zip)',
        pattern: /(aarch64|arm64).*\.(dmg|zip)$/i,
      },
      {
        label: 'Intel installer (.dmg or .zip)',
        pattern: /(x86_64|x64|intel).*\.(dmg|zip)$/i,
      },
    ],
  },
  linux: {
    artifactExtensions: ['.appimage', '.deb'],
    requiredArtifacts: [
      {
        label: 'AppImage bundle (.AppImage)',
        pattern: /\.appimage$/i,
      },
      {
        label: 'Debian package (.deb)',
        pattern: /\.deb$/i,
      },
    ],
  },
  windows: {
    artifactExtensions: ['.msi', '.exe'],
    requiredArtifacts: [
      {
        label: 'x64 NSIS installer (.exe)',
        pattern: /(x86_64|x64).*\.(msi|exe)$/i,
      },
      {
        label: 'ARM64 NSIS installer (.exe)',
        pattern: /arm64.*\.(msi|exe)$/i,
      },
    ],
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

export async function readOpenApiDocument(source = openApiSpecPath) {
  return parse(await readFile(source, 'utf8'))
}

export async function writeOpenApiDocument(document, target = openApiSpecPath) {
  await writeFile(target, stringify(document, { lineWidth: 0 }))
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

  const openApiInfo = parse(await readFile(openApiInfoPath, 'utf8')) ?? {}
  openApiInfo.version = version
  await writeOpenApiDocument(openApiInfo, openApiInfoPath)

  const { writeBundledOpenApi } = await import('./openapi-bundle-lib.mjs')
  await writeBundledOpenApi({
    rootPath: openApiSourceEntryPath,
    outputPath: openApiSpecPath,
  })

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

  const openApiInfo = parse(await readFile(openApiInfoPath, 'utf8'))
  const openApiSourceVersion = openApiInfo?.version
  if (openApiSourceVersion !== version) {
    mismatches.push(`contracts/openapi/src/info.yaml: expected ${version}, received ${openApiSourceVersion ?? '<missing>'}`)
  }

  return { mismatches, version }
}

export function sha256(value) {
  return createHash('sha256').update(value).digest('hex')
}
