import { copyFile, mkdir, readdir } from 'node:fs/promises'
import path from 'node:path'

import { releasePlatformArtifactRules, repoRoot } from './governance-lib.mjs'

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

const platform = readArgument('--platform')
if (!platform || !releasePlatformArtifactRules[platform]) {
  const supportedPlatforms = Object.keys(releasePlatformArtifactRules).join(', ')
  throw new Error(`--platform must be one of: ${supportedPlatforms}`)
}

const sourceDir = path.resolve(repoRoot, readArgument('--source-dir') ?? path.join('target', 'release', 'bundle'))
const outputDir = path.resolve(repoRoot, readArgument('--output-dir') ?? path.join('release-artifacts', 'publish'))
const platformOutputDir = path.join(outputDir, platform)
const { artifactExtensions } = releasePlatformArtifactRules[platform]

const bundleFiles = (await collectFiles(sourceDir))
  .filter((filePath) => artifactExtensions.some((extension) => filePath.toLowerCase().endsWith(extension)))
  .sort()

if (bundleFiles.length === 0) {
  throw new Error(`No publishable ${platform} release artifacts were found under ${path.relative(repoRoot, sourceDir)}`)
}

await mkdir(platformOutputDir, { recursive: true })

for (const filePath of bundleFiles) {
  await copyFile(filePath, path.join(platformOutputDir, path.basename(filePath)))
}

console.log(`Collected ${bundleFiles.length} ${platform} release artifacts into ${path.relative(repoRoot, platformOutputDir)}.`)
