import { createHash } from 'node:crypto'
import { mkdir, readFile, readdir, stat, writeFile } from 'node:fs/promises'
import path from 'node:path'

import { releasePlatformArtifactRules, repoRoot } from './governance-lib.mjs'

function readArgument(flag) {
  const index = process.argv.indexOf(flag)
  return index >= 0 ? process.argv[index + 1] : undefined
}

const artifactsDir = path.resolve(repoRoot, readArgument('--artifacts-dir') ?? 'release-artifacts')
const metadataDir = path.resolve(repoRoot, readArgument('--metadata-dir') ?? path.join(artifactsDir, 'metadata'))
const publishDir = path.resolve(repoRoot, readArgument('--publish-dir') ?? path.join(artifactsDir, 'publish'))
const notesPath = path.resolve(repoRoot, readArgument('--notes') ?? path.join('tmp', 'release-notes', 'latest.md'))
const outputPath = path.resolve(repoRoot, readArgument('--checksums') ?? path.join(artifactsDir, 'SHA256SUMS.txt'))

const errors = []

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

const requiredMetadataFiles = [
  path.join(metadataDir, 'VERSION'),
  path.join(metadataDir, 'octopus.openapi.yaml'),
  path.join(metadataDir, 'generated.ts'),
  path.join(metadataDir, 'release-notes.json'),
  path.join(metadataDir, 'change-log.json'),
  notesPath,
]

for (const requiredPath of requiredMetadataFiles) {
  try {
    await stat(requiredPath)
  } catch {
    errors.push(`missing required release asset: ${path.relative(repoRoot, requiredPath)}`)
  }
}

const publishableArtifacts = []
for (const [platform, { artifactExtensions, requiredArtifacts }] of Object.entries(releasePlatformArtifactRules)) {
  const platformDir = path.join(publishDir, platform)
  const platformFiles = (await collectFiles(platformDir))
    .filter((filePath) => artifactExtensions.some((extension) => filePath.toLowerCase().endsWith(extension)))
    .sort()

  publishableArtifacts.push(...platformFiles)

  if (platformFiles.length === 0) {
    errors.push(`missing publishable release artifacts for ${platform} under ${path.relative(repoRoot, platformDir)}`)
    continue
  }

  for (const { label, pattern } of requiredArtifacts) {
    if (!platformFiles.some((filePath) => pattern.test(path.basename(filePath)))) {
      errors.push(`missing required release artifact for ${platform}: ${label} under ${path.relative(repoRoot, platformDir)}`)
    }
  }
}

if (errors.length) {
  console.error('Release artifact verification failed:\n')
  for (const error of errors) {
    console.error(`- ${error}`)
  }
  process.exit(1)
}

const metadataFiles = (await collectFiles(metadataDir))
  .filter((filePath) => filePath !== outputPath)
  .sort()
const checksumLines = []
for (const filePath of [...metadataFiles, ...publishableArtifacts].sort()) {
  const buffer = await readFile(filePath)
  const checksum = createHash('sha256').update(buffer).digest('hex')
  checksumLines.push(`${checksum}  ${path.relative(artifactsDir, filePath)}`)
}

await mkdir(path.dirname(outputPath), { recursive: true })
await writeFile(outputPath, `${checksumLines.join('\n')}\n`)

console.log(`Verified ${publishableArtifacts.length} desktop release artifacts and ${metadataFiles.length} metadata files.`)
