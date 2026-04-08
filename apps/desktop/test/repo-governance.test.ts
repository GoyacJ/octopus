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
    expect(workflow).toContain('sudo apt update')
    expect(workflow).toContain('libwebkit2gtk-4.1-dev')
    expect(workflow).toContain('libayatana-appindicator3-dev')
    expect(workflow).toContain('librsvg2-dev')
    expect(workflow).toContain('pnpm prepare:desktop-backend:sidecar')
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
    expect(workflow).toContain('pnpm release:collect-metadata')
    expect(workflow).toContain('--require-manual-summary true')
    expect(workflow).toContain('--language zh-CN')
    expect(workflow).toContain('pnpm release:collect-artifacts --platform macos')
    expect(workflow).toContain('pnpm release:collect-artifacts --platform windows')
    expect(workflow).toContain('pnpm release:verify-artifacts')
    expect(workflow).toContain("if: runner.os == 'Windows'")
    expect(workflow).toContain('pnpm tauri build --bundles nsis --config apps/desktop/src-tauri/tauri.conf.json')
    expect(workflow).toContain('sudo apt update')
    expect(workflow).toContain('libwebkit2gtk-4.1-dev')
    expect(workflow).toContain('libayatana-appindicator3-dev')
    expect(workflow).toContain('librsvg2-dev')
    expect(workflow).toContain('pnpm prepare:desktop-backend:sidecar')
    expect(workflow).toContain('release-artifacts/publish')
    expect(workflow).toContain('release-artifacts/metadata')
  })

  it('publishes preview releases from main and manual dispatch without formal tag gating', () => {
    const workflowPath = path.join(repoRoot, '.github', 'workflows', 'release-preview.yml')

    expect(existsSync(workflowPath)).toBe(true)

    const workflow = readFileSync(workflowPath, 'utf8')
    expect(workflow).toContain('branches:')
    expect(workflow).toContain('- main')
    expect(workflow).toContain('workflow_dispatch:')
    expect(workflow).toContain('pnpm release:notes:preview')
    expect(workflow).toContain('pnpm release:tag:preview')
    expect(workflow).toContain('pnpm release:collect-metadata')
    expect(workflow).toContain('pnpm release:verify-artifacts')
    expect(workflow).toContain('release_name=Octopus Preview ${PREVIEW_TAG}')
    expect(workflow).toContain('--language zh-CN')
    expect(workflow).toContain('prerelease: true')
    expect(workflow).not.toContain('export RELEASE_TAG="${GITHUB_REF_NAME}"')
    expect(workflow).not.toContain('pnpm version:check "${RELEASE_TAG}"')
    expect(workflow).toContain('pnpm version:check')
    expect(workflow).toContain('target_commitish: ${{ github.sha }}')
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
    expect(packageJson.scripts?.['release:notes:preview']).toBe('node scripts/generate-release-notes.mjs --channel preview')
    expect(packageJson.scripts?.['release:notes:check']).toBe('node scripts/generate-release-notes.mjs --require-manual-summary true')
    expect(packageJson.scripts?.['release:collect-artifacts']).toBe('node scripts/collect-release-artifacts.mjs')
    expect(packageJson.scripts?.['release:tag:preview']).toBe('node scripts/generate-preview-release-tag.mjs')
    expect(packageJson.scripts?.['release:verify-artifacts']).toBe('node scripts/verify-release-artifacts.mjs')
    expect(packageJson.scripts?.['check:rust']).toContain('pnpm prepare:desktop-backend:sidecar')
  })

  it('uses nsis-only Windows hosted builds to avoid WiX-only MSI coupling in CI and release', () => {
    const ciWorkflow = readRepoFile('.github', 'workflows', 'ci.yml')
    const releaseWorkflow = readRepoFile('.github', 'workflows', 'release.yml')
    const previewWorkflow = readRepoFile('.github', 'workflows', 'release-preview.yml')

    expect(ciWorkflow).toContain("if: runner.os == 'Windows'")
    expect(ciWorkflow).toContain('pnpm tauri build --bundles nsis --config apps/desktop/src-tauri/tauri.conf.json')
    expect(releaseWorkflow).toContain("if: runner.os == 'Windows'")
    expect(releaseWorkflow).toContain('pnpm tauri build --bundles nsis --config apps/desktop/src-tauri/tauri.conf.json')
    expect(previewWorkflow).toContain("if: runner.os == 'Windows'")
    expect(previewWorkflow).toContain('pnpm tauri build --bundles nsis --config apps/desktop/src-tauri/tauri.conf.json')
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
