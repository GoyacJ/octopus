import { existsSync, readFileSync } from 'node:fs'
import path from 'node:path'

import { describe, expect, it } from 'vitest'

const repoRoot = path.resolve(__dirname, '../../..')

function readRepoFile(...segments: string[]) {
  return readFileSync(path.join(repoRoot, ...segments), 'utf8')
}

describe('repository governance', () => {
  it('runs CI for mainline changes and enforces all-repo quality gates', () => {
    const workflow = readRepoFile('.github', 'workflows', 'ci.yml')

    expect(workflow).toContain('- main')
    expect(workflow).toContain('pnpm check:all')
    expect(workflow).toContain('cargo fmt --all --check')
    expect(workflow).toContain('cargo clippy --workspace --all-targets -- -D warnings')
    expect(workflow).toContain('cargo test --workspace --locked')
  })

  it('publishes releases from git tags instead of branch builds', () => {
    const workflowPath = path.join(repoRoot, '.github', 'workflows', 'release.yml')

    expect(existsSync(workflowPath)).toBe(true)

    const workflow = readFileSync(workflowPath, 'utf8')
    expect(workflow).toContain('tags:')
    expect(workflow).toContain("- 'v*'")
    expect(workflow).toContain('softprops/action-gh-release')
    expect(workflow).toContain('pnpm release:notes')
    expect(workflow).toContain('pnpm release:collect-artifacts --platform macos')
    expect(workflow).toContain('pnpm release:collect-artifacts --platform windows')
    expect(workflow).toContain('pnpm release:verify-artifacts')
    expect(workflow).toContain('release-artifacts/publish')
    expect(workflow).toContain('release-artifacts/metadata')
  })

  it('uses a single version source and validates mirrored versions', () => {
    const versionFile = path.join(repoRoot, 'VERSION')

    expect(existsSync(versionFile)).toBe(true)
    expect(readFileSync(versionFile, 'utf8').trim()).toMatch(/^\d+\.\d+\.\d+$/)

    const packageJson = JSON.parse(readRepoFile('package.json')) as {
      scripts?: Record<string, string>
    }

    expect(packageJson.scripts?.['version:sync']).toBe('node scripts/sync-version.mjs')
    expect(packageJson.scripts?.['version:check']).toBe('node scripts/check-version-governance.mjs')
    expect(packageJson.scripts?.['release:notes']).toBe('node scripts/generate-release-notes.mjs')
    expect(packageJson.scripts?.['release:collect-artifacts']).toBe('node scripts/collect-release-artifacts.mjs')
    expect(packageJson.scripts?.['release:verify-artifacts']).toBe('node scripts/verify-release-artifacts.mjs')
  })

  it('treats OpenAPI as the canonical shared schema source and checks generated freshness', () => {
    const packageJson = JSON.parse(readRepoFile('package.json')) as {
      scripts?: Record<string, string>
    }

    expect(existsSync(path.join(repoRoot, 'contracts', 'openapi', 'octopus.openapi.yaml'))).toBe(true)
    expect(existsSync(path.join(repoRoot, 'packages', 'schema', 'src', 'generated.ts'))).toBe(true)
    expect(packageJson.scripts?.['schema:generate']).toBe('node scripts/generate-schema.mjs')
    expect(packageJson.scripts?.['schema:check']).toBe('node scripts/check-schema-governance.mjs')
    expect(packageJson.scripts?.['check:all']).toBe('pnpm check:frontend && pnpm check:rust && pnpm schema:check && pnpm version:check')
  })
})
