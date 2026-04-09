import { existsSync, readFileSync } from 'node:fs'
import path from 'node:path'

import { describe, expect, it } from 'vitest'

const repoRoot = path.resolve(__dirname, '../../..')

function readRepoFile(...segments: string[]) {
  return readFileSync(path.join(repoRoot, ...segments), 'utf8')
}

function expectDesktopMatrix(workflow: string) {
  expect(workflow).toContain('label: macos-arm64')
  expect(workflow).toContain('runs_on: macos-latest')
  expect(workflow).toContain('target: aarch64-apple-darwin')
  expect(workflow).toContain('label: macos-x64')
  expect(workflow).toContain('runs_on: macos-15-intel')
  expect(workflow).toContain('target: x86_64-apple-darwin')
  expect(workflow).toContain('label: linux-x64')
  expect(workflow).toContain('runs_on: ubuntu-24.04')
  expect(workflow).toContain('target: x86_64-unknown-linux-gnu')
  expect(workflow).toContain('bundles: appimage,deb')
  expect(workflow).toContain('label: windows-x64')
  expect(workflow).toContain('runs_on: windows-latest')
  expect(workflow).toContain('target: x86_64-pc-windows-msvc')
  expect(workflow).toContain('label: windows-arm64')
  expect(workflow).toContain('runs_on: windows-11-arm')
  expect(workflow).toContain('target: aarch64-pc-windows-msvc')
}

function expectPreviewDesktopMatrix(workflow: string) {
  expect(workflow).toContain('label: macos-arm64')
  expect(workflow).toContain('runs_on: macos-latest')
  expect(workflow).toContain('target: aarch64-apple-darwin')
  expect(workflow).not.toContain('label: macos-x64')
  expect(workflow).not.toContain('runs_on: macos-15-intel')
  expect(workflow).not.toContain('target: x86_64-apple-darwin')
  expect(workflow).toContain('label: linux-x64')
  expect(workflow).toContain('runs_on: ubuntu-24.04')
  expect(workflow).toContain('target: x86_64-unknown-linux-gnu')
  expect(workflow).toContain('bundles: appimage,deb')
  expect(workflow).toContain('label: windows-x64')
  expect(workflow).toContain('runs_on: windows-latest')
  expect(workflow).toContain('target: x86_64-pc-windows-msvc')
  expect(workflow).toContain('label: windows-arm64')
  expect(workflow).toContain('runs_on: windows-11-arm')
  expect(workflow).toContain('target: aarch64-pc-windows-msvc')
}

