import { spawn } from 'node:child_process'
import { createHash } from 'node:crypto'
import { rmSync } from 'node:fs'
import { homedir, tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const pnpmCommand = process.platform === 'win32' ? 'pnpm.cmd' : 'pnpm'
const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..', '..', '..', '..')
const browserHostStateRoot = join(
  homedir(),
  '.octopus',
  'dev-runtime',
  createHash('sha256').update(repoRoot).digest('hex').slice(0, 12),
  'browser-host',
)
const playwrightWorkspaceRoot = join(tmpdir(), 'octopus-playwright-workspace')

function waitForExit(child) {
  return new Promise((resolve) => {
    child.once('exit', (code, signal) => resolve({ code, signal }))
  })
}

rmSync(browserHostStateRoot, { recursive: true, force: true })
rmSync(playwrightWorkspaceRoot, { recursive: true, force: true })

const child = spawn(pnpmCommand, ['dev:web'], {
  cwd: repoRoot,
  env: {
    ...process.env,
    CI: '1',
  },
  stdio: 'inherit',
  shell: process.platform === 'win32',
})

let shuttingDown = false

const forwardSignal = (signal) => {
  if (shuttingDown) {
    return
  }

  shuttingDown = true
  if (!child.killed) {
    child.kill(signal)
  }
}

process.on('SIGINT', () => forwardSignal('SIGINT'))
process.on('SIGTERM', () => forwardSignal('SIGTERM'))

const result = await waitForExit(child)
if (result.signal) {
  process.kill(process.pid, result.signal)
} else {
  process.exit(result.code ?? 1)
}
