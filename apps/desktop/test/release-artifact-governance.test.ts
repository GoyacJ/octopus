import { execFile, execFileSync } from 'node:child_process'
import { createServer } from 'node:http'
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { promisify } from 'node:util'

import { afterEach, describe, expect, it } from 'vitest'

const repoRoot = path.resolve(__dirname, '../../..')
const nodeExecutable = process.execPath
const verifyScriptPath = path.join(repoRoot, 'scripts', 'verify-release-artifacts.mjs')
const collectScriptPath = path.join(repoRoot, 'scripts', 'collect-release-artifacts.mjs')
const collectMetadataScriptPath = path.join(repoRoot, 'scripts', 'collect-release-metadata.mjs')
const generateUpdateManifestsScriptPath = path.join(repoRoot, 'scripts', 'generate-update-manifests.mjs')
const releaseNotesScriptPath = path.join(repoRoot, 'scripts', 'generate-release-notes.mjs')
const previewTagScriptPath = path.join(repoRoot, 'scripts', 'generate-preview-release-tag.mjs')
const tempDirectories: string[] = []
const execFileAsync = promisify(execFile)

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
    writeFile(path.join(sourceDir, 'updater', 'macos', 'Octopus.app.tar.gz'), 'macos updater archive')
    writeFile(path.join(sourceDir, 'updater', 'macos', 'Octopus.app.tar.gz.sig'), 'macos updater signature')
    writeFile(path.join(sourceDir, 'updater', 'macos', 'latest.json'), '{"version":"0.1.0"}\n')
    writeFile(path.join(sourceDir, 'macos', 'Octopus.app', 'Contents', 'Info.plist'), 'ignored app bundle')
    writeFile(path.join(sourceDir, 'appimage', 'Octopus_0.1.0_amd64.AppImage'), 'linux appimage')
    writeFile(path.join(sourceDir, 'deb', 'octopus_0.1.0_amd64.deb'), 'linux deb')
    writeFile(path.join(sourceDir, 'updater', 'linux', 'Octopus.AppImage.tar.gz'), 'linux updater archive')
    writeFile(path.join(sourceDir, 'updater', 'linux', 'Octopus.AppImage.tar.gz.sig'), 'linux updater signature')
    writeFile(path.join(sourceDir, 'updater', 'linux', 'latest.json'), '{"version":"0.1.0"}\n')
    writeFile(path.join(sourceDir, 'nsis', 'Octopus_0.1.0_x64-setup.exe'), 'windows setup')
    writeFile(path.join(sourceDir, 'nsis', 'Octopus_0.1.0_arm64-setup.exe'), 'windows arm64 setup')
    writeFile(path.join(sourceDir, 'updater', 'windows', 'Octopus.nsis.zip'), 'windows updater archive')
    writeFile(path.join(sourceDir, 'updater', 'windows', 'Octopus.nsis.zip.sig'), 'windows updater signature')
    writeFile(path.join(sourceDir, 'updater', 'windows', 'latest.json'), '{"version":"0.1.0"}\n')
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
    expect(checksums).toContain('publish/macos/Octopus.app.tar.gz')
    expect(checksums).toContain('publish/macos/Octopus.app.tar.gz.sig')
    expect(checksums).toContain('publish/macos/latest.json')
    expect(checksums).toContain('publish/linux/Octopus_0.1.0_amd64.AppImage')
    expect(checksums).toContain('publish/linux/octopus_0.1.0_amd64.deb')
    expect(checksums).toContain('publish/linux/Octopus.AppImage.tar.gz')
    expect(checksums).toContain('publish/linux/Octopus.AppImage.tar.gz.sig')
    expect(checksums).toContain('publish/linux/latest.json')
    expect(checksums).toContain('publish/windows/Octopus_0.1.0_x64-setup.exe')
    expect(checksums).toContain('publish/windows/Octopus_0.1.0_arm64-setup.exe')
    expect(checksums).toContain('publish/windows/Octopus.nsis.zip')
    expect(checksums).toContain('publish/windows/Octopus.nsis.zip.sig')
    expect(checksums).toContain('publish/windows/latest.json')
    expect(checksums).not.toContain('publish/macos/Octopus.app/')
    expect(checksums).not.toContain('Info.plist')
  })

  it('generates GitHub Pages updater manifests for both formal and preview channels from release assets', async () => {
    const outputDir = createTempDir('octopus-update-manifests-')

    const manifestByPath = new Map<string, unknown>([
      ['/asset/formal-macos-latest.json', {
        version: '0.2.0',
        notes: 'formal notes from asset should be replaced by release body',
        pub_date: '2026-04-08T10:00:00Z',
        platforms: {
          'darwin-aarch64': {
            signature: 'formal-macos-signature',
            url: 'https://example.invalid/Octopus.app.tar.gz',
          },
        },
      }],
      ['/asset/formal-windows-latest.json', {
        version: '0.2.0',
        notes: 'formal notes from asset should be replaced by release body',
        pub_date: '2026-04-08T10:00:00Z',
        platforms: {
          'windows-x86_64': {
            signature: 'formal-windows-signature',
            url: 'https://example.invalid/Octopus.nsis.zip',
          },
        },
      }],
      ['/asset/preview-macos-latest.json', {
        version: '0.2.0-preview.4',
        notes: 'preview notes from asset should be replaced by release body',
        pub_date: '2026-04-09T08:00:00Z',
        platforms: {
          'darwin-aarch64': {
            signature: 'preview-macos-signature',
            url: 'https://example.invalid/Octopus.app.tar.gz',
          },
        },
      }],
      ['/asset/preview-linux-latest.json', {
        version: '0.2.0-preview.4',
        notes: 'preview notes from asset should be replaced by release body',
        pub_date: '2026-04-09T08:00:00Z',
        platforms: {
          'linux-x86_64': {
            signature: 'preview-linux-signature',
            url: 'https://example.invalid/Octopus.AppImage.tar.gz',
          },
        },
      }],
    ])

    const server = createServer((request, response) => {
      if (!request.url) {
        response.statusCode = 404
        response.end('missing url')
        return
      }

      if (request.url === '/repos/GoyacJ/octopus/releases?per_page=20') {
        response.setHeader('content-type', 'application/json')
        response.end(JSON.stringify([
          {
            tag_name: 'v0.2.0',
            prerelease: false,
            published_at: '2026-04-08T11:30:00Z',
            body: 'Formal release body',
            html_url: 'https://github.com/GoyacJ/octopus/releases/tag/v0.2.0',
            assets: [
              {
                name: 'macos-latest.json',
                browser_download_url: 'https://github.com/GoyacJ/octopus/releases/download/v0.2.0/macos-latest.json',
                url: `http://127.0.0.1:${(server.address() as { port: number }).port}/asset/formal-macos-latest.json`,
              },
              {
                name: 'windows-latest.json',
                browser_download_url: 'https://github.com/GoyacJ/octopus/releases/download/v0.2.0/windows-latest.json',
                url: `http://127.0.0.1:${(server.address() as { port: number }).port}/asset/formal-windows-latest.json`,
              },
              {
                name: 'Octopus.app.tar.gz',
                browser_download_url: 'https://github.com/GoyacJ/octopus/releases/download/v0.2.0/Octopus.app.tar.gz',
                url: 'unused',
              },
              {
                name: 'Octopus.nsis.zip',
                browser_download_url: 'https://github.com/GoyacJ/octopus/releases/download/v0.2.0/Octopus.nsis.zip',
                url: 'unused',
              },
            ],
          },
          {
            tag_name: 'v0.2.0-preview.4',
            prerelease: true,
            published_at: '2026-04-09T09:15:00Z',
            body: 'Preview release body',
            html_url: 'https://github.com/GoyacJ/octopus/releases/tag/v0.2.0-preview.4',
            assets: [
              {
                name: 'macos-latest.json',
                browser_download_url: 'https://github.com/GoyacJ/octopus/releases/download/v0.2.0-preview.4/macos-latest.json',
                url: `http://127.0.0.1:${(server.address() as { port: number }).port}/asset/preview-macos-latest.json`,
              },
              {
                name: 'linux-latest.json',
                browser_download_url: 'https://github.com/GoyacJ/octopus/releases/download/v0.2.0-preview.4/linux-latest.json',
                url: `http://127.0.0.1:${(server.address() as { port: number }).port}/asset/preview-linux-latest.json`,
              },
              {
                name: 'Octopus.app.tar.gz',
                browser_download_url: 'https://github.com/GoyacJ/octopus/releases/download/v0.2.0-preview.4/Octopus.app.tar.gz',
                url: 'unused',
              },
              {
                name: 'Octopus.AppImage.tar.gz',
                browser_download_url: 'https://github.com/GoyacJ/octopus/releases/download/v0.2.0-preview.4/Octopus.AppImage.tar.gz',
                url: 'unused',
              },
            ],
          },
        ]))
        return
      }

      const payload = manifestByPath.get(request.url)
      if (payload) {
        response.setHeader('content-type', 'application/json')
        response.end(JSON.stringify(payload))
        return
      }

      response.statusCode = 404
      response.end('not found')
    })

    await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', () => resolve()))
    const closeServer = async () => {
      server.closeAllConnections()
      await new Promise<void>((resolve, reject) => server.close((error) => (error ? reject(error) : resolve())))
    }

    const port = (server.address() as { port: number }).port

    try {
      await execFileAsync(nodeExecutable, [
        generateUpdateManifestsScriptPath,
        '--repo',
        'GoyacJ/octopus',
        '--api-base-url',
        `http://127.0.0.1:${port}`,
        '--output-dir',
        outputDir,
      ], {
        cwd: repoRoot,
      })

      const formalManifest = JSON.parse(readFileSync(path.join(outputDir, 'formal', 'latest.json'), 'utf8')) as {
        version: string
        notes: string
        pub_date: string
        channel: string
        notesUrl: string
        platforms: Record<string, { signature: string, url: string }>
      }
      const previewManifest = JSON.parse(readFileSync(path.join(outputDir, 'preview', 'latest.json'), 'utf8')) as {
        version: string
        notes: string
        pub_date: string
        channel: string
        notesUrl: string
        platforms: Record<string, { signature: string, url: string }>
      }

      expect(formalManifest.version).toBe('0.2.0')
      expect(formalManifest.notes).toBe('Formal release body')
      expect(formalManifest.pub_date).toBe('2026-04-08T11:30:00Z')
      expect(formalManifest.channel).toBe('formal')
      expect(formalManifest.notesUrl).toBe('https://github.com/GoyacJ/octopus/releases/tag/v0.2.0')
      expect(formalManifest.platforms['darwin-aarch64']?.url).toBe('https://github.com/GoyacJ/octopus/releases/download/v0.2.0/Octopus.app.tar.gz')
      expect(formalManifest.platforms['windows-x86_64']?.url).toBe('https://github.com/GoyacJ/octopus/releases/download/v0.2.0/Octopus.nsis.zip')

      expect(previewManifest.version).toBe('0.2.0-preview.4')
      expect(previewManifest.notes).toBe('Preview release body')
      expect(previewManifest.pub_date).toBe('2026-04-09T09:15:00Z')
      expect(previewManifest.channel).toBe('preview')
      expect(previewManifest.notesUrl).toBe('https://github.com/GoyacJ/octopus/releases/tag/v0.2.0-preview.4')
      expect(previewManifest.platforms['darwin-aarch64']?.url).toBe('https://github.com/GoyacJ/octopus/releases/download/v0.2.0-preview.4/Octopus.app.tar.gz')
      expect(previewManifest.platforms['linux-x86_64']?.url).toBe('https://github.com/GoyacJ/octopus/releases/download/v0.2.0-preview.4/Octopus.AppImage.tar.gz')
    } finally {
      await closeServer()
    }
  })
})
