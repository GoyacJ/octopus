import { mkdir, readdir, readFile, writeFile } from 'node:fs/promises'
import path from 'node:path'

import { repoRoot, versionFilePath } from './governance-lib.mjs'

function readArgument(flag) {
  const index = process.argv.indexOf(flag)
  return index >= 0 ? process.argv[index + 1] : undefined
}

const fragmentsDir = path.join(repoRoot, 'docs', 'release-notes', 'fragments')
const requestedOutput = readArgument('--output')
const channel = readArgument('--channel') ?? 'formal'
const version = (await readFile(versionFilePath, 'utf8')).trim()
const releaseTag = readArgument('--tag') ?? `v${version}`
const runNumber = readArgument('--run-number')
const commitSha = readArgument('--sha')
const outputPath = requestedOutput
  ? path.resolve(repoRoot, requestedOutput)
  : path.join(repoRoot, 'tmp', 'release-notes', channel === 'preview' ? `${releaseTag}.md` : `v${version}.md`)

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

const title = channel === 'preview'
  ? `# Octopus ${releaseTag}`
  : `# Octopus v${version}`

const releaseMetadata = channel === 'preview'
  ? [
      '## Preview Metadata',
      '',
      `Release channel: ${channel}`,
      `Run number: ${runNumber ?? 'unknown'}`,
      `Commit SHA: ${commitSha ?? 'unknown'}`,
      '',
    ]
  : []

const governanceChecks = channel === 'preview'
  ? [
      '- Preview release from main branch governance',
      '- Single VERSION source synchronized across package, Cargo, Tauri, and OpenAPI metadata',
      '- Canonical OpenAPI schema generated into @octopus/schema',
      '- Full-repository quality gates verified before publishing',
    ]
  : [
      '- Tag-driven release',
      '- Single VERSION source synchronized across package, Cargo, Tauri, and OpenAPI metadata',
      '- Canonical OpenAPI schema generated into @octopus/schema',
      '- Full-repository quality gates verified before publishing',
    ]

const notes = [
  title,
  '',
  `Release date: ${new Date().toISOString().slice(0, 10)}`,
  '',
  ...releaseMetadata,
  fragments.length ? fragments.join('\n\n') : '## Summary\n\nNo release note fragments were added for this release.',
  '',
  '## Governance Checks',
  '',
  ...governanceChecks,
  '',
].join('\n')

await mkdir(path.dirname(outputPath), { recursive: true })
await writeFile(outputPath, notes)

console.log(outputPath)
