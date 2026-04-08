import { createHash, randomUUID } from 'node:crypto'
import { spawn } from 'node:child_process'
import { existsSync } from 'node:fs'
import { homedir } from 'node:os'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import net from 'node:net'

const __dirname = dirname(fileURLToPath(import.meta.url))
const repoRoot = join(__dirname, '..')
const pnpmCommand = process.platform === 'win32' ? 'pnpm.cmd' : 'pnpm'
const nodeCommand = process.execPath

function createRepoStateRoot() {
  const repoHash = createHash('sha256').update(repoRoot).digest('hex').slice(0, 12)
  return join(homedir(), '.octopus', 'dev-runtime', repoHash, 'browser-host')
}

async function findAvailablePort() {
  return await new Promise((resolve, reject) => {
    const server = net.createServer()
    server.unref()
    server.on('error', reject)
    server.listen(0, '127.0.0.1', () => {
      const address = server.address()
      if (!address || typeof address === 'string') {
        server.close(() => reject(new Error('Failed to allocate loopback port')))
        return
      }

      const { port } = address
      server.close((error) => {
        if (error) {
          reject(error)
          return
        }

        resolve(port)
      })
    })
  })
}

function waitForExit(child) {
  return new Promise((resolve) => {
    child.once('exit', (code, signal) => resolve({ code, signal }))
  })
}

function spawnCommand(command, args, options = {}) {
  return spawn(command, args, {
    cwd: repoRoot,
    stdio: 'inherit',
    ...options,
  })
}

async function runCommand(command, args, options = {}) {
  const child = spawnCommand(command, args, options)
  const result = await waitForExit(child)
  if (result.signal) {
    process.kill(process.pid, result.signal)
    return
  }
  if ((result.code ?? 1) !== 0) {
    throw new Error(`Command failed: ${command} ${args.join(' ')}`)
  }
}

async function waitForHealthyBackend(baseUrl, timeoutMs = 20_000) {
  const startedAt = Date.now()
  while (Date.now() - startedAt < timeoutMs) {
    try {
      const response = await fetch(`${baseUrl}/health`)
      if (response.ok) {
        return
      }
    } catch {
      // Retry until timeout.
    }

    await new Promise((resolve) => setTimeout(resolve, 250))
  }

  throw new Error(`Timed out waiting for backend health at ${baseUrl}/health`)
}

async function main() {
  if (!existsSync(join(repoRoot, 'package.json'))) {
    throw new Error(`Repo root was not found: ${repoRoot}`)
  }

  const port = await findAvailablePort()
  const authToken = randomUUID()
  const stateRoot = createRepoStateRoot()
  const preferencesPath = join(stateRoot, 'shell-preferences.json')
  const runtimeRoot = join(stateRoot, 'runtime')
  const baseUrl = `http://127.0.0.1:${port}`

  await runCommand(pnpmCommand, ['prepare:desktop-backend:dev'])

  const backend = spawnCommand(pnpmCommand, [
    'dev:backend',
    '--',
    '--port',
    String(port),
    '--auth-token',
    authToken,
    '--app-version',
    '0.1.0',
    '--cargo-workspace',
    'true',
    '--host-platform',
    'web',
    '--host-mode',
    'local',
    '--host-shell',
    'browser',
    '--preferences-path',
    preferencesPath,
    '--runtime-root',
    runtimeRoot,
  ])

  let shuttingDown = false
  const shutdownBackend = () => {
    if (shuttingDown) {
      return
    }

    shuttingDown = true
    if (!backend.killed) {
      backend.kill('SIGTERM')
    }
  }

  process.on('SIGINT', shutdownBackend)
  process.on('SIGTERM', shutdownBackend)

  try {
    await waitForHealthyBackend(baseUrl)
  } catch (error) {
    shutdownBackend()
    await waitForExit(backend)
    throw error
  }

  const ui = spawnCommand(nodeCommand, [
    join(repoRoot, 'scripts', 'run-desktop-ui.mjs'),
    'dev',
    'browser',
  ], {
    env: {
      ...process.env,
      VITE_HOST_RUNTIME: 'browser',
      VITE_HOST_API_BASE_URL: baseUrl,
      VITE_HOST_AUTH_TOKEN: authToken,
    },
  })

  const backendExit = waitForExit(backend)
  const uiExit = waitForExit(ui)
  const winner = await Promise.race([
    backendExit.then((result) => ({ kind: 'backend', ...result })),
    uiExit.then((result) => ({ kind: 'ui', ...result })),
  ])

  if (winner.kind === 'backend') {
    if (!ui.killed) {
      ui.kill('SIGTERM')
    }
    const uiResult = await uiExit
    if (winner.signal) {
      process.kill(process.pid, winner.signal)
      return
    }
    process.exit(winner.code ?? uiResult.code ?? 1)
    return
  }

  shutdownBackend()
  await backendExit
  if (winner.signal) {
    process.kill(process.pid, winner.signal)
    return
  }
  process.exit(winner.code ?? 0)
}

await main()
