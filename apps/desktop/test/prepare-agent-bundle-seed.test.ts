import { execFileSync } from 'node:child_process'
import { mkdtempSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs'
import os from 'node:os'
import path from 'node:path'

import { afterEach, describe, expect, it } from 'vitest'

const repoRoot = path.resolve(__dirname, '../../..')
const nodeExecutable = process.execPath
const scriptPath = path.join(repoRoot, 'scripts', 'prepare-template-snapshots.mjs')
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
}) {
  execFileSync(nodeExecutable, [
    scriptPath,
    '--templates-root',
    options.templatesRoot,
    '--output-root',
    options.outputRoot,
    '--example-root',
    options.exampleRoot,
  ], {
    cwd: repoRoot,
    stdio: 'pipe',
  })
}

function createBaseTemplates(root: string) {
  writeFile(path.join(root, '财务部', '财务部门说明.md'), [
    '---',
    'name: 财务部',
    'description: 负责财务统筹。',
    'leader: 财务负责人',
    'member:',
    '  - 财务负责人',
    '  - 财务分析师',
    'avatar: 头像',
    '---',
    '',
    '# 团队职责',
    '',
    '负责财务统筹。',
    '',
  ].join('\n'))

  writeFile(path.join(root, '财务部', '财务负责人', '财务负责人.md'), [
    '---',
    'name: 财务负责人',
    'description: 负责财务管理。',
    'avatar: 头像',
    'skills:',
    '  - ledger-skill',
    'mcps:',
    '  - finance-ops',
    '---',
    '',
    '# 角色定义',
    '',
    '管理财务流程。',
    '',
  ].join('\n'))
  writeFile(path.join(root, '财务部', '财务负责人', 'skills', 'ledger-skill', 'SKILL.md'), [
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
  writeFile(
    path.join(root, '财务部', '财务负责人', 'mcps', 'finance-ops.json'),
    JSON.stringify({ type: 'http', url: 'https://example.com/mcp/finance' }, null, 2),
  )

  writeFile(path.join(root, '财务部', '财务分析师', '财务分析师.md'), [
    '---',
    'name: 财务分析师',
    'description: 审核财务数据。',
    'avatar: 头像',
    '---',
    '',
    '# 角色定义',
    '',
    'Review the numbers.',
    '',
  ].join('\n'))

  writeFile(path.join(root, '市场部', '技术写作者', '技术写作者.md'), [
    '---',
    'name: 技术写作者',
    'description: 负责文档编写。',
    'avatar: 头像',
    '---',
    '',
    '# 角色定义',
    '',
    'Write technical content.',
    '',
  ].join('\n'))

  writeFile(path.join(root, '管理层与PMO', '项目经理', '项目经理.md'), [
    '---',
    'name: 项目经理',
    'description: 应被忽略。',
    '---',
    '',
    '# 角色定义',
    '',
    'Ignored.',
    '',
  ].join('\n'))
}

afterEach(() => {
  for (const directory of tempDirectories.splice(0)) {
    rmSync(directory, { recursive: true, force: true })
  }
})

describe('prepare-template-snapshots', () => {
  it('mirrors current template trees into output snapshots and skips ignored template roots', () => {
    const tempDir = createTempDir('octopus-template-snapshot-')
    const templatesRoot = path.join(tempDir, 'templates')
    const outputRoot = path.join(tempDir, 'seed', 'builtin-assets')
    const exampleRoot = path.join(tempDir, 'example', 'agent')

    createBaseTemplates(templatesRoot)

    runGenerator({ templatesRoot, outputRoot, exampleRoot })

    expect(collectRelativeFiles(outputRoot)).toEqual(collectRelativeFiles(exampleRoot))
    expect(collectRelativeFiles(outputRoot)).toContain('财务部/财务部门说明.md')
    expect(collectRelativeFiles(outputRoot)).toContain('财务部/财务负责人/skills/ledger-skill/SKILL.md')
    expect(collectRelativeFiles(outputRoot)).toContain('市场部/技术写作者/技术写作者.md')
    expect(collectRelativeFiles(outputRoot)).not.toContain('管理层与PMO/项目经理/项目经理.md')
    expect(collectRelativeFiles(outputRoot)).not.toContain('.octopus/manifest.json')
    expect(readFileSync(path.join(outputRoot, '财务部', '财务负责人', '财务负责人.md'), 'utf8'))
      .toContain('skills:')
  })

  it('preserves explicit svg avatars from current template directories', () => {
    const tempDir = createTempDir('octopus-template-avatar-')
    const templatesRoot = path.join(tempDir, 'templates')
    const outputRoot = path.join(tempDir, 'seed', 'builtin-assets')
    const exampleRoot = path.join(tempDir, 'example', 'agent')

    writeFile(path.join(templatesRoot, '产品部', '视觉设计师', '视觉设计师.md'), [
      '---',
      'name: 视觉设计师',
      'avatar: portrait.svg',
      '---',
      '',
      '# 角色定义',
      '',
      'Design visuals.',
      '',
    ].join('\n'))
    writeFile(
      path.join(templatesRoot, '产品部', '视觉设计师', 'portrait.svg'),
      '<svg xmlns="http://www.w3.org/2000/svg"><rect width="8" height="8"/></svg>',
    )

    runGenerator({ templatesRoot, outputRoot, exampleRoot })

    expect(readFileSync(path.join(outputRoot, '产品部', '视觉设计师', '视觉设计师.md'), 'utf8'))
      .toContain('avatar: portrait.svg')
    expect(collectRelativeFiles(outputRoot)).toContain('产品部/视觉设计师/portrait.svg')
  })

  it('fails when an agent template references a missing local skill', () => {
    const tempDir = createTempDir('octopus-template-missing-skill-')
    const templatesRoot = path.join(tempDir, 'templates')
    const outputRoot = path.join(tempDir, 'seed', 'builtin-assets')
    const exampleRoot = path.join(tempDir, 'example', 'agent')

    writeFile(path.join(templatesRoot, '财务部', '财务分析师', '财务分析师.md'), [
      '---',
      'name: 财务分析师',
      'skills:',
      '  - missing-skill',
      '---',
      '',
      '# 角色定义',
      '',
    ].join('\n'))

    expect(() => runGenerator({ templatesRoot, outputRoot, exampleRoot }))
      .toThrowError(/missing skill/i)
  })

  it('fails when an agent template references a missing local MCP', () => {
    const tempDir = createTempDir('octopus-template-missing-mcp-')
    const templatesRoot = path.join(tempDir, 'templates')
    const outputRoot = path.join(tempDir, 'seed', 'builtin-assets')
    const exampleRoot = path.join(tempDir, 'example', 'agent')

    writeFile(path.join(templatesRoot, '财务部', '财务分析师', '财务分析师.md'), [
      '---',
      'name: 财务分析师',
      'mcps:',
      '  - missing-mcp',
      '---',
      '',
      '# 角色定义',
      '',
    ].join('\n'))

    expect(() => runGenerator({ templatesRoot, outputRoot, exampleRoot }))
      .toThrowError(/missing MCP/i)
  })
})
