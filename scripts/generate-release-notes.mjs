import { mkdir, readdir, readFile, writeFile } from 'node:fs/promises'
import path from 'node:path'

import { repoRoot, versionFilePath } from './governance-lib.mjs'

const fragmentsDir = path.join(repoRoot, 'docs', 'release-notes', 'fragments')
const outputArgIndex = process.argv.indexOf('--output')
const requestedOutput = outputArgIndex >= 0 ? process.argv[outputArgIndex + 1] : undefined
const version = (await readFile(versionFilePath, 'utf8')).trim()
const outputPath = requestedOutput
  ? path.resolve(repoRoot, requestedOutput)
  : path.join(repoRoot, 'tmp', 'release-notes', `v${version}.md`)

const fragmentFiles = (await readdir(fragmentsDir).catch(() => []))
  .filter((entry) => entry.endsWith('.md'))
  .filter((entry) => entry.toLowerCase() !== 'readme.md')
  .sort()

const fragments = []
for (const fileName of fragmentFiles) {
  const content = (await readFile(path.join(fragmentsDir, fileName), 'utf8')).trim()
  if (content) {
    fragments.push(`## ${fileName.replace(/\.md$/, '')}\n\n${content}`)
  }
}

const notes = [
  `# Octopus v${version}`,
  '',
  `Release date: ${new Date().toISOString().slice(0, 10)}`,
  '',
  fragments.length ? fragments.join('\n\n') : '## Summary\n\nNo release note fragments were added for this release.',
  '',
  '## Governance Checks',
  '',
  '- Tag-driven release',
  '- Single VERSION source synchronized across package, Cargo, Tauri, and OpenAPI metadata',
  '- Canonical OpenAPI schema generated into @octopus/schema',
  '- Full-repository quality gates verified before publishing',
  '',
].join('\n')

await mkdir(path.dirname(outputPath), { recursive: true })
await writeFile(outputPath, notes)

console.log(outputPath)
