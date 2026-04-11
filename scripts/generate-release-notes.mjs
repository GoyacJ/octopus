import {
  buildReleaseNotesData,
  collectChangeLog,
  collectChangedFiles,
  collectFragments,
  releaseNotesFragmentsDir,
  renderReleaseNotesMarkdown,
  resolveChangeTargetRef,
  resolveReleaseNotesOptions,
  resolveSinceRef,
  selectFragmentsForRange,
  writeReleaseNotesArtifacts,
} from './release-notes-lib.mjs'

const options = await resolveReleaseNotesOptions(process.argv)
const sinceRef = resolveSinceRef({
  channel: options.channel,
  releaseTag: options.releaseTag,
  requestedSinceRef: options.sinceRef,
})
const changeLog = collectChangeLog({
  channel: options.channel,
  releaseTag: options.releaseTag,
  commitSha: options.commitSha,
  sinceRef,
})
const allFragments = await collectFragments(options.fragmentsDir)
const shouldFilterFragmentsByRange = options.fragmentsDir === releaseNotesFragmentsDir
const targetRef = resolveChangeTargetRef(options.commitSha, options.releaseTag)
const changedFragmentPaths = shouldFilterFragmentsByRange && sinceRef
  ? collectChangedFiles({
      sinceRef,
      targetRef,
      pathspecs: ['docs/release-notes/fragments'],
    })
  : null
const selectedFragments = selectFragmentsForRange(
  allFragments,
  changedFragmentPaths,
  shouldFilterFragmentsByRange ? sinceRef : null,
)

if (shouldFilterFragmentsByRange && sinceRef && selectedFragments.length === 0) {
  const changedFiles = collectChangedFiles({ sinceRef, targetRef })
  const changedNonReleaseFiles = changedFiles.filter((filePath) => !(
    filePath.startsWith('docs/release-notes/')
    || filePath === '.github/workflows/release.yml'
    || filePath === '.github/workflows/release-preview.yml'
    || filePath === '.github/workflows/update-manifests.yml'
    || filePath === 'docs/release-governance.md'
    || filePath === 'docs/release-notes/README.md'
  ))

  if (changedNonReleaseFiles.length > 0) {
    console.warn(
      `[release-notes] warning: no release note fragments were updated in ${changeLog.rangeLabel}, `
      + `but ${changedNonReleaseFiles.length} non-release files changed.`,
    )
  }
}

const releaseNotesData = buildReleaseNotesData(
  { ...options, sinceRef },
  selectedFragments,
  changeLog,
)
const markdown = renderReleaseNotesMarkdown(releaseNotesData)

await writeReleaseNotesArtifacts(options, releaseNotesData, changeLog, markdown)

console.log(options.outputPath)
