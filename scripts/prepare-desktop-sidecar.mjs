import { cpSync, existsSync, mkdirSync } from 'node:fs'
import { join } from 'node:path'
import { execFileSync } from 'node:child_process'

const repoRoot = process.cwd()
const rustcVersion = execFileSync('rustc', ['-vV'], {
  cwd: repoRoot,
  encoding: 'utf8',
})
const hostLine = rustcVersion
  .split('\n')
  .find((line) => line.startsWith('host: '))

if (!hostLine) {
  throw new Error('Failed to resolve the Rust host target triple')
}

const targetTriple = hostLine.replace('host: ', '').trim()
const executableName = process.platform === 'win32'
  ? 'octopus-desktop.exe'
  : 'octopus-desktop'
const sidecarName = process.platform === 'win32'
  ? `octopus-desktop-${targetTriple}.exe`
  : `octopus-desktop-${targetTriple}`

execFileSync('cargo', ['build', '-p', 'octopus-desktop', '--release', '--target', targetTriple], {
  cwd: repoRoot,
  stdio: 'inherit',
})

const sourceBinary = join(repoRoot, 'target', targetTriple, 'release', executableName)
if (!existsSync(sourceBinary)) {
  throw new Error(`Built desktop sidecar binary was not found at ${sourceBinary}`)
}

const outputDir = join(repoRoot, 'apps', 'desktop', 'src-tauri', 'bin')
mkdirSync(outputDir, { recursive: true })
cpSync(sourceBinary, join(outputDir, sidecarName))
