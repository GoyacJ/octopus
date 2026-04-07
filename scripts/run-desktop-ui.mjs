import { spawn } from 'node:child_process'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = dirname(fileURLToPath(import.meta.url))
const repoRoot = join(__dirname, '..')
const desktopDir = join(repoRoot, 'apps', 'desktop')
const pnpmCommand = process.platform === 'win32' ? 'pnpm.cmd' : 'pnpm'

const [, , mode, runtime] = process.argv

if (!mode || !runtime) {
  throw new Error('Usage: node scripts/run-desktop-ui.mjs <dev|build> <tauri|browser>')
}

if (mode !== 'dev' && mode !== 'build') {
  throw new Error(`Unsupported UI mode: ${mode}`)
}

if (runtime !== 'tauri' && runtime !== 'browser') {
  throw new Error(`Unsupported host runtime: ${runtime}`)
}

const env = {
  ...process.env,
  VITE_HOST_RUNTIME: runtime,
  VITE_UI_PORT: runtime === 'browser' ? '15421' : '15420',
}

function waitForExit(child) {
  return new Promise((resolve) => {
    child.once('exit', (code, signal) => resolve({ code, signal }))
  })
}

function spawnPnpm(args) {
  return spawn(pnpmCommand, args, {
    cwd: repoRoot,
    stdio: 'inherit',
    env,
    shell: process.platform === 'win32',
  })
}

async function runCommand(args) {
  const child = spawnPnpm(args)
  const result = await waitForExit(child)
  if (result.signal) {
    process.kill(process.pid, result.signal)
    return
  }
  process.exit(result.code ?? 1)
}

if (mode === 'dev') {
  await runCommand(['-C', desktopDir, 'exec', 'vite'])
} else {
  const typecheck = spawnPnpm(['-C', desktopDir, 'exec', 'vue-tsc', '--noEmit'])
  const typecheckResult = await waitForExit(typecheck)
  if (typecheckResult.signal) {
    process.kill(process.pid, typecheckResult.signal)
  } else if ((typecheckResult.code ?? 1) !== 0) {
    process.exit(typecheckResult.code ?? 1)
  } else {
    await runCommand(['-C', desktopDir, 'exec', 'vite', 'build'])
  }
}
