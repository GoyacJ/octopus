import { spawn } from 'node:child_process'
import { copyFile, mkdir } from 'node:fs/promises'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = dirname(fileURLToPath(import.meta.url))
const repoRoot = join(__dirname, '..')

function resolveTargetTriple() {
  if (process.platform === 'win32' && process.arch === 'x64') {
    return { triple: 'x86_64-pc-windows-msvc', ext: '.exe' }
  }
  if (process.platform === 'linux' && process.arch === 'x64') {
    return { triple: 'x86_64-unknown-linux-gnu', ext: '' }
  }
  if (process.platform === 'linux' && process.arch === 'arm64') {
    return { triple: 'aarch64-unknown-linux-gnu', ext: '' }
  }
  if (process.platform === 'darwin' && process.arch === 'x64') {
    return { triple: 'x86_64-apple-darwin', ext: '' }
  }
  if (process.platform === 'darwin' && process.arch === 'arm64') {
    return { triple: 'aarch64-apple-darwin', ext: '' }
  }

  throw new Error(`Unsupported host for desktop backend packaging: ${process.platform}/${process.arch}`)
}

function waitForExit(child) {
  return new Promise((resolve) => {
    child.once('exit', (code, signal) => resolve({ code, signal }))
  })
}

async function stopStaleWindowsDevProcesses() {
  if (process.platform !== 'win32') {
    return
  }

  const repoRootForPs = repoRoot.replace(/'/g, "''")
  const script = `
$repoRoot = '${repoRootForPs}'
$targets = @(
  'octopus-desktop-backend.exe',
  'octopus-desktop-shell.exe'
)
$processes = Get-CimInstance Win32_Process |
  Where-Object {
    $targets -contains $_.Name -and
    $_.ExecutablePath -and
    $_.ExecutablePath.StartsWith($repoRoot, [System.StringComparison]::OrdinalIgnoreCase)
  }

foreach ($process in $processes) {
  Stop-Process -Id $process.ProcessId -Force -ErrorAction SilentlyContinue
}
`

  const child = spawn('powershell.exe', ['-NoProfile', '-Command', script], {
    cwd: repoRoot,
    stdio: 'inherit',
  })

  const result = await waitForExit(child)
  if (result.signal) {
    process.kill(process.pid, result.signal)
    return
  }
  if ((result.code ?? 1) !== 0) {
    process.exit(result.code ?? 1)
  }
}

async function runCargoBuild() {
  const cargoCommand = process.platform === 'win32' ? 'cargo.exe' : 'cargo'
  const child = spawn(cargoCommand, ['build', '-p', 'octopus-desktop-backend'], {
    cwd: repoRoot,
    stdio: 'inherit',
  })

  const result = await waitForExit(child)
  if (result.signal) {
    process.kill(process.pid, result.signal)
    return
  }
  if ((result.code ?? 1) !== 0) {
    process.exit(result.code ?? 1)
  }
}

async function main() {
  const { triple, ext } = resolveTargetTriple()
  await stopStaleWindowsDevProcesses()
  await runCargoBuild()

  const source = join(repoRoot, 'target', 'debug', `octopus-desktop-backend${ext}`)
  const targetDir = join(repoRoot, 'apps', 'desktop', 'src-tauri', 'bin')
  const target = join(targetDir, `octopus-desktop-backend-${triple}${ext}`)

  await mkdir(targetDir, { recursive: true })
  await copyFile(source, target)
  console.log(`Prepared desktop backend binary: ${target}`)
}

await main()
