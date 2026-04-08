import { copyFile, mkdir } from 'node:fs/promises'
import path from 'node:path'

import { repoRoot } from './governance-lib.mjs'

function readArgument(flag) {
  const index = process.argv.indexOf(flag)
  return index >= 0 ? process.argv[index + 1] : undefined
}

const outputDir = path.resolve(repoRoot, readArgument('--output-dir') ?? path.join('release-artifacts', 'metadata'))
const versionFile = path.resolve(repoRoot, readArgument('--version-file') ?? 'VERSION')
const openapiFile = path.resolve(repoRoot, readArgument('--openapi-file') ?? path.join('contracts', 'openapi', 'octopus.openapi.yaml'))
const schemaFile = path.resolve(repoRoot, readArgument('--schema-file') ?? path.join('packages', 'schema', 'src', 'generated.ts'))
const notesFile = path.resolve(repoRoot, readArgument('--notes') ?? path.join('tmp', 'release-notes', 'latest.md'))
const releaseNotesJsonFile = path.resolve(
  repoRoot,
  readArgument('--release-notes-json') ?? path.join(path.dirname(notesFile), 'release-notes.json'),
)
const changeLogJsonFile = path.resolve(
  repoRoot,
  readArgument('--change-log-json') ?? path.join(path.dirname(notesFile), 'change-log.json'),
)

const metadataFiles = [
  [versionFile, 'VERSION'],
  [openapiFile, 'octopus.openapi.yaml'],
  [schemaFile, 'generated.ts'],
  [notesFile, path.basename(notesFile)],
  [releaseNotesJsonFile, 'release-notes.json'],
  [changeLogJsonFile, 'change-log.json'],
]

await mkdir(outputDir, { recursive: true })

for (const [sourcePath, fileName] of metadataFiles) {
  await copyFile(sourcePath, path.join(outputDir, fileName))
}

console.log(`Collected ${metadataFiles.length} release metadata files into ${path.relative(repoRoot, outputDir)}.`)
