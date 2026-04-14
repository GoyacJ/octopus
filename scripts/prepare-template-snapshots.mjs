import { copyFile, mkdir, readdir, readFile, rm } from 'node:fs/promises'
import { basename, dirname, join, relative, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

import YAML from 'yaml'

const __dirname = dirname(fileURLToPath(import.meta.url))
const repoRoot = join(__dirname, '..')
const defaultConfig = {
  repoRoot,
  templatesRoot: join(repoRoot, 'templates'),
  outputRoot: join(repoRoot, 'crates', 'octopus-infra', 'seed', 'builtin-assets'),
  exampleRoot: join(repoRoot, 'example', 'agent'),
}

const FILTERED_DIR_NAMES = new Set([
  '.DS_Store',
  '.git',
  '.turbo',
  '.cache',
  'node_modules',
  'dist',
  'build',
  'coverage',
  '__pycache__',
  '.venv',
  'venv',
])

const IGNORED_TEMPLATE_ROOTS = new Set(['系统通用', '管理层与PMO'])
const OWNER_EXCLUDED_SEGMENTS = new Set(['skills', 'mcps', 'references', 'templates', 'scripts'])

function parseCliArguments(argv) {
  const entries = new Map()
  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index]
    if (!token.startsWith('--')) {
      continue
    }
    const key = token.slice(2)
    const value = argv[index + 1]
    if (!value || value.startsWith('--')) {
      throw new Error(`Missing value for --${key}`)
    }
    entries.set(key, value)
    index += 1
  }
  return entries
}

function resolvePathOption(value, fallback) {
  return value ? resolve(value) : fallback
}

function resolveConfig(argv = process.argv.slice(2)) {
  const cliArguments = parseCliArguments(argv)
  return {
    ...defaultConfig,
    templatesRoot: resolvePathOption(cliArguments.get('templates-root'), defaultConfig.templatesRoot),
    outputRoot: resolvePathOption(cliArguments.get('output-root'), defaultConfig.outputRoot),
    exampleRoot: resolvePathOption(cliArguments.get('example-root'), defaultConfig.exampleRoot),
  }
}

function normalizePath(value) {
  return value.replace(/\\/g, '/')
}

function topLevelDirectory(relativePath) {
  return normalizePath(relativePath).split('/')[0] ?? ''
}

function isFrontmatterDelimiter(line) {
  const trimmed = line.trim()
  return trimmed.length >= 3 && /^-+$/.test(trimmed)
}

function sanitizeFrontmatterLine(line) {
  const trimmed = line.trimEnd()
  if (trimmed !== '---' && trimmed.endsWith('---') && trimmed.includes(':')) {
    return trimmed.replace(/-+$/g, '').trimEnd()
  }
  return line
}

function stripWrappingQuotes(value) {
  if (value.length >= 2) {
    const first = value[0]
    const last = value[value.length - 1]
    if ((first === '"' && last === '"') || (first === '\'' && last === '\'')) {
      return value.slice(1, -1)
    }
  }
  return value
}

function parseFrontmatterEntryLine(line) {
  if (!line.trim() || /^\s/.test(line)) {
    return null
  }
  const colonIndex = line.indexOf(':')
  if (colonIndex <= 0) {
    return null
  }
  return {
    key: line.slice(0, colonIndex).trim(),
    value: line.slice(colonIndex + 1).trim(),
  }
}

function parseFrontmatterScalar(value) {
  if (!value.trim()) {
    return null
  }
  try {
    return YAML.parse(value)
  }
  catch {
    return stripWrappingQuotes(value)
  }
}

function parseFrontmatterFallback(lines) {
  const frontmatter = {}
  let currentKey = null
  let currentValueLines = []

  const flush = () => {
    if (!currentKey) {
      currentValueLines = []
      return
    }
    const normalized = currentValueLines
      .map(line => stripWrappingQuotes(line.trim()))
      .filter(Boolean)
      .join(' ')
    frontmatter[currentKey] = parseFrontmatterScalar(normalized)
    currentKey = null
    currentValueLines = []
  }

  for (const line of lines) {
    const parsed = parseFrontmatterEntryLine(line)
    if (parsed) {
      flush()
      currentKey = parsed.key
      currentValueLines.push(parsed.value)
      continue
    }
    if (currentKey && line.trim()) {
      currentValueLines.push(line.trim())
    }
  }

  flush()
  return frontmatter
}

