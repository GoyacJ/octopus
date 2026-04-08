import { execFileSync } from 'node:child_process'
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'

import { afterEach, describe, expect, it } from 'vitest'

const repoRoot = path.resolve(__dirname, '../../..')
const nodeExecutable = process.execPath
const verifyScriptPath = path.join(repoRoot, 'scripts', 'verify-release-artifacts.mjs')
const collectScriptPath = path.join(repoRoot, 'scripts', 'collect-release-artifacts.mjs')
const releaseNotesScriptPath = path.join(repoRoot, 'scripts', 'generate-release-notes.mjs')
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
  it('generates release notes from user fragments without including the fragment readme', () => {
    const outputDir = createTempDir('octopus-release-notes-')
    const outputPath = path.join(outputDir, 'v0.1.0.md')

    execFileSync(nodeExecutable, [
      releaseNotesScriptPath,
      '--output',
      outputPath,
    ], {
      cwd: repoRoot,
      stdio: 'pipe',
    })

    const notes = readFileSync(outputPath, 'utf8')
    expect(notes).toContain('2026-04-08-initial-release-governance')
    expect(notes).not.toContain('## README')
  })

  it('fails verification when formal desktop installers are missing', () => {
    const artifactsDir = createTempDir('octopus-release-artifacts-')
    const metadataDir = path.join(artifactsDir, 'metadata')
    const notesPath = path.join(metadataDir, 'v0.1.0.md')

    writeFile(path.join(metadataDir, 'VERSION'), '0.1.0\n')
    writeFile(path.join(metadataDir, 'octopus.openapi.yaml'), 'openapi: 3.1.0\n')
    writeFile(path.join(metadataDir, 'generated.ts'), 'export {}\n')
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
    ).toThrowError(/formal desktop installer/i)
  })

  it('collects publishable bundles and verification writes release checksums', () => {
    const sourceDir = createTempDir('octopus-release-source-')
    const artifactsDir = createTempDir('octopus-release-output-')
    const metadataDir = path.join(artifactsDir, 'metadata')
    const notesPath = path.join(metadataDir, 'v0.1.0.md')
    const publishDir = path.join(artifactsDir, 'publish')
    const checksumsPath = path.join(artifactsDir, 'SHA256SUMS.txt')

    writeFile(path.join(sourceDir, 'dmg', 'Octopus_0.1.0_aarch64.dmg'), 'macos bundle')
    writeFile(path.join(sourceDir, 'macos', 'Octopus.app', 'Contents', 'Info.plist'), 'ignored app bundle')
    writeFile(path.join(sourceDir, 'nsis', 'Octopus_0.1.0_x64-setup.exe'), 'windows setup')
    writeFile(path.join(sourceDir, 'README.txt'), 'ignore me')

    writeFile(path.join(metadataDir, 'VERSION'), '0.1.0\n')
    writeFile(path.join(metadataDir, 'octopus.openapi.yaml'), 'openapi: 3.1.0\n')
    writeFile(path.join(metadataDir, 'generated.ts'), 'export {}\n')
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
    expect(checksums).toContain('publish/macos/Octopus_0.1.0_aarch64.dmg')
    expect(checksums).toContain('publish/windows/Octopus_0.1.0_x64-setup.exe')
    expect(checksums).not.toContain('Octopus.app')
  })
})
