import { execFileSync } from 'node:child_process'
import { mkdtempSync, mkdirSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'

import { afterEach, describe, expect, it } from 'vitest'

const repoRoot = path.resolve(__dirname, '../../..')
const nodeExecutable = process.execPath
const scriptPath = path.join(repoRoot, 'scripts', 'prepare-agent-bundle-seed.mjs')
const tempDirectories: string[] = []

function createTempDir(prefix: string) {
  const directory = mkdtempSync(path.join(os.tmpdir(), prefix))
  tempDirectories.push(directory)
  return directory
}

function writeFile(filePath: string, contents: string | Buffer) {
  mkdirSync(path.dirname(filePath), { recursive: true })
  writeFileSync(filePath, contents)
}

function collectRelativeFiles(root: string, current = root, files: string[] = []) {
  for (const name of readdirSync(current).sort((left, right) => left.localeCompare(right))) {
    const absolutePath = path.join(current, name)
    const stats = statSync(absolutePath)
    if (stats.isDirectory()) {
      collectRelativeFiles(root, absolutePath, files)
      continue
    }
    files.push(path.relative(root, absolutePath).replace(/\\/g, '/'))
  }
  return files
}

function runGenerator(options: {
  templatesRoot: string
  outputRoot: string
  exampleRoot: string
  avatarLibraryRoot: string
}) {
  execFileSync(nodeExecutable, [
    scriptPath,
    '--templates-root',
    options.templatesRoot,
    '--output-root',
    options.outputRoot,
    '--example-root',
    options.exampleRoot,
    '--avatar-library-root',
    options.avatarLibraryRoot,
  ], {
    cwd: repoRoot,
    stdio: 'pipe',
  })
}

function createAvatarLibrary(root: string) {
  writeFile(path.join(root, 'employee', 'employee-1.png'), Buffer.from('employee-1'))
  writeFile(path.join(root, 'employee', 'employee-2.png'), Buffer.from('employee-2'))
  writeFile(path.join(root, 'leader', 'leader-1.png'), Buffer.from('leader-1'))
  writeFile(path.join(root, 'leader', 'leader-2.png'), Buffer.from('leader-2'))
}

function createBaseTemplates(root: string) {
  writeFile(path.join(root, 'skills', 'ledger-skill', 'SKILL.md'), [
    '---',
    'name: Ledger Skill',
    'description: Handle ledger calculations.',
    '---',
    '',
    '# Overview',
    '',
    'Compute finance ledgers.',
    '',
  ].join('\n'))
  writeFile(path.join(root, 'skills', 'ledger-skill', 'templates', 'formula.md'), 'FORMULA\n')
  writeFile(path.join(root, 'mcps', 'finance-ops.json'), JSON.stringify({
    type: 'http',
    url: 'https://example.com/mcp/finance',
  }, null, 2))
  writeFile(path.join(root, 'agents', 'Finance Planner', 'Finance Planner.md'), [
    '---',
    'name: Finance Planner',
    'description: Plans finance work.',
    'skills:',
    '  - ledger-skill',
    'mcps:',
    '  - finance-ops',
    '---',
    '',
    '# Finance Planner',
    '',
    'Own finance planning.',
    '',
  ].join('\n'))
  writeFile(path.join(root, 'teams', 'Finance Ops Team', 'Finance Ops Team.md'), [
    '---',
    'name: Finance Ops Team',
    'description: Coordinates finance delivery.',
    'skills:',
    '  - ledger-skill',
    'mcps:',
    '  - finance-ops',
    '---',
    '',
    '# Finance Ops Team',
    '',
    'Coordinate finance delivery.',
    '',
  ].join('\n'))
  writeFile(path.join(root, 'teams', 'Finance Ops Team', 'Analyst', 'Analyst.md'), [
    '---',
    'name: Analyst',
    'description: Reviews financial data.',
    'skills:',
    '  - ledger-skill',
    'mcps:',
    '  - finance-ops',
    '---',
    '',
    '# Analyst',
    '',
    'Review the numbers.',
    '',
  ].join('\n'))
}

afterEach(() => {
  for (const directory of tempDirectories.splice(0)) {
    rmSync(directory, { recursive: true, force: true })
  }
})

describe('prepare-agent-bundle-seed', () => {
  it('builds stable bundle and example outputs from templates with deterministic fallback avatars', () => {
    const tempDir = createTempDir('octopus-agent-bundle-seed-')
    const templatesRoot = path.join(tempDir, 'templates')
    const outputRoot = path.join(tempDir, 'seed', 'builtin-assets')
    const exampleRoot = path.join(tempDir, 'example', 'agent')
    const avatarLibraryRoot = path.join(tempDir, 'avatar-library')

    createAvatarLibrary(avatarLibraryRoot)
    createBaseTemplates(templatesRoot)

    runGenerator({ templatesRoot, outputRoot, exampleRoot, avatarLibraryRoot })
    const firstManifest = readFileSync(path.join(outputRoot, '.octopus', 'manifest.json'), 'utf8')

    runGenerator({ templatesRoot, outputRoot, exampleRoot, avatarLibraryRoot })
    const secondManifest = readFileSync(path.join(outputRoot, '.octopus', 'manifest.json'), 'utf8')

    expect(secondManifest).toBe(firstManifest)

    const manifest = JSON.parse(secondManifest) as {
      sourceMetadata?: Record<string, string>
      agents: Array<{ path: string, avatar: string, generatedAvatar: boolean, templatePath?: string }>
      teams: Array<{ path: string, avatar: string, generatedAvatar: boolean, templatePath?: string }>
      skills: Array<{ path: string, templatePath?: string }>
      mcps: Array<{ path: string, templatePath?: string }>
    }

    expect(manifest.sourceMetadata).toEqual(expect.objectContaining({
      generatedBy: 'scripts/prepare-agent-bundle-seed.mjs',
    }))
    expect(manifest.agents[0]?.generatedAvatar).toBe(true)
    expect(['employee-1.png', 'employee-2.png']).toContain(manifest.agents[0]?.avatar)
    expect(manifest.agents[0]?.templatePath).toBe('templates/agents/Finance Planner')
    expect(manifest.teams[0]?.generatedAvatar).toBe(true)
    expect(['leader-1.png', 'leader-2.png']).toContain(manifest.teams[0]?.avatar)
    expect(manifest.teams[0]?.templatePath).toBe('templates/teams/Finance Ops Team')
    expect(manifest.skills[0]?.templatePath).toBe('templates/skills/ledger-skill')
    expect(manifest.mcps[0]?.templatePath).toBe('templates/mcps/finance-ops.json')

    expect(collectRelativeFiles(path.join(outputRoot, 'bundle'))).toEqual(collectRelativeFiles(exampleRoot))
    expect(readFileSync(path.join(outputRoot, 'bundle', 'Finance Planner', 'Finance Planner.md'), 'utf8')).toContain(`avatar: ${manifest.agents[0]?.avatar}`)
    expect(readFileSync(path.join(outputRoot, 'bundle', 'Finance Ops Team', 'Finance Ops Team.md'), 'utf8')).toContain(`avatar: ${manifest.teams[0]?.avatar}`)
    expect(collectRelativeFiles(path.join(outputRoot, 'bundle'))).toContain('Finance Planner/mcps/finance-ops.json')
    expect(collectRelativeFiles(path.join(outputRoot, 'bundle'))).toContain('Finance Planner/skills/ledger-skill/SKILL.md')
    expect(collectRelativeFiles(path.join(outputRoot, 'bundle'))).toContain('.octopus/manifest.json')
  })

  it('fails when a template references a missing skill', () => {
    const tempDir = createTempDir('octopus-agent-bundle-missing-skill-')
    const templatesRoot = path.join(tempDir, 'templates')
    const outputRoot = path.join(tempDir, 'seed', 'builtin-assets')
    const exampleRoot = path.join(tempDir, 'example', 'agent')
    const avatarLibraryRoot = path.join(tempDir, 'avatar-library')

    createAvatarLibrary(avatarLibraryRoot)
    mkdirSync(path.join(templatesRoot, 'skills'), { recursive: true })
    writeFile(path.join(templatesRoot, 'mcps', 'finance-ops.json'), JSON.stringify({ type: 'http', url: 'https://example.com' }, null, 2))
    writeFile(path.join(templatesRoot, 'agents', 'Broken Agent', 'Broken Agent.md'), [
      '---',
      'name: Broken Agent',
      'skills:',
      '  - missing-skill',
      'mcps:',
      '  - finance-ops',
      '---',
      '',
      '# Broken Agent',
      '',
    ].join('\n'))

    expect(() => runGenerator({ templatesRoot, outputRoot, exampleRoot, avatarLibraryRoot }))
      .toThrowError(/missing skill/i)
  })

  it('fails when a template references a missing MCP', () => {
    const tempDir = createTempDir('octopus-agent-bundle-missing-mcp-')
    const templatesRoot = path.join(tempDir, 'templates')
    const outputRoot = path.join(tempDir, 'seed', 'builtin-assets')
    const exampleRoot = path.join(tempDir, 'example', 'agent')
    const avatarLibraryRoot = path.join(tempDir, 'avatar-library')

    createAvatarLibrary(avatarLibraryRoot)
    writeFile(path.join(templatesRoot, 'skills', 'ledger-skill', 'SKILL.md'), [
      '---',
      'name: Ledger Skill',
      'description: Handle ledger calculations.',
      '---',
      '',
      '# Overview',
      '',
    ].join('\n'))
    mkdirSync(path.join(templatesRoot, 'mcps'), { recursive: true })
    writeFile(path.join(templatesRoot, 'agents', 'Broken Agent', 'Broken Agent.md'), [
      '---',
      'name: Broken Agent',
      'skills:',
      '  - ledger-skill',
      'mcps:',
      '  - missing-mcp',
      '---',
      '',
      '# Broken Agent',
      '',
    ].join('\n'))

    expect(() => runGenerator({ templatesRoot, outputRoot, exampleRoot, avatarLibraryRoot }))
      .toThrowError(/missing MCP/i)
  })
})
