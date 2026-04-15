import { readdir, readFile } from 'node:fs/promises'
import path from 'node:path'

import { repoRoot } from './governance-lib.mjs'

const ignoredSegments = new Set(['node_modules', '.git', 'target', 'dist', '.turbo'])

const checks = [
  {
    label: 'adapter legacy execution-root symbols',
    roots: ['crates/octopus-runtime-adapter/src'],
    patterns: [
      /\bturn_submit\b/,
      /\bExecutionResponse\b/,
      /\bRuntimeModelExecutor\b/,
      /\bexecute_turn\b/,
    ],
  },
  {
    label: 'compat skill and registry entrypoints',
    roots: ['crates/tools/src', 'crates/compat-harness/src', 'crates/octopus-runtime-adapter/src'],
    patterns: [
      /"SkillDiscovery"/,
      /"SkillTool"/,
      /\bSkillDiscoveryInput\b/,
      /\bSkillToolInput\b/,
      /\brun_skill_discovery\b/,
      /\brun_skill_tool\b/,
      /\bToolRegistry\b/,
    ],
  },
  {
    label: 'legacy runtime persistence and session recovery symbols',
    roots: [
      'crates/octopus-runtime-adapter/src',
      'crates/runtime/src',
      'crates/rusty-claude-cli/src',
      'crates/rusty-claude-cli/tests',
    ],
    patterns: [
      /\bbuild_legacy_configured_models\b/,
      /\blegacy_configured_models\b/,
      /\bmigrate_legacy_workspace_config_if_needed\b/,
      /\bloads_legacy_session_json_object\b/,
      /\brejects_legacy_session_json_without_messages\b/,
      /legacy session should load/,
      /\bresolve_legacy_json\b/,
    ],
  },
  {
    label: 'runtime-normative phase 8 wording regressions',
    files: [
      'crates/runtime/src/lib.rs',
      'docs/runtime_config_api.md',
      'apps/desktop/test/tauri-client-host.test.ts',
      'apps/desktop/test/tauri-client-runtime.test.ts',
      'apps/desktop/test/openapi-transport.test.ts',
      'apps/desktop/test/runtime-store.test.ts',
    ],
    patterns: [
      /one-shot turns/,
      /legacy runtime helper/,
      /user-center\/profile\/runtime-config/,
      /\bmigrate_legacy_workspace_config_if_needed\b/,
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

    files.push(fullPath)
  }

  return files
}

function toRepoPath(filePath) {
  return path.relative(repoRoot, filePath).split(path.sep).join('/')
}

async function collectFiles({ roots = [], files = [] }) {
  const collected = []

  for (const relativeRoot of roots) {
    collected.push(...await walk(path.join(repoRoot, relativeRoot)))
  }

  for (const relativeFile of files) {
    collected.push(path.join(repoRoot, relativeFile))
  }

  return collected
}

async function findViolations({ label, roots, files, patterns }) {
  const violations = []

  for (const filePath of await collectFiles({ roots, files })) {
    const content = await readFile(filePath, 'utf8')
    for (const pattern of patterns) {
      if (pattern.test(content)) {
        violations.push(`${label}: ${toRepoPath(filePath)} contains "${pattern.source}"`)
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
    console.error('Phase 8 runtime governance check failed:')
    for (const violation of violations) {
      console.error(`- ${violation}`)
    }
    process.exitCode = 1
    return
  }

  console.log('Phase 8 runtime governance check passed.')
}

await main()
