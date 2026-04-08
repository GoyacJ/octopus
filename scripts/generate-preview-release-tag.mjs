import { readVersion } from './governance-lib.mjs'

function readArgument(flag) {
  const index = process.argv.indexOf(flag)
  return index >= 0 ? process.argv[index + 1] : undefined
}

const version = readArgument('--version') ?? await readVersion()
const runNumber = readArgument('--run-number') ?? process.env.GITHUB_RUN_NUMBER

if (!/^\d+\.\d+\.\d+$/.test(version)) {
  throw new Error(`--version must be a semantic version, received ${version ?? '<missing>'}`)
}

if (!runNumber || !/^\d+$/.test(runNumber)) {
  throw new Error(`--run-number must be a numeric identifier, received ${runNumber ?? '<missing>'}`)
}

console.log(`v${version}-preview.${runNumber}`)
