import { collectVersionMismatches } from './governance-lib.mjs'

const { mismatches, version } = await collectVersionMismatches()
const releaseTag = process.env.RELEASE_TAG ?? process.argv[2]

if (releaseTag && releaseTag !== `v${version}`) {
  mismatches.push(`release tag must match VERSION: expected v${version}, received ${releaseTag}`)
}

if (mismatches.length) {
  console.error('Version governance check failed:\n')
  for (const mismatch of mismatches) {
    console.error(`- ${mismatch}`)
  }
  process.exit(1)
}

console.log(`Version governance check passed for ${version}.`)