describe('repository governance', () => {
  it('runs CI for mainline changes while keeping desktop and website quality gates independent', () => {
    const workflow = readRepoFile('.github', 'workflows', 'ci.yml')

    expect(workflow).toContain('- main')
    expect(workflow).toContain('name: Desktop quality gates')
    expect(workflow).toContain('name: Website quality gates')
    expect(workflow).toContain('sudo apt update')
    expect(workflow).toContain('libwebkit2gtk-4.1-dev')
    expect(workflow).toContain('libayatana-appindicator3-dev')
    expect(workflow).toContain('librsvg2-dev')
    expect(workflow).toContain('pnpm check:desktop')
    expect(workflow).toContain('pnpm check:website')
    expect(workflow).toContain('pnpm check:desktop-release')
    expect(workflow).toContain('pnpm prepare:desktop-backend:sidecar')
    expect(workflow).toContain('cargo fmt --all --check')
    expect(workflow).toContain('cargo clippy --workspace --all-targets -- -D warnings')
    expect(workflow).toContain('cargo test --workspace --locked')
    expect(workflow).not.toContain('name: Desktop release build (${{ matrix.label }})')
    expect(workflow).not.toContain('pnpm tauri build --bundles nsis --target ${{ matrix.target }} --config apps/desktop/src-tauri/tauri.conf.json')
    expect(workflow).not.toContain('pnpm tauri build --bundles appimage,deb --target x86_64-unknown-linux-gnu --config apps/desktop/src-tauri/tauri.conf.json')
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
    expect(workflow).toContain('pnpm release:collect-artifacts --platform linux')
    expect(workflow).toContain('pnpm release:collect-artifacts --platform windows')
    expect(workflow).toContain('TAURI_SIGNING_PRIVATE_KEY')
    expect(workflow).toContain('TAURI_SIGNING_PRIVATE_KEY_PASSWORD')
    expect(workflow).toContain('pnpm release:verify-artifacts')
    expect(workflow).toContain('pnpm tauri build --bundles nsis --target ${{ matrix.target }} --config apps/desktop/src-tauri/tauri.conf.json')
    expect(workflow).toContain('sudo apt update')
    expect(workflow).toContain('libwebkit2gtk-4.1-dev')
    expect(workflow).toContain('libayatana-appindicator3-dev')
    expect(workflow).toContain('librsvg2-dev')
    expect(workflow).toContain('pnpm check:desktop-release')
    expect(workflow).not.toContain('pnpm check:website')
    expect(workflow).toContain('release-artifacts/publish')
    expect(workflow).toContain('release-artifacts/metadata')
    expect(workflow).toContain('release-artifacts/publish/linux/*')
    expect(workflow).toContain('path: release-artifacts/publish/macos')
    expect(workflow).toContain('path: release-artifacts/publish/linux')
    expect(workflow).toContain('path: release-artifacts/publish/windows')
    expectDesktopMatrix(workflow)
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
    expect(workflow).toContain('TAURI_SIGNING_PRIVATE_KEY')
    expect(workflow).toContain('TAURI_SIGNING_PRIVATE_KEY_PASSWORD')
    expect(workflow).toContain('pnpm release:verify-artifacts')
    expect(workflow).toContain('release_name=Octopus Preview ${PREVIEW_TAG}')
    expect(workflow).toContain('--language zh-CN')
    expect(workflow).toContain('prerelease: true')
    expect(workflow).not.toContain('export RELEASE_TAG="${GITHUB_REF_NAME}"')
    expect(workflow).not.toContain('pnpm version:check "${RELEASE_TAG}"')
    expect(workflow).toContain('pnpm check:desktop-release')
    expect(workflow).not.toContain('pnpm check:website')
    expect(workflow).toContain('target_commitish: ${{ github.sha }}')
    expect(workflow).toContain('release-artifacts/publish/linux/*')
    expect(workflow).toContain('path: release-artifacts/publish/macos')
    expect(workflow).toContain('path: release-artifacts/publish/linux')
    expect(workflow).toContain('path: release-artifacts/publish/windows')
    expect(workflow).not.toContain('Download macOS Intel bundles')
    expect(workflow).not.toContain('octopus-desktop-macos-x64-bundles')
    expectPreviewDesktopMatrix(workflow)
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
    expect(packageJson.scripts?.['release:generate-update-manifests']).toBe('node scripts/generate-update-manifests.mjs')
    expect(packageJson.scripts?.['check:desktop']).toBe('pnpm check:frontend-governance && pnpm -C apps/desktop typecheck && pnpm -C apps/desktop test')
    expect(packageJson.scripts?.['check:desktop-release']).toBe('pnpm check:desktop && pnpm check:rust && pnpm schema:check && pnpm version:check')
    expect(packageJson.scripts?.['check:rust']).toContain('pnpm prepare:desktop-backend:sidecar')
  })

  it('keeps CI focused on quality gates while release workflows own hosted bundle coverage', () => {
    const ciWorkflow = readRepoFile('.github', 'workflows', 'ci.yml')
    const releaseWorkflow = readRepoFile('.github', 'workflows', 'release.yml')
    const previewWorkflow = readRepoFile('.github', 'workflows', 'release-preview.yml')

    expect(ciWorkflow).not.toContain('windows-11-arm')
    expect(ciWorkflow).not.toContain('pnpm tauri build --bundles nsis --target ${{ matrix.target }} --config apps/desktop/src-tauri/tauri.conf.json')
    expect(ciWorkflow).not.toContain('pnpm tauri build --bundles appimage,deb --target x86_64-unknown-linux-gnu --config apps/desktop/src-tauri/tauri.conf.json')
    expect(ciWorkflow).not.toContain('macos-15-intel')
    expect(releaseWorkflow).toContain('windows-11-arm')
    expect(releaseWorkflow).toContain('pnpm tauri build --bundles nsis --target ${{ matrix.target }} --config apps/desktop/src-tauri/tauri.conf.json')
    expect(releaseWorkflow).toContain('pnpm tauri build --bundles appimage,deb --target x86_64-unknown-linux-gnu --config apps/desktop/src-tauri/tauri.conf.json')
    expect(releaseWorkflow).toContain('macos-15-intel')
    expect(previewWorkflow).toContain('windows-11-arm')
    expect(previewWorkflow).toContain('pnpm tauri build --bundles nsis --target ${{ matrix.target }} --config apps/desktop/src-tauri/tauri.conf.json')
    expect(previewWorkflow).toContain('pnpm tauri build --bundles appimage,deb --target x86_64-unknown-linux-gnu --config apps/desktop/src-tauri/tauri.conf.json')
    expect(previewWorkflow).not.toContain('macos-15-intel')
  })

  it('deploys a full GitHub Pages updater manifest site for both channels', () => {
    const workflowPath = path.join(repoRoot, '.github', 'workflows', 'update-manifests.yml')

    expect(existsSync(workflowPath)).toBe(true)

    const workflow = readFileSync(workflowPath, 'utf8')
    expect(workflow).toContain('release:')
    expect(workflow).toContain('- published')
    expect(workflow).toContain('- edited')
    expect(workflow).toContain('workflow_dispatch:')
    expect(workflow).toContain('actions/configure-pages')
    expect(workflow).toContain('actions/upload-pages-artifact')
    expect(workflow).toContain('actions/deploy-pages')
    expect(workflow).toContain('pnpm release:generate-update-manifests')
    expect(workflow).toContain('updates/formal/latest.json')
    expect(workflow).toContain('updates/preview/latest.json')
  })

  it('treats OpenAPI as the canonical shared schema source and checks generated freshness', () => {
    const packageJson = JSON.parse(readRepoFile('package.json')) as {
      scripts?: Record<string, string>
    }

    expect(existsSync(path.join(repoRoot, 'contracts', 'openapi', 'src', 'root.yaml'))).toBe(true)
    expect(existsSync(path.join(repoRoot, 'contracts', 'openapi', 'src', 'info.yaml'))).toBe(true)
    expect(existsSync(path.join(repoRoot, 'contracts', 'openapi', 'src', 'paths', 'host.yaml'))).toBe(true)
    expect(existsSync(path.join(repoRoot, 'contracts', 'openapi', 'src', 'components', 'schemas', 'runtime.yaml'))).toBe(true)
    expect(existsSync(path.join(repoRoot, 'contracts', 'openapi', 'src', 'components', 'parameters', 'common.yaml'))).toBe(true)
    expect(existsSync(path.join(repoRoot, 'contracts', 'openapi', 'src', 'components', 'responses', 'errors.yaml'))).toBe(true)
    expect(existsSync(path.join(repoRoot, 'contracts', 'openapi', 'octopus.openapi.yaml'))).toBe(true)
    expect(existsSync(path.join(repoRoot, 'packages', 'schema', 'src', 'generated.ts'))).toBe(true)
    expect(packageJson.scripts?.['openapi:bundle']).toBe('node scripts/bundle-openapi.mjs')
    expect(packageJson.scripts?.['schema:generate']).toBe('pnpm openapi:bundle && node scripts/generate-schema.mjs')
    expect(packageJson.scripts?.['schema:check:generated']).toBe('node scripts/check-schema-governance.mjs')
    expect(packageJson.scripts?.['schema:check:routes']).toBe('node scripts/check-openapi-route-parity.mjs')
    expect(packageJson.scripts?.['schema:check:adapters']).toBe('node scripts/check-openapi-adapter-parity.mjs')
    expect(packageJson.scripts?.['schema:check']).toBe('pnpm schema:check:generated && pnpm schema:check:routes && pnpm schema:check:adapters')
    expect(packageJson.scripts?.['check:frontend']).toBe('pnpm check:desktop && pnpm check:website')
    expect(packageJson.scripts?.['check:all']).toBe('pnpm check:desktop-release && pnpm check:website')
  })

  it('tracks OpenAPI parity assets and audit artifacts in the repository', () => {
    expect(existsSync(path.join(repoRoot, 'scripts', 'check-openapi-route-parity.mjs'))).toBe(true)
    expect(existsSync(path.join(repoRoot, 'scripts', 'check-openapi-adapter-parity.mjs'))).toBe(true)
    expect(existsSync(path.join(repoRoot, 'contracts', 'openapi', 'route-parity-allowlist.json'))).toBe(true)
    expect(existsSync(path.join(repoRoot, 'contracts', 'openapi', 'adapter-parity-allowlist.json'))).toBe(true)
    expect(existsSync(path.join(repoRoot, 'docs', 'openapi-audit.md'))).toBe(true)
  })

  it('documents AI-first API governance through canonical policy docs and local AGENTS files', () => {
    const rootAgents = readRepoFile('AGENTS.md')
    const docsAgentsPath = path.join(repoRoot, 'docs', 'AGENTS.md')
    const docsAgents = readRepoFile('docs', 'AGENTS.md')
    const openApiAgentsPath = path.join(repoRoot, 'contracts', 'openapi', 'AGENTS.md')
    const openApiAgents = readRepoFile('contracts', 'openapi', 'AGENTS.md')
    const apiGovernance = readRepoFile('docs', 'api-openapi-governance.md')

    expect(existsSync(path.join(repoRoot, 'docs', 'api-openapi-governance.md'))).toBe(true)
    expect(existsSync(docsAgentsPath)).toBe(true)
    expect(existsSync(openApiAgentsPath)).toBe(true)

    expect(docsAgents.trim().length).toBeGreaterThan(0)
    expect(openApiAgents.trim().length).toBeGreaterThan(0)

    expect(rootAgents).toContain('docs/api-openapi-governance.md')
    expect(rootAgents).toContain('docs/AGENTS.md')
    expect(rootAgents).toContain('contracts/openapi/AGENTS.md')

    expect(apiGovernance).toContain('contracts/openapi/src/**')
    expect(apiGovernance).toContain('contracts/openapi/octopus.openapi.yaml')
    expect(apiGovernance).toContain('packages/schema/src/generated.ts')
    expect(apiGovernance).toContain('pnpm openapi:bundle')
    expect(apiGovernance).toContain('pnpm schema:generate')
    expect(apiGovernance).toContain('pnpm schema:check')
    expect(apiGovernance).toContain('apps/desktop/src/tauri/shell.ts')
    expect(apiGovernance).toContain('apps/desktop/src/tauri/workspace-client.ts')

    expect(docsAgents).toContain('api-openapi-governance.md')
    expect(docsAgents).toContain('openapi-audit.md')
    expect(openApiAgents).toContain('src/**')
    expect(openApiAgents).toContain('octopus.openapi.yaml')
    expect(openApiAgents).toContain('generated.ts')
  })

  it('extends OpenAPI and generated transport coverage for the next workspace, catalog, and runtime clusters', () => {
    const openApiSpec = readRepoFile('contracts', 'openapi', 'octopus.openapi.yaml')
    const generatedSchema = readRepoFile('packages', 'schema', 'src', 'generated.ts')

    expect(openApiSpec).toContain('/api/v1/workspace/pet:')
    expect(openApiSpec).toContain('/api/v1/workspace/agents:')
    expect(openApiSpec).toContain('/api/v1/workspace/automations:')
    expect(openApiSpec).toContain('/api/v1/workspace/teams:')
    expect(openApiSpec).toContain('/api/v1/workspace/rbac/users:')
    expect(openApiSpec).toContain('/api/v1/workspace/catalog/models:')
    expect(openApiSpec).toContain('/api/v1/workspace/catalog/tools:')
    expect(openApiSpec).toContain('/api/v1/workspace/catalog/skills/{skillId}/files/{relativePath}:')
    expect(openApiSpec).toContain('/api/v1/projects/{projectId}/agent-links:')
    expect(openApiSpec).toContain('/api/v1/runtime/bootstrap:')
    expect(openApiSpec).toContain('/api/v1/runtime/config/validate:')
    expect(openApiSpec).toContain('/api/v1/runtime/config/configured-models/probe:')
    expect(openApiSpec).toContain('/api/v1/runtime/config/scopes/{scope}:')
    expect(openApiSpec).toContain('/api/v1/runtime/sessions:')
    expect(openApiSpec).toContain('/api/v1/runtime/sessions/{sessionId}:')
    expect(openApiSpec).toContain('/api/v1/runtime/sessions/{sessionId}/turns:')
    expect(openApiSpec).toContain('/api/v1/runtime/sessions/{sessionId}/approvals/{approvalId}:')
    expect(openApiSpec).toContain('/api/v1/runtime/sessions/{sessionId}/events:')
    expect(openApiSpec).toContain('/api/v1/projects/{projectId}/runtime-config:')
    expect(openApiSpec).toContain('/api/v1/projects/{projectId}/runtime-config/validate:')
    expect(openApiSpec).toContain('/api/v1/workspace/user-center/profile/runtime-config:')
    expect(openApiSpec).toContain('/api/v1/workspace/user-center/profile/runtime-config/validate:')

    expect(generatedSchema).toContain('"/api/v1/workspace/pet": {')
    expect(generatedSchema).toContain('"/api/v1/workspace/agents": {')
    expect(generatedSchema).toContain('"/api/v1/workspace/automations": {')
    expect(generatedSchema).toContain('"/api/v1/workspace/catalog/tools": {')
    expect(generatedSchema).toContain('"/api/v1/projects/{projectId}/agent-links": {')
    expect(generatedSchema).toContain('"/api/v1/workspace/catalog/skills/{skillId}/files/{relativePath}": {')
    expect(generatedSchema).toContain('"/api/v1/runtime/config/validate": {')
    expect(generatedSchema).toContain('"/api/v1/runtime/config/configured-models/probe": {')
    expect(generatedSchema).toContain('"/api/v1/runtime/config/scopes/{scope}": {')
    expect(generatedSchema).toContain('"/api/v1/runtime/sessions": {')
    expect(generatedSchema).toContain('"/api/v1/runtime/sessions/{sessionId}": {')
    expect(generatedSchema).toContain('"/api/v1/runtime/sessions/{sessionId}/turns": {')
    expect(generatedSchema).toContain('"/api/v1/runtime/sessions/{sessionId}/approvals/{approvalId}": {')
    expect(generatedSchema).toContain('"/api/v1/runtime/sessions/{sessionId}/events": {')
    expect(generatedSchema).toContain('"/api/v1/projects/{projectId}/runtime-config": {')
    expect(generatedSchema).toContain('"/api/v1/projects/{projectId}/runtime-config/validate": {')
    expect(generatedSchema).toContain('"/api/v1/workspace/user-center/profile/runtime-config": {')
    expect(generatedSchema).toContain('"/api/v1/workspace/user-center/profile/runtime-config/validate": {')
  })

  it('finishes OpenAPI convergence for remaining apps, audit, inbox, and knowledge routes', () => {
    const openApiSpec = readRepoFile('contracts', 'openapi', 'octopus.openapi.yaml')
    const generatedSchema = readRepoFile('packages', 'schema', 'src', 'generated.ts')
    const routeAllowlist = JSON.parse(
      readRepoFile('contracts', 'openapi', 'route-parity-allowlist.json'),
    ) as { paths?: string[] }
    const adapterAllowlist = JSON.parse(
      readRepoFile('contracts', 'openapi', 'adapter-parity-allowlist.json'),
    ) as { paths?: string[] }

    expect(openApiSpec).toContain('/api/v1/apps:')
    expect(openApiSpec).toContain('/api/v1/audit:')
    expect(openApiSpec).toContain('/api/v1/inbox:')
    expect(openApiSpec).toContain('/api/v1/knowledge:')

    expect(generatedSchema).toContain('"/api/v1/apps": {')
    expect(generatedSchema).toContain('"/api/v1/audit": {')
    expect(generatedSchema).toContain('"/api/v1/inbox": {')
    expect(generatedSchema).toContain('"/api/v1/knowledge": {')

    expect(routeAllowlist.paths ?? []).toEqual([])
    expect(adapterAllowlist.paths ?? []).toEqual([])
  })
})
