import {
  buildReleaseNotesData,
  collectChangeLog,
  collectFragments,
  renderReleaseNotesMarkdown,
  resolveReleaseNotesOptions,
  resolveSinceRef,
  writeReleaseNotesArtifacts,
} from './release-notes-lib.mjs'

const options = await resolveReleaseNotesOptions(process.argv)
const fragments = await collectFragments(options.fragmentsDir)
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
const releaseNotesData = buildReleaseNotesData({ ...options, sinceRef }, fragments, changeLog)
const markdown = renderReleaseNotesMarkdown(releaseNotesData)

await writeReleaseNotesArtifacts(options, releaseNotesData, changeLog, markdown)

console.log(options.outputPath)