function splitFrontmatter(contents) {
  const normalized = contents.replace(/\r\n/g, '\n')
  const lines = normalized.split('\n')
  if (!isFrontmatterDelimiter(lines[0] ?? '')) {
    return { frontmatter: {}, body: normalized }
  }

  const frontmatterLines = []
  let index = 1
  while (index < lines.length) {
    const line = lines[index]
    if (isFrontmatterDelimiter(line)) {
      index += 1
      break
    }
    frontmatterLines.push(sanitizeFrontmatterLine(line))
    index += 1
  }

  const frontmatterText = frontmatterLines.join('\n')
  let frontmatter = {}
  if (frontmatterText.trim()) {
    try {
      frontmatter = YAML.parse(frontmatterText) ?? {}
    }
    catch {
      frontmatter = parseFrontmatterFallback(frontmatterLines)
    }
  }

  return {
    frontmatter,
    body: lines.slice(index).join('\n'),
  }
}

function stableStringList(value) {
  if (Array.isArray(value)) {
    return value.map(item => String(item).trim()).filter(Boolean)
  }
  if (typeof value === 'string') {
    return value
      .split(/[、，,;]/g)
      .map(item => item.trim())
      .filter(Boolean)
  }
  return []
}

async function ensureDirectory(path) {
  await mkdir(path, { recursive: true })
}

async function collectTemplateFiles(root, current = root, files = []) {
  const entries = await readdir(current, { withFileTypes: true })
  for (const entry of entries.sort((left, right) => left.name.localeCompare(right.name))) {
    if (FILTERED_DIR_NAMES.has(entry.name)) {
      continue
    }
    const absolutePath = join(current, entry.name)
    const relativePath = normalizePath(relative(root, absolutePath))
    if (IGNORED_TEMPLATE_ROOTS.has(topLevelDirectory(relativePath))) {
      continue
    }
    if (entry.isDirectory()) {
      await collectTemplateFiles(root, absolutePath, files)
      continue
    }
    files.push({ absolutePath, relativePath })
  }
  return files
}

function isOwnerTemplate(relativePath) {
  if (!relativePath.endsWith('.md')) {
    return false
  }
  if (basename(relativePath) === 'SKILL.md') {
    return false
  }
  const segments = normalizePath(relativePath).split('/')
  if (segments.some(segment => OWNER_EXCLUDED_SEGMENTS.has(segment))) {
    return false
  }
  const fileName = basename(relativePath)
  if (fileName.endsWith('说明.md')) {
    return true
  }
  const fileStem = fileName.slice(0, -'.md'.length)
  const dirName = basename(dirname(relativePath))
  return fileStem === dirName
}

async function assertFileExists(filePath, message) {
  try {
    await readFile(filePath)
  }
  catch {
    throw new Error(message)
  }
}

async function validateOwnerTemplate(file) {
  const contents = await readFile(file.absolutePath, 'utf8')
  const { frontmatter } = splitFrontmatter(contents)
  const ownerDir = dirname(file.absolutePath)
  const relativePath = file.relativePath
  const ownerLabel = basename(file.absolutePath).endsWith('说明.md') ? 'team' : 'agent'

  for (const skillSlug of stableStringList(frontmatter.skills)) {
    await assertFileExists(
      join(ownerDir, 'skills', skillSlug, 'SKILL.md'),
      `${ownerLabel} template '${relativePath}' references missing skill '${skillSlug}'`,
    )
  }

  for (const serverName of stableStringList(frontmatter.mcps)) {
    const mcpPath = join(ownerDir, 'mcps', `${serverName}.json`)
    await assertFileExists(
      mcpPath,
      `${ownerLabel} template '${relativePath}' references missing MCP '${serverName}'`,
    )
    JSON.parse(await readFile(mcpPath, 'utf8'))
  }

  if (basename(file.absolutePath).endsWith('说明.md')) {
    const teamMembers = stableStringList(frontmatter.member)
    for (const memberName of teamMembers) {
      await assertFileExists(
        join(ownerDir, memberName, `${memberName}.md`),
        `team template '${relativePath}' references missing member '${memberName}'`,
      )
    }

    if (typeof frontmatter.leader === 'string' && frontmatter.leader.trim()) {
      const leaderName = frontmatter.leader.trim()
      await assertFileExists(
        join(ownerDir, leaderName, `${leaderName}.md`),
        `team template '${relativePath}' references missing leader '${leaderName}'`,
      )
    }
  }
}

async function validateTemplates(config) {
  const files = await collectTemplateFiles(config.templatesRoot)
  for (const file of files) {
    if (!isOwnerTemplate(file.relativePath)) {
      continue
    }
    await validateOwnerTemplate(file)
  }
  return files
}

async function syncTemplateSnapshot(files, destinationRoot) {
  await rm(destinationRoot, { recursive: true, force: true })
  await ensureDirectory(destinationRoot)

  for (const file of files) {
    const targetPath = join(destinationRoot, file.relativePath)
    await ensureDirectory(dirname(targetPath))
    await copyFile(file.absolutePath, targetPath)
  }
}

async function main() {
  const config = resolveConfig()
  const files = await validateTemplates(config)
  await syncTemplateSnapshot(files, config.outputRoot)
  await syncTemplateSnapshot(files, config.exampleRoot)
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : error)
  process.exit(1)
})
