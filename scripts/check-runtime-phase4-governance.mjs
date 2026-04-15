import { readdir, readFile } from 'node:fs/promises'
import path from 'node:path'

import { repoRoot } from './governance-lib.mjs'

const ignoredSegments = new Set(['node_modules', '.git', 'target', 'dist', '.turbo'])
const ignoredFiles = new Set([
  'adapter_tests.rs',
  'split_module_tests.rs',
  'repo-governance.test.ts',
  'openapi-transport.test.ts',
  'runtime-store.test.ts',
  'tauri-client-runtime.test.ts',
])

const checks = [
  {
    label: 'legacy runtime session filesystem path in production code',
    roots: ['crates/octopus-runtime-adapter/src', 'crates/octopus-infra/src'],
    patterns: ['runtime_sessions_dir', 'runtime/sessions'],
  },
  {
    label: 'legacy session persistence helpers in production code',
    roots: ['crates/octopus-runtime-adapter/src'],
    patterns: ['persist_session(', 'load_runtime_events(', '-events.json'],
  },
  {
    label: 'legacy team-runtime feature gate in production code',
    roots: ['crates/octopus-runtime-adapter/src', 'crates/runtime/src'],
    patterns: ['team_runtime_not_enabled'],
  },
  {
    label: 'legacy tool-side orchestration entrypoints in production code',
    roots: ['crates/octopus-runtime-adapter/src', 'crates/runtime/src', 'crates/tools'],
    patterns: [
      'TeamCreate',
      'TeamDelete',
      'WorkerCreate',
      'WorkerGet',
      'WorkerObserve',
      'WorkerResolveTrust',
      'WorkerAwaitReady',
      'WorkerSendPrompt',
      'WorkerRestart',
      'WorkerTerminate',
      'TaskCreate',
      'TaskGet',
      'TaskList',
      'TaskStop',
      'TaskUpdate',
      'TaskOutput',
      'CronCreate',
      'CronDelete',
      'CronList',
    ],
  },
]

async function walk(dir) {
  const entries = await readdir(dir, { withFileTypes: true })
  const files = []

  for (const entry of entries) {
    if (ignoredSegments.has(entry.name)) {
      continue
    }

    const fullPath = path.join(dir, entry.name)
    if (entry.isDirectory()) {
      files.push(...await walk(fullPath))
      continue
    }

    if (ignoredFiles.has(entry.name)) {
      continue
    }

    files.push(fullPath)
  }

  return files
}

function toRepoPath(filePath) {
  return path.relative(repoRoot, filePath).split(path.sep).join('/')
}

async function findViolations({ label, roots, patterns }) {
  const violations = []

  for (const relativeRoot of roots) {
    const absoluteRoot = path.join(repoRoot, relativeRoot)
    const files = await walk(absoluteRoot)

    for (const filePath of files) {
      const content = await readFile(filePath, 'utf8')
      for (const pattern of patterns) {
        if (content.includes(pattern)) {
          violations.push(`${label}: ${toRepoPath(filePath)} contains "${pattern}"`)
        }
      }
    }
  }

  return violations
}

async function main() {
  const violations = []

  for (const check of checks) {
    violations.push(...await findViolations(check))
  }

  if (violations.length > 0) {
    console.error('Phase 4 runtime governance check failed:')
    for (const violation of violations) {
      console.error(`- ${violation}`)
    }
    process.exitCode = 1
    return
  }

  console.log('Phase 4 runtime governance check passed.')
}

await main()
