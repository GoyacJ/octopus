import { tmpdir } from 'node:os'
import { join } from 'node:path'

export const PLAYWRIGHT_WORKSPACE_ROOT = join(tmpdir(), 'octopus-playwright-workspace')
