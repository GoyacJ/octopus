import { execFileSync } from 'node:child_process'
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'

import { afterEach, describe, expect, it } from 'vitest'

const repoRoot = path.resolve(__dirname, '../../..')
const nodeExecutable = process.execPath
const verifyScriptPath = path.join(repoRoot, 'scripts', 'verify-release-artifacts.mjs')
const collectScriptPath = path.join(repoRoot, 'scripts', 'collect-release-artifacts.mjs')
const collectMetadataScriptPath = path.join(repoRoot, 'scripts', 'collect-release-metadata.mjs')
const releaseNotesScriptPath = path.join(repoRoot, 'scripts', 'generate-release-notes.mjs')
const previewTagScriptPath = path.join(repoRoot, 'scripts', 'generate-preview-release-tag.mjs')
const tempDirectories: string[] = []

function createTempDir(prefix: string) {
  const directory = mkdtempSync(path.join(os.tmpdir(), prefix))
  tempDirectories.push(directory)
  return directory
}

function writeFile(filePath: string, contents: string) {
  mkdirSync(path.dirname(filePath), { recursive: true })
  writeFileSync(filePath, contents)
}

afterEach(() => {
  for (const directory of tempDirectories.splice(0)) {
    rmSync(directory, { recursive: true, force: true })
  }
})

