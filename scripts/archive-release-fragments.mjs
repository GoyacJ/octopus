import { mkdir, rename, stat } from 'node:fs/promises'
import path from 'node:path'

import {
  collectChangedFiles,
  collectFragments,
  releaseNotesFragmentsDir,
  resolveChangeTargetRef,
  resolveSinceRef,
  selectFragmentsForRange,
} from './release-notes-lib.mjs'
import { repoRoot } from './governance-lib.mjs'

function readArgument(flag) {
  const index = process.argv.indexOf(flag)
  return index >= 0 ? process.argv[index + 1] : undefined
}

async function exists(filePath) {
  try {
    await stat(filePath)
    return true
  } catch {
    return false
  }
}

const releaseTag = readArgument('--tag')
if (!releaseTag) {
  throw new Error('--tag is required')
}

const fragmentsDir = path.resolve(repoRoot, readArgument('--fragments-dir') ?? releaseNotesFragmentsDir)
const archiveRootDir = path.resolve(
  repoRoot,
  readArgument('--archive-dir') ?? path.join('docs', 'release-notes', 'archive'),
)
const archiveDir = path.join(archiveRootDir, releaseTag)
const explicitFiles = readArgument('--files')
  ?.split(',')
  .map((entry) => entry.trim())
  .filter(Boolean)
const requestedSinceRef = readArgument('--since-ref') ?? null
const targetRef = resolveChangeTargetRef(readArgument('--target-ref') ?? null, releaseTag)

let filesToArchive = explicitFiles ?? []
if (filesToArchive.length === 0) {
  const sinceRef = resolveSinceRef({
    channel: 'formal',
    releaseTag,
    requestedSinceRef,
  })
  const fragments = await collectFragments(fragmentsDir)
  const changedFragmentPaths = sinceRef
    ? collectChangedFiles({
        sinceRef,
        targetRef,
        pathspecs: ['docs/release-notes/fragments'],
      })
    : null
  filesToArchive = selectFragmentsForRange(fragments, changedFragmentPaths, sinceRef).map((fragment) => fragment.fileName)
}

if (filesToArchive.length === 0) {
  throw new Error(`no fragments selected for archive under ${path.relative(repoRoot, fragmentsDir)}`)
}

await mkdir(archiveDir, { recursive: true })

for (const fileName of filesToArchive) {
  const sourcePath = path.join(fragmentsDir, fileName)
  const destinationPath = path.join(archiveDir, fileName)

  if (!(await exists(sourcePath))) {
    throw new Error(`fragment does not exist: ${path.relative(repoRoot, sourcePath)}`)
  }

  await rename(sourcePath, destinationPath)
}

console.log(`Archived ${filesToArchive.length} release fragments into ${path.relative(repoRoot, archiveDir)}.`)
