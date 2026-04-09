import { mkdir, readdir, rename } from 'node:fs/promises'
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

const artifactsDir = path.resolve(repoRoot, readArgument('--artifacts-dir') ?? 'release-artifacts')
const publishDir = path.resolve(repoRoot, readArgument('--publish-dir') ?? path.join(artifactsDir, 'publish'))
const outputDir = path.resolve(repoRoot, readArgument('--output-dir') ?? path.join(artifactsDir, 'release-assets'))
const platforms = ['macos', 'linux', 'windows']

await mkdir(outputDir, { recursive: true })

for (const platform of platforms) {
  const platformDir = path.join(publishDir, platform)
  const latestManifestPath = (await collectFiles(platformDir))
    .find((filePath) => path.basename(filePath).toLowerCase() === 'latest.json')

  if (!latestManifestPath) {
    throw new Error(`missing updater manifest latest.json for ${platform} under ${path.relative(repoRoot, platformDir)}`)
  }

  await rename(latestManifestPath, path.join(outputDir, `${platform}-latest.json`))
}

console.log(`Prepared updater manifest assets for ${platforms.join(', ')} in ${path.relative(repoRoot, outputDir)}.`)
