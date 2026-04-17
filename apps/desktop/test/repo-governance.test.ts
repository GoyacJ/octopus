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
    expect(workflow).toContain('node scripts/prepare-release-assets.mjs --artifacts-dir release-artifacts --output-dir release-artifacts/release-assets')
    expect(workflow).toContain('release-artifacts/publish/linux/**/*')
    expect(workflow).toContain('path: release-artifacts/publish/macos/octopus-desktop-macos-arm64-bundles')
    expect(workflow).toContain('path: release-artifacts/publish/macos/octopus-desktop-macos-x64-bundles')
    expect(workflow).toContain('path: release-artifacts/publish/linux/octopus-desktop-linux-x64-bundles')
    expect(workflow).toContain('path: release-artifacts/publish/windows/octopus-desktop-windows-x64-bundles')
    expect(workflow).toContain('path: release-artifacts/publish/windows/octopus-desktop-windows-arm64-bundles')
    expectDesktopMatrix(workflow)
  })

  it('publishes preview releases from manual dispatch without formal tag gating', () => {
    const workflowPath = path.join(repoRoot, '.github', 'workflows', 'release-preview.yml')

    expect(existsSync(workflowPath)).toBe(true)

    const workflow = readFileSync(workflowPath, 'utf8')
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
    expect(workflow).not.toContain('push:')
    expect(workflow).not.toContain('branches:')
    expect(workflow).toContain('pnpm check:desktop-release')
    expect(workflow).not.toContain('pnpm check:website')
    expect(workflow).toContain('target_commitish: ${{ github.sha }}')
    expect(workflow).toContain('pnpm release:verify-artifacts --channel preview')
    expect(workflow).toContain('node scripts/prepare-release-assets.mjs --artifacts-dir release-artifacts --output-dir release-artifacts/release-assets')
    expect(workflow).toContain('release-artifacts/publish/linux/**/*')
    expect(workflow).toContain('path: release-artifacts/publish/macos/octopus-desktop-macos-arm64-bundles')
    expect(workflow).toContain('path: release-artifacts/publish/linux/octopus-desktop-linux-x64-bundles')
    expect(workflow).toContain('path: release-artifacts/publish/windows/octopus-desktop-windows-x64-bundles')
    expect(workflow).toContain('path: release-artifacts/publish/windows/octopus-desktop-windows-arm64-bundles')
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
    expect(packageJson.scripts?.['release:archive-fragments']).toBe('node scripts/archive-release-fragments.mjs')
    expect(packageJson.scripts?.['release:tag:preview']).toBe('node scripts/generate-preview-release-tag.mjs')
    expect(packageJson.scripts?.['release:verify-artifacts']).toBe('node scripts/verify-release-artifacts.mjs')
    expect(packageJson.scripts?.['release:generate-update-manifests']).toBe('node scripts/generate-update-manifests.mjs')
    expect(packageJson.scripts?.['check:desktop']).toBe('pnpm check:frontend-governance && pnpm -C apps/desktop typecheck && pnpm -C apps/desktop test')
    expect(packageJson.scripts?.['check:desktop-release']).toBe(
      'pnpm check:desktop && pnpm check:rust && pnpm schema:check && pnpm check:runtime-phase4 && pnpm check:runtime-phase8 && pnpm version:check',
    )
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

  it('enforces a dedicated Phase 4 runtime closure governance gate', () => {
    const packageJson = JSON.parse(readRepoFile('package.json')) as {
      scripts?: Record<string, string>
    }

    expect(existsSync(path.join(repoRoot, 'scripts', 'check-runtime-phase4-governance.mjs'))).toBe(true)
    expect(packageJson.scripts?.['check:runtime-phase4']).toBe(
      'node scripts/check-runtime-phase4-governance.mjs',
    )
    expect(packageJson.scripts?.['check:desktop-release']).toContain('pnpm check:runtime-phase4')
  })

  it('enforces a dedicated Phase 8 legacy deletion governance gate', () => {
    const packageJson = JSON.parse(readRepoFile('package.json')) as {
      scripts?: Record<string, string>
    }

    expect(existsSync(path.join(repoRoot, 'scripts', 'check-runtime-phase8-governance.mjs'))).toBe(true)
    expect(packageJson.scripts?.['check:runtime-phase8']).toBe(
      'node scripts/check-runtime-phase8-governance.mjs',
    )
    expect(packageJson.scripts?.['check:desktop-release']).toContain('pnpm check:runtime-phase8')
  })

  it('keeps the Phase 8 plan aligned with the compat-only legacy deletion gate', () => {
    const phaseEightPlan = readRepoFile('docs', 'plans', 'runtime', 'phase-8-legacy-deletion.md')
    const phaseEightScript = readRepoFile('scripts', 'check-runtime-phase8-governance.mjs')

    expect(phaseEightScript).toContain('/\\bturn_submit\\b/')
    expect(phaseEightScript).toContain('/\\bRuntimeModelExecutor\\b/')
    expect(phaseEightScript).toContain('/\\bSkillDiscoveryInput\\b/')
    expect(phaseEightScript).toContain('/\\bSkillToolInput\\b/')
    expect(phaseEightScript).not.toContain('/\\bsubmit_turn\\b/')

    expect(phaseEightPlan).toContain(
      'rg -n "turn_submit|ExecutionResponse|RuntimeModelExecutor|execute_turn\\\\(" crates/octopus-runtime-adapter/src',
    )
    expect(phaseEightPlan).toContain(
      'rg -n "\\"SkillDiscovery\\"|\\"SkillTool\\"|SkillDiscoveryInput|SkillToolInput|run_skill_discovery|run_skill_tool" crates/tools/src crates/octopus-runtime-adapter/src',
    )
    expect(phaseEightPlan).not.toContain(
      'rg -n "turn_submit|submit_turn\\\\(" crates/octopus-runtime-adapter/src crates/runtime/src',
    )
    expect(phaseEightPlan).not.toContain(
      'rg -n "SkillDiscovery|SkillTool" crates/tools/src crates/octopus-runtime-adapter/src',
    )
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
    expect(openApiSpec).toContain('/api/v1/workspace/teams:')
    expect(openApiSpec).not.toContain('/api/v1/workspace/automations:')
    expect(openApiSpec).not.toContain('/api/v1/auth/login:')
    expect(openApiSpec).not.toContain('/api/v1/auth/logout:')
    expect(openApiSpec).not.toContain('/api/v1/auth/register-owner:')
    expect(openApiSpec).not.toContain('/api/v1/auth/session:')
    expect(openApiSpec).not.toContain('/api/v1/workspace/rbac/users:')
    expect(openApiSpec).not.toContain('/api/v1/workspace/rbac/roles:')
    expect(openApiSpec).not.toContain('/api/v1/workspace/rbac/permissions:')
    expect(openApiSpec).not.toContain('/api/v1/workspace/rbac/menus:')
    expect(openApiSpec).not.toContain('/api/v1/workspace/permission-center/overview:')
    expect(openApiSpec).toContain('/api/v1/workspace/catalog/models:')
    expect(openApiSpec).toContain('/api/v1/workspace/catalog/tools:')
    expect(openApiSpec).toContain('/api/v1/workspace/catalog/skills/{skillId}/files/{relativePath}:')
    expect(openApiSpec).toContain('/api/v1/workspace/agents/import-preview:')
    expect(openApiSpec).toContain('/api/v1/workspace/agents/import:')
    expect(openApiSpec).toContain('/api/v1/workspace/agents/export:')
    expect(openApiSpec).toContain('/api/v1/projects/{projectId}/agent-links:')
    expect(openApiSpec).toContain('/api/v1/projects/{projectId}/agents/import-preview:')
    expect(openApiSpec).toContain('/api/v1/projects/{projectId}/agents/import:')
    expect(openApiSpec).toContain('/api/v1/projects/{projectId}/agents/export:')
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
    expect(openApiSpec).toContain('/api/v1/workspace/personal-center/profile/runtime-config:')
    expect(openApiSpec).toContain('/api/v1/workspace/personal-center/profile/runtime-config/validate:')

    expect(generatedSchema).toContain('"/api/v1/workspace/pet": {')
    expect(generatedSchema).toContain('"/api/v1/workspace/agents": {')
    expect(generatedSchema).toContain('"/api/v1/workspace/catalog/tools": {')
    expect(generatedSchema).not.toContain('"/api/v1/workspace/automations": {')
    expect(generatedSchema).toContain('"/api/v1/workspace/agents/import-preview": {')
    expect(generatedSchema).toContain('"/api/v1/workspace/agents/import": {')
    expect(generatedSchema).toContain('"/api/v1/workspace/agents/export": {')
    expect(generatedSchema).toContain('"/api/v1/projects/{projectId}/agent-links": {')
    expect(generatedSchema).toContain('"/api/v1/projects/{projectId}/agents/import-preview": {')
    expect(generatedSchema).toContain('"/api/v1/projects/{projectId}/agents/import": {')
    expect(generatedSchema).toContain('"/api/v1/projects/{projectId}/agents/export": {')
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
    expect(generatedSchema).not.toContain('"/api/v1/auth/login": {')
    expect(generatedSchema).not.toContain('"/api/v1/auth/logout": {')
    expect(generatedSchema).not.toContain('"/api/v1/auth/register-owner": {')
    expect(generatedSchema).not.toContain('"/api/v1/auth/session": {')
    expect(generatedSchema).not.toContain('"/api/v1/workspace/rbac/users": {')
    expect(generatedSchema).not.toContain('"/api/v1/workspace/rbac/roles": {')
    expect(generatedSchema).not.toContain('"/api/v1/workspace/rbac/permissions": {')
    expect(generatedSchema).not.toContain('"/api/v1/workspace/rbac/menus": {')
    expect(generatedSchema).not.toContain('"/api/v1/workspace/permission-center/overview": {')
    expect(generatedSchema).toContain('"/api/v1/workspace/personal-center/profile/runtime-config": {')
    expect(generatedSchema).toContain('"/api/v1/workspace/personal-center/profile/runtime-config/validate": {')
    expect(generatedSchema).not.toContain('PermissionCenterOverviewSnapshot')
    expect(generatedSchema).not.toContain('PermissionCenterAlertRecord')
    expect(generatedSchema).not.toContain('PermissionRecord')
    expect(generatedSchema).not.toContain('MenuRecord')
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
    expect(openApiSpec).toContain('/api/v1/access/audit:')
    expect(openApiSpec).not.toContain('/api/v1/audit:')
    expect(openApiSpec).toContain('/api/v1/inbox:')
    expect(openApiSpec).toContain('/api/v1/knowledge:')

    expect(generatedSchema).toContain('"/api/v1/apps": {')
    expect(generatedSchema).toContain('"/api/v1/access/audit": {')
    expect(generatedSchema).not.toContain('"/api/v1/audit": {')
    expect(generatedSchema).toContain('"/api/v1/inbox": {')
    expect(generatedSchema).toContain('"/api/v1/knowledge": {')

    expect(routeAllowlist.paths ?? []).toEqual([])
    expect(adapterAllowlist.paths ?? []).toEqual([])
  })

  it('defines personal pet and knowledge transport contracts with explicit owner semantics', () => {
    const openApiSpec = readRepoFile('contracts', 'openapi', 'octopus.openapi.yaml')
    const generatedSchema = readRepoFile('packages', 'schema', 'src', 'generated.ts')

    expect(openApiSpec).toContain('/api/v1/workspace/pet/dashboard:')
    expect(openApiSpec).toContain('operationId: getCurrentUserPetHomeSnapshot')
    expect(openApiSpec).toContain('operationId: getCurrentUserProjectPetSnapshot')
    expect(openApiSpec).toContain('operationId: getCurrentUserPetDashboardSummary')
    expect(openApiSpec).toContain('PetDashboardSummary:')
    expect(openApiSpec).toContain('PetContextScope:')
    expect(openApiSpec).toContain('KnowledgePlaneScope:')
    expect(openApiSpec).toContain('KnowledgeVisibilityMode:')
    expect(openApiSpec).toMatch(/KnowledgeRecord:\n[\s\S]*?ownerUserId:\n\s+type: string/)
    expect(openApiSpec).toMatch(/KnowledgeRecord:\n[\s\S]*?scope:\n\s+\$ref: "#\/components\/schemas\/KnowledgePlaneScope"/)
    expect(openApiSpec).toMatch(/KnowledgeRecord:\n[\s\S]*?visibility:\n\s+\$ref: "#\/components\/schemas\/KnowledgeVisibilityMode"/)
    expect(openApiSpec).toMatch(/PetConversationBinding:\n[\s\S]*?contextScope:\n\s+\$ref: "#\/components\/schemas\/PetContextScope"/)
    expect(openApiSpec).toMatch(/PetConversationBinding:\n[\s\S]*?ownerUserId:\n\s+type: string/)

    expect(generatedSchema).toContain('"/api/v1/workspace/pet/dashboard": {')
    expect(generatedSchema).toContain('get: { operationId: "getCurrentUserPetHomeSnapshot"; response: PetWorkspaceSnapshot; error: ApiErrorEnvelope }')
    expect(generatedSchema).toContain('get: { operationId: "getCurrentUserProjectPetSnapshot"; response: PetWorkspaceSnapshot; error: ApiErrorEnvelope }')
    expect(generatedSchema).toContain('get: { operationId: "getCurrentUserPetDashboardSummary"; response: PetDashboardSummary; error: ApiErrorEnvelope }')
    expect(generatedSchema).toContain('export type PetContextScope = "home" | "project"')
    expect(generatedSchema).toContain('export type KnowledgePlaneScope = "personal" | "project" | "workspace"')
    expect(generatedSchema).toContain('export type KnowledgeVisibilityMode = "private" | "public"')
    expect(generatedSchema).toMatch(/export interface KnowledgeRecord \{[\s\S]*?ownerUserId\?: string[\s\S]*?scope\?: KnowledgePlaneScope[\s\S]*?visibility\?: KnowledgeVisibilityMode[\s\S]*?\}/)
    expect(generatedSchema).toMatch(/export interface PetConversationBinding \{[\s\S]*?contextScope: PetContextScope[\s\S]*?ownerUserId: string[\s\S]*?\}/)
    expect(generatedSchema).toContain('export interface PetDashboardSummary {')
  })

  it('ships the enterprise access-control hard cut without migration tooling or migrate-before-startup guidance', () => {
    const infraIndex = readRepoFile('crates', 'octopus-infra', 'src', 'lib.rs')
    const infraState = readRepoFile('crates', 'octopus-infra', 'src', 'infra_state.rs')
    const migrationModulePath = path.join(
      repoRoot,
      'crates',
      'octopus-infra',
      'src',
      'legacy_access_control_migration.rs',
    )
    const migrationBinaryPath = path.join(
      repoRoot,
      'crates',
      'octopus-infra',
      'src',
      'bin',
      'access_control_hard_cut_migrate.rs',
    )

    expect(existsSync(migrationModulePath)).toBe(false)
    expect(existsSync(migrationBinaryPath)).toBe(false)
    expect(infraIndex).not.toContain('migrate_legacy_access_control')
    expect(infraIndex).not.toContain('LegacyAccessControlMigrationReport')
    expect(infraState).not.toContain('legacy access control tables with data detected')
    expect(infraState).not.toContain('run the offline access-control migrate command')
  })
})
