import { createHash } from 'node:crypto'
import { rmSync } from 'node:fs'
import { homedir } from 'node:os'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

import type { FullConfig } from '@playwright/test'

import { PLAYWRIGHT_WORKSPACE_ROOT } from './testPaths'

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..', '..', '..', '..')

function resolveBrowserHostStateRoot() {
  const repoHash = createHash('sha256').update(repoRoot).digest('hex').slice(0, 12)
  return join(homedir(), '.octopus', 'dev-runtime', repoHash, 'browser-host')
}

export default async function globalSetup(_config: FullConfig) {
  rmSync(resolveBrowserHostStateRoot(), { recursive: true, force: true })
  rmSync(PLAYWRIGHT_WORKSPACE_ROOT, { recursive: true, force: true })
}