describe('release artifact governance scripts', () => {
  it('generates formal release notes from categorized fragments and companion json artifacts', () => {
    const outputDir = createTempDir('octopus-release-notes-')
    const fragmentsDir = path.join(outputDir, 'fragments')
    const outputPath = path.join(outputDir, 'v0.1.0.md')
    const releaseNotesJsonPath = path.join(outputDir, 'release-notes.json')
    const changeLogJsonPath = path.join(outputDir, 'change-log.json')

    writeFile(path.join(fragmentsDir, 'README.md'), 'ignored')
    writeFile(path.join(fragmentsDir, 'summary-2026-04-08-initial-desktop-release.md'), 'Octopus v0.1.0 作为首个正式桌面版本，建立了可正式发版的桌面交付基线。')
    writeFile(path.join(fragmentsDir, 'feature-2026-04-08-desktop-installers.md'), '桌面用户现在可以通过 macOS 与 Windows 安装包完成正式安装。')
    writeFile(path.join(fragmentsDir, 'breaking-2026-04-08-runtime-config-layout.md'), '运行时配置采用新的作用域布局，升级时需要核对 workspace 级配置文件。')
    writeFile(path.join(fragmentsDir, 'internal-2026-04-08-ci-governance.md'), '这条 internal fragment 不应进入正式版正文。')

    execFileSync(nodeExecutable, [
      releaseNotesScriptPath,
      '--tag',
      'v0.1.0',
      '--sha',
      'deadbeefcafebabe',
      '--since-ref',
      'v0.0.9',
      '--fragments-dir',
      fragmentsDir,
      '--output',
      outputPath,
    ], {
      cwd: repoRoot,
      stdio: 'pipe',
    })

    const notes = readFileSync(outputPath, 'utf8')
    const releaseNotesJson = JSON.parse(readFileSync(releaseNotesJsonPath, 'utf8')) as {
      title: string
      channel: string
      sections: {
        overview: { items: Array<{ content: string }> }
      }
    }
    const changeLogJson = JSON.parse(readFileSync(changeLogJsonPath, 'utf8')) as {
      rangeLabel: string
    }

    expect(notes).toContain('# Octopus v0.1.0')
    expect(notes).toContain('## 版本概览')
    expect(notes).toContain('首个正式桌面版本')
    expect(notes).toContain('## 用户可感知变化')
    expect(notes).toContain('macOS 与 Windows 安装包')
    expect(notes).toContain('## 升级提示')
    expect(notes).toContain('新的作用域布局')
    expect(notes).not.toContain('internal fragment 不应进入正式版正文')
    expect(notes).toContain('变更范围：v0.0.9 -> v0.1.0')
    expect(releaseNotesJson.title).toBe('Octopus v0.1.0')
    expect(releaseNotesJson.channel).toBe('formal')
    expect(releaseNotesJson.sections.overview.items[0]?.content).toContain('首个正式桌面版本')
    expect(changeLogJson.rangeLabel).toBe('v0.0.9 -> v0.1.0')
  })

  it('fails formal release note generation when the manual summary fragment is missing', () => {
    const outputDir = createTempDir('octopus-formal-release-notes-missing-summary-')
    const fragmentsDir = path.join(outputDir, 'fragments')
    const outputPath = path.join(outputDir, 'v0.1.0.md')

    writeFile(path.join(fragmentsDir, 'feature-2026-04-08-desktop-installers.md'), '桌面用户现在可以通过 macOS 与 Windows 安装包完成正式安装。')

    expect(() =>
      execFileSync(nodeExecutable, [
        releaseNotesScriptPath,
        '--tag',
        'v0.1.0',
        '--require-manual-summary',
        'true',
        '--fragments-dir',
        fragmentsDir,
        '--output',
        outputPath,
      ], {
        cwd: repoRoot,
        encoding: 'utf8',
        stdio: 'pipe',
      }),
    ).toThrowError(/summary-\* fragment/i)
  })

  it('generates preview release notes with automatic summary, channel metadata, and companion json artifacts', () => {
    const outputDir = createTempDir('octopus-preview-release-notes-')
    const fragmentsDir = path.join(outputDir, 'fragments')
    const outputPath = path.join(outputDir, 'v0.1.0-preview.42.md')
    const releaseNotesJsonPath = path.join(outputDir, 'release-notes.json')
    const changeLogJsonPath = path.join(outputDir, 'change-log.json')

    writeFile(path.join(fragmentsDir, 'internal-2026-04-08-preview-governance.md'), '预览构建延续 main 分支自动发版链路。')

    execFileSync(nodeExecutable, [
      releaseNotesScriptPath,
      '--channel',
      'preview',
      '--tag',
      'v0.1.0-preview.42',
      '--run-number',
      '42',
      '--sha',
      'deadbeefcafebabe',
      '--fragments-dir',
      fragmentsDir,
      '--output',
      outputPath,
    ], {
      cwd: repoRoot,
      stdio: 'pipe',
    })

    const notes = readFileSync(outputPath, 'utf8')
    const releaseNotesJson = JSON.parse(readFileSync(releaseNotesJsonPath, 'utf8')) as {
      channel: string
      title: string
      appendix: { metadata: { runNumber: string, commitSha: string } }
    }
    const changeLogJson = JSON.parse(readFileSync(changeLogJsonPath, 'utf8')) as {
      releaseTag: string
    }

    expect(notes).toContain('# Octopus Preview v0.1.0-preview.42')
    expect(notes).toContain('这是来自 `main` 分支的预览构建')
    expect(notes).toContain('## 本次变更')
    expect(notes).toContain('预览构建延续 main 分支自动发版链路')
    expect(notes).toContain('## 构建元数据')
    expect(notes).toContain('Run Number：42')
    expect(notes).toContain('Commit SHA：deadbeefcafebabe')
    expect(releaseNotesJson.channel).toBe('preview')
    expect(releaseNotesJson.title).toBe('Octopus Preview v0.1.0-preview.42')
    expect(releaseNotesJson.appendix.metadata.runNumber).toBe('42')
    expect(releaseNotesJson.appendix.metadata.commitSha).toBe('deadbeefcafebabe')
    expect(changeLogJson.releaseTag).toBe('v0.1.0-preview.42')
  })

  it('generates preview release tags from version and run number', () => {
    const tag = execFileSync(nodeExecutable, [
      previewTagScriptPath,
      '--version',
      '0.1.0',
      '--run-number',
      '42',
    ], {
      cwd: repoRoot,
      encoding: 'utf8',
      stdio: 'pipe',
    }).trim()

    expect(tag).toBe('v0.1.0-preview.42')
  })

  it('collects canonical release metadata into a flat directory for publish verification', () => {
    const sourceDir = createTempDir('octopus-release-metadata-source-')
    const outputDir = createTempDir('octopus-release-metadata-output-')
    const notesPath = path.join(sourceDir, 'notes', 'v0.1.0.md')
    const releaseNotesJsonPath = path.join(sourceDir, 'notes', 'release-notes.json')
    const changeLogJsonPath = path.join(sourceDir, 'notes', 'change-log.json')

    writeFile(path.join(sourceDir, 'VERSION'), '0.1.0\n')
    writeFile(path.join(sourceDir, 'contracts', 'openapi', 'octopus.openapi.yaml'), 'openapi: 3.1.0\n')
    writeFile(path.join(sourceDir, 'packages', 'schema', 'src', 'generated.ts'), 'export {}\n')
    writeFile(notesPath, '# Octopus v0.1.0\n')
    writeFile(releaseNotesJsonPath, '{"title":"Octopus v0.1.0"}\n')
    writeFile(changeLogJsonPath, '{"rangeLabel":"v0.0.9 -> v0.1.0"}\n')

    execFileSync(nodeExecutable, [
      collectMetadataScriptPath,
      '--output-dir',
      outputDir,
      '--version-file',
      path.join(sourceDir, 'VERSION'),
      '--openapi-file',
      path.join(sourceDir, 'contracts', 'openapi', 'octopus.openapi.yaml'),
      '--schema-file',
      path.join(sourceDir, 'packages', 'schema', 'src', 'generated.ts'),
      '--notes',
      notesPath,
    ], {
      cwd: repoRoot,
      stdio: 'pipe',
    })

    expect(readFileSync(path.join(outputDir, 'VERSION'), 'utf8')).toBe('0.1.0\n')
    expect(readFileSync(path.join(outputDir, 'octopus.openapi.yaml'), 'utf8')).toBe('openapi: 3.1.0\n')
    expect(readFileSync(path.join(outputDir, 'generated.ts'), 'utf8')).toBe('export {}\n')
    expect(readFileSync(path.join(outputDir, 'v0.1.0.md'), 'utf8')).toBe('# Octopus v0.1.0\n')
    expect(readFileSync(path.join(outputDir, 'release-notes.json'), 'utf8')).toBe('{"title":"Octopus v0.1.0"}\n')
    expect(readFileSync(path.join(outputDir, 'change-log.json'), 'utf8')).toBe('{"rangeLabel":"v0.0.9 -> v0.1.0"}\n')
  })

  it('fails verification when formal desktop installers are missing', () => {
    const artifactsDir = createTempDir('octopus-release-artifacts-')
    const metadataDir = path.join(artifactsDir, 'metadata')
    const notesPath = path.join(metadataDir, 'v0.1.0.md')

    writeFile(path.join(metadataDir, 'VERSION'), '0.1.0\n')
    writeFile(path.join(metadataDir, 'octopus.openapi.yaml'), 'openapi: 3.1.0\n')
    writeFile(path.join(metadataDir, 'generated.ts'), 'export {}\n')
    writeFile(path.join(metadataDir, 'release-notes.json'), '{"title":"Octopus v0.1.0"}\n')
    writeFile(path.join(metadataDir, 'change-log.json'), '{"rangeLabel":"v0.0.9 -> v0.1.0"}\n')
    writeFile(notesPath, '# Octopus v0.1.0\n')

    expect(() =>
      execFileSync(nodeExecutable, [
        verifyScriptPath,
        '--artifacts-dir',
        artifactsDir,
        '--metadata-dir',
        metadataDir,
        '--notes',
        notesPath,
      ], {
        cwd: repoRoot,
        encoding: 'utf8',
        stdio: 'pipe',
      }),
    ).toThrowError(/missing publishable release artifacts/i)
  })

  it('fails verification when a platform is missing a required release artifact variant', () => {
    const artifactsDir = createTempDir('octopus-release-artifacts-missing-variant-')
    const metadataDir = path.join(artifactsDir, 'metadata')
    const notesPath = path.join(metadataDir, 'v0.1.0.md')
    const publishDir = path.join(artifactsDir, 'publish')

    writeFile(path.join(publishDir, 'macos', 'Octopus_0.1.0_aarch64.dmg'), 'macos arm64 bundle')
    writeFile(path.join(publishDir, 'macos', 'Octopus_0.1.0_x64.dmg'), 'macos x64 bundle')
    writeFile(path.join(publishDir, 'linux', 'Octopus_0.1.0_amd64.AppImage'), 'linux appimage')
    writeFile(path.join(publishDir, 'windows', 'Octopus_0.1.0_x64-setup.exe'), 'windows x64 setup')
    writeFile(path.join(publishDir, 'windows', 'Octopus_0.1.0_arm64-setup.exe'), 'windows arm64 setup')

    writeFile(path.join(metadataDir, 'VERSION'), '0.1.0\n')
    writeFile(path.join(metadataDir, 'octopus.openapi.yaml'), 'openapi: 3.1.0\n')
    writeFile(path.join(metadataDir, 'generated.ts'), 'export {}\n')
    writeFile(path.join(metadataDir, 'release-notes.json'), '{"title":"Octopus v0.1.0"}\n')
    writeFile(path.join(metadataDir, 'change-log.json'), '{"rangeLabel":"v0.0.9 -> v0.1.0"}\n')
    writeFile(notesPath, '# Octopus v0.1.0\n')

    expect(() =>
      execFileSync(nodeExecutable, [
        verifyScriptPath,
        '--artifacts-dir',
        artifactsDir,
        '--metadata-dir',
        metadataDir,
        '--notes',
        notesPath,
      ], {
        cwd: repoRoot,
        encoding: 'utf8',
        stdio: 'pipe',
      }),
    ).toThrowError(/missing required release artifact for linux: Debian package \(\.deb\)/i)
  })

  it('collects publishable bundles and verification writes release checksums for every required platform variant', () => {
    const sourceDir = createTempDir('octopus-release-source-')
    const artifactsDir = createTempDir('octopus-release-output-')
    const metadataDir = path.join(artifactsDir, 'metadata')
    const notesPath = path.join(metadataDir, 'v0.1.0.md')
    const publishDir = path.join(artifactsDir, 'publish')
    const checksumsPath = path.join(artifactsDir, 'SHA256SUMS.txt')

    writeFile(path.join(sourceDir, 'dmg', 'Octopus_0.1.0_aarch64.dmg'), 'macos arm64 bundle')
    writeFile(path.join(sourceDir, 'dmg', 'Octopus_0.1.0_x64.dmg'), 'macos x64 bundle')
    writeFile(path.join(sourceDir, 'macos', 'Octopus.app', 'Contents', 'Info.plist'), 'ignored app bundle')
    writeFile(path.join(sourceDir, 'appimage', 'Octopus_0.1.0_amd64.AppImage'), 'linux appimage')
    writeFile(path.join(sourceDir, 'deb', 'octopus_0.1.0_amd64.deb'), 'linux deb')
    writeFile(path.join(sourceDir, 'nsis', 'Octopus_0.1.0_x64-setup.exe'), 'windows setup')
    writeFile(path.join(sourceDir, 'nsis', 'Octopus_0.1.0_arm64-setup.exe'), 'windows arm64 setup')
    writeFile(path.join(sourceDir, 'README.txt'), 'ignore me')

    writeFile(path.join(metadataDir, 'VERSION'), '0.1.0\n')
    writeFile(path.join(metadataDir, 'octopus.openapi.yaml'), 'openapi: 3.1.0\n')
    writeFile(path.join(metadataDir, 'generated.ts'), 'export {}\n')
    writeFile(path.join(metadataDir, 'release-notes.json'), '{"title":"Octopus v0.1.0"}\n')
    writeFile(path.join(metadataDir, 'change-log.json'), '{"rangeLabel":"v0.0.9 -> v0.1.0"}\n')
    writeFile(notesPath, '# Octopus v0.1.0\n')

    execFileSync(nodeExecutable, [
      collectScriptPath,
      '--platform',
      'macos',
      '--source-dir',
      sourceDir,
      '--output-dir',
      publishDir,
    ], {
      cwd: repoRoot,
      stdio: 'pipe',
    })

    execFileSync(nodeExecutable, [
      collectScriptPath,
      '--platform',
      'linux',
      '--source-dir',
      sourceDir,
      '--output-dir',
      publishDir,
    ], {
      cwd: repoRoot,
      stdio: 'pipe',
    })

    execFileSync(nodeExecutable, [
      collectScriptPath,
      '--platform',
      'windows',
      '--source-dir',
      sourceDir,
      '--output-dir',
      publishDir,
    ], {
      cwd: repoRoot,
      stdio: 'pipe',
    })

    execFileSync(nodeExecutable, [
      verifyScriptPath,
      '--artifacts-dir',
      artifactsDir,
      '--metadata-dir',
      metadataDir,
      '--notes',
      notesPath,
      '--checksums',
      checksumsPath,
    ], {
      cwd: repoRoot,
      stdio: 'pipe',
    })

    const checksums = readFileSync(checksumsPath, 'utf8')
    expect(checksums).toContain('metadata/VERSION')
    expect(checksums).toContain('metadata/octopus.openapi.yaml')
    expect(checksums).toContain('metadata/generated.ts')
    expect(checksums).toContain('metadata/release-notes.json')
    expect(checksums).toContain('metadata/change-log.json')
    expect(checksums).toContain('publish/macos/Octopus_0.1.0_aarch64.dmg')
    expect(checksums).toContain('publish/macos/Octopus_0.1.0_x64.dmg')
    expect(checksums).toContain('publish/linux/Octopus_0.1.0_amd64.AppImage')
    expect(checksums).toContain('publish/linux/octopus_0.1.0_amd64.deb')
    expect(checksums).toContain('publish/windows/Octopus_0.1.0_x64-setup.exe')
    expect(checksums).toContain('publish/windows/Octopus_0.1.0_arm64-setup.exe')
    expect(checksums).not.toContain('Octopus.app')
  })
})
