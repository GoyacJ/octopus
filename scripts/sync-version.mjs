import { syncMirroredVersions } from './governance-lib.mjs'

const version = await syncMirroredVersions()
console.log(`Synchronized mirrored versions to ${version}.`)
