import { spawn } from 'node:child_process'
import { existsSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = dirname(fileURLToPath(import.meta.url))
const repoRoot = join(__dirname, '..')
const executableName = process.platform === 'win32'
  ? 'octopus-desktop-backend.exe'
  : 'octopus-desktop-backend'
const backendBinary = join(repoRoot, 'target', 'debug', executableName)

if (!existsSync(backendBinary)) {
  throw new Error(`Desktop backend binary is missing at ${backendBinary}. Run "pnpm prepare:desktop-backend:dev" first.`)
}

const forwardedArgs = process.argv.slice(2)
if (forwardedArgs[0] === '--') {
  forwardedArgs.shift()
}

const child = spawn(backendBinary, forwardedArgs, {
  cwd: repoRoot,
  stdio: 'inherit',
})

child.on('exit', (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal)
    return
  }

  process.exit(code ?? 1)
})
