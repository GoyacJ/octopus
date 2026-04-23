import { rmSync } from 'node:fs'
import { spawnSync } from 'node:child_process'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const scriptDir = dirname(fileURLToPath(import.meta.url))
const projectDir = resolve(scriptDir, '..')
const workspaceDir = resolve(projectDir, '..', '..')
const nuxtBin = resolve(
  projectDir,
  'node_modules',
  '.bin',
  process.platform === 'win32' ? 'nuxt.cmd' : 'nuxt',
)

for (const target of [
  resolve(projectDir, '.nuxt'),
  resolve(projectDir, '.output'),
  resolve(projectDir, 'node_modules', '.cache', 'vite'),
  resolve(projectDir, 'node_modules', '.vite'),
  resolve(workspaceDir, 'node_modules', '.vite'),
]) {
  rmSync(target, { recursive: true, force: true })
}

const result = spawnSync(nuxtBin, ['prepare'], {
  cwd: projectDir,
  stdio: 'inherit',
})

if (result.status !== 0) {
  process.exit(result.status ?? 1)
}
