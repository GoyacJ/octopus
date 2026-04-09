import { mkdir, readdir, readFile, writeFile } from 'node:fs/promises'
import path from 'node:path'

import { repoRoot } from './governance-lib.mjs'

function readArgument(flag) {
  const index = process.argv.indexOf(flag)
  return index >= 0 ? process.argv[index + 1] : undefined
}

async function collectFiles(directory) {
  const entries = await readdir(directory, { withFileTypes: true }).catch(() => [])
  const files = []
  for (const entry of entries) {
    const resolved = path.join(directory, entry.name)
    if (entry.isDirectory()) {
      files.push(...await collectFiles(resolved))
      continue
    }
    files.push(resolved)
  }
  return files
}

function normalizeForMatch(filePath) {
  return filePath.replaceAll(path.sep, '/').toLowerCase()
}

function detectArchitecture(value) {
  if (/(^|[^a-z0-9])(aarch64|arm64)([^a-z0-9]|$)/i.test(value)) {
    return 'aarch64'
  }
  if (/(^|[^a-z0-9])(x86_64|x64|amd64|intel)([^a-z0-9]|$)/i.test(value)) {
    return 'x86_64'
  }
  return undefined
}

function unique(values) {
  return [...new Set(values.filter(Boolean))]
}

function detectPlatformKey(platform, updaterPath, platformFiles) {
  const updaterDirectory = path.dirname(updaterPath)
  const sameDirectoryFiles = platformFiles.filter((filePath) => path.dirname(filePath) === updaterDirectory)
  const localArchitectureCandidates = unique([
    detectArchitecture(normalizeForMatch(updaterPath)),
    ...sameDirectoryFiles.map((filePath) => detectArchitecture(normalizeForMatch(filePath))),
  ])
  const globalArchitectureCandidates = unique(platformFiles.map((filePath) => detectArchitecture(normalizeForMatch(filePath))))
  const architectureCandidates = localArchitectureCandidates.length > 0
    ? localArchitectureCandidates
    : globalArchitectureCandidates

  if (architectureCandidates.length !== 1) {
    throw new Error(
      `unable to determine updater architecture for ${platform} asset ${path.basename(updaterPath)} under ${path.relative(repoRoot, updaterDirectory)}`,
    )
  }

  const architecture = architectureCandidates[0]
  if (platform === 'macos') {
    return `darwin-${architecture}`
  }
  if (platform === 'linux') {
    return `linux-${architecture}`
  }
  return `windows-${architecture}`
}

function isSupportedUpdaterAsset(platform, filePath) {
  const normalized = normalizeForMatch(filePath)

  if (platform === 'macos') {
    return normalized.endsWith('.app.tar.gz')
  }

  if (platform === 'linux') {
    return normalized.endsWith('.appimage') || normalized.endsWith('.appimage.tar.gz')
  }

  return normalized.endsWith('.exe') || normalized.endsWith('.msi') || normalized.endsWith('.zip')
}

async function buildPlatformManifest(platform, platformDir, version) {
  const platformFiles = (await collectFiles(platformDir)).sort()
  const signatureFiles = platformFiles.filter((filePath) => normalizeForMatch(filePath).endsWith('.sig'))
  const platforms = {}

  for (const signaturePath of signatureFiles) {
    const updaterAssetPath = signaturePath.slice(0, -4)
    if (!platformFiles.includes(updaterAssetPath) || !isSupportedUpdaterAsset(platform, updaterAssetPath)) {
      continue
    }

    const platformKey = detectPlatformKey(platform, updaterAssetPath, platformFiles)
    if (platforms[platformKey]) {
      throw new Error(`duplicate updater artifact for ${platformKey} under ${path.relative(repoRoot, platformDir)}`)
    }

    platforms[platformKey] = {
      signature: (await readFile(signaturePath, 'utf8')).trim(),
      url: path.basename(updaterAssetPath),
    }
  }

  if (Object.keys(platforms).length === 0) {
    throw new Error(
      `missing signed updater artifacts for ${platform} under ${path.relative(repoRoot, platformDir)}`,
    )
  }

  return {
    version,
    platforms,
  }
}

const artifactsDir = path.resolve(repoRoot, readArgument('--artifacts-dir') ?? 'release-artifacts')
const publishDir = path.resolve(repoRoot, readArgument('--publish-dir') ?? path.join(artifactsDir, 'publish'))
const metadataDir = path.resolve(repoRoot, readArgument('--metadata-dir') ?? path.join(artifactsDir, 'metadata'))
const outputDir = path.resolve(repoRoot, readArgument('--output-dir') ?? path.join(artifactsDir, 'release-assets'))
const version = (await readFile(path.join(metadataDir, 'VERSION'), 'utf8')).trim()
const platforms = ['macos', 'linux', 'windows']

await mkdir(outputDir, { recursive: true })

for (const platform of platforms) {
  const platformDir = path.join(publishDir, platform)
  const manifest = await buildPlatformManifest(platform, platformDir, version)
  await writeFile(
    path.join(outputDir, `${platform}-latest.json`),
    `${JSON.stringify(manifest, null, 2)}\n`,
  )
}

console.log(`Prepared updater manifest assets for ${platforms.join(', ')} in ${path.relative(repoRoot, outputDir)}.`)
