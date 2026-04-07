import { createHash } from 'node:crypto'
import { mkdir, readdir, readFile, rm, writeFile } from 'node:fs/promises'
import { dirname, join, relative } from 'node:path'
import { fileURLToPath } from 'node:url'

const FILTERED_DIR_NAMES = new Set([
  'node_modules',
  '.git',
  '.cache',
  '.turbo',
  'dist',
  'build',
  'coverage',
  '__pycache__',
  '.venv',
  'venv',
])

const __dirname = dirname(fileURLToPath(import.meta.url))
const repoRoot = join(__dirname, '..')
const defaultOutputDir = join(repoRoot, 'crates', 'octopus-infra', 'seed', 'agent-bundle')

function normalizePath(value) {
  return value.replace(/\\/g, '/')
}

function isFrontmatterDelimiter(line) {
  const trimmed = line.trim()
  return trimmed.length >= 3 && /^-+$/.test(trimmed)
}

function unquoteFrontmatterValue(value) {
  return value.replace(/^['"]|['"]$/g, '').trim()
}

function splitFrontmatter(contents) {
  const normalized = contents.replace(/\r\n/g, '\n')
  const lines = normalized.split('\n')
  if (!isFrontmatterDelimiter(lines[0] ?? '')) {
    return { frontmatter: new Map(), body: normalized }
  }

  const frontmatter = new Map()
  let index = 1
  while (index < lines.length) {
    const line = lines[index]
    if (isFrontmatterDelimiter(line)) {
      index += 1
      break
    }
    const splitIndex = line.indexOf(':')
    if (splitIndex > 0) {
      const key = line.slice(0, splitIndex).trim()
      const value = unquoteFrontmatterValue(line.slice(splitIndex + 1).trim())
      if (key && value) {
        frontmatter.set(key, value)
      }
    }
    index += 1
  }

  return {
    frontmatter,
    body: lines.slice(index).join('\n'),
  }
}

function firstNonEmptyParagraph(body) {
  const parts = []
  for (const rawLine of body.split('\n')) {
    const line = rawLine.trim()
    if (!line) {
      if (parts.length) {
        break
      }
      continue
    }
    if (line.startsWith('#')) {
      if (parts.length) {
        break
      }
      continue
    }
    parts.push(line)
  }
  return parts.length ? parts.join(' ') : null
}

function firstParagraphAfterHeading(body, heading) {
  let found = false
  const parts = []
  for (const rawLine of body.split('\n')) {
    const line = rawLine.trim()
    if (!found) {
      if (line.startsWith('#') && line.replace(/^#+/, '').trim() === heading) {
        found = true
      }
      continue
    }
    if (!line) {
      if (parts.length) {
        break
      }
      continue
    }
    if (line.startsWith('#')) {
      break
    }
    parts.push(line)
  }
  return parts.length ? parts.join(' ') : null
}

function hashBytes(chunks) {
  const hash = createHash('sha256')
  for (const chunk of chunks) {
    hash.update(chunk)
  }
  return `sha256-${hash.digest('hex')}`
}

function hashText(value) {
  return hashBytes([Buffer.from(value, 'utf8')])
}

function hashBundleFiles(files) {
  const chunks = []
  for (const file of files) {
    chunks.push(Buffer.from(file.relativePath, 'utf8'))
    chunks.push(Buffer.from('\n', 'utf8'))
    chunks.push(file.contents)
    chunks.push(Buffer.from('\n', 'utf8'))
  }
  return hashBytes(chunks)
}

function shortHash(value) {
  return value.split('-').at(-1)?.slice(0, 8) ?? value.slice(0, 8)
}

function slugifySkillName(value, fallbackPrefix = 'skill') {
  let slug = ''
  let lastWasSeparator = false
  for (const character of value) {
    if (/[A-Za-z0-9]/.test(character)) {
      slug += character.toLowerCase()
      lastWasSeparator = false
      continue
    }
    if (/[-_. ]/.test(character) && slug && !lastWasSeparator) {
      slug += '-'
      lastWasSeparator = true
    }
  }
  slug = slug.replace(/-+$/g, '')
  return slug || `${fallbackPrefix}-${shortHash(hashText(value))}`
}

async function collectFiles(root, current, files = []) {
  const entries = await readdir(current, { withFileTypes: true })
  for (const entry of entries) {
    if (FILTERED_DIR_NAMES.has(entry.name)) {
      continue
    }
    const absolutePath = join(current, entry.name)
    if (entry.isDirectory()) {
      await collectFiles(root, absolutePath, files)
      continue
    }
    const relativePath = normalizePath(relative(root, absolutePath))
    files.push({
      relativePath,
      contents: await readFile(absolutePath),
    })
  }
  return files
}

function groupAgentFiles(files) {
  const grouped = new Map()
  for (const file of files) {
    const segments = file.relativePath.split('/')
    if (segments.length < 2) {
      continue
    }
    const key = `${segments[0]}/${segments[1]}`
    const list = grouped.get(key) ?? []
    list.push(file)
    grouped.set(key, list)
  }
  return grouped
}

function groupSkillFiles(department, agentDir, files) {
  const prefix = `${department}/${agentDir}/skills/`
  const grouped = new Map()
  for (const file of files) {
    if (!file.relativePath.startsWith(prefix)) {
      continue
    }
    const suffix = file.relativePath.slice(prefix.length)
    const segments = suffix.split('/')
    if (segments.length < 2) {
      continue
    }
    const skillDir = segments[0]
    const relativePath = segments.slice(1).join('/')
    const list = grouped.get(skillDir) ?? []
    list.push({ relativePath, contents: file.contents })
    grouped.set(skillDir, list)
  }
  for (const filesForSkill of grouped.values()) {
    filesForSkill.sort((left, right) => left.relativePath.localeCompare(right.relativePath))
  }
  return grouped
}

async function ensureDirectory(path) {
  await mkdir(path, { recursive: true })
}

async function writeSeed(outputDir, manifest, skills) {
  await rm(outputDir, { recursive: true, force: true })
  await ensureDirectory(outputDir)
  await ensureDirectory(join(outputDir, 'skills'))

  for (const skill of skills) {
    const skillDir = join(outputDir, 'skills', skill.slug)
    await ensureDirectory(skillDir)
    for (const file of skill.files) {
      const target = join(skillDir, file.relativePath)
      await ensureDirectory(dirname(target))
      await writeFile(target, file.contents)
    }
  }

  await writeFile(join(outputDir, 'manifest.json'), `${JSON.stringify(manifest, null, 2)}\n`)
}

async function main() {
  const sourceRoot = process.argv[2]
  const outputDir = process.argv[3] ? join(repoRoot, process.argv[3]) : defaultOutputDir

  if (!sourceRoot) {
    console.error('Usage: node scripts/prepare-agent-bundle-seed.mjs <source-root> [output-dir]')
    process.exit(1)
  }

  const files = await collectFiles(sourceRoot, sourceRoot)
  const groupedAgents = groupAgentFiles(files)
  const parsedAgents = []
  const parsedSkillSources = []

  for (const [key, groupFiles] of [...groupedAgents.entries()].sort((a, b) => a[0].localeCompare(b[0]))) {
    const [department, agentDir] = key.split('/')
    const markdownPath = `${department}/${agentDir}/${agentDir}.md`
    const agentFile = groupFiles.find(file => file.relativePath === markdownPath)
    if (!agentFile) {
      continue
    }

    const { frontmatter, body } = splitFrontmatter(agentFile.contents.toString('utf8'))
    const agentName = frontmatter.get('name')?.trim() || agentDir
    const description = frontmatter.get('description')?.trim()
      || firstNonEmptyParagraph(body)
      || agentName
    const personality = firstParagraphAfterHeading(body, '角色定义')
      || firstParagraphAfterHeading(body, 'Role Definition')
      || agentName
    const prompt = body.trim()
    const sourceId = `${department}/${agentDir}`

    const skillSourceIds = []
    for (const [skillDir, skillFiles] of [...groupSkillFiles(department, agentDir, groupFiles).entries()].sort((a, b) => a[0].localeCompare(b[0]))) {
      const skillFile = skillFiles.find(file => file.relativePath === 'SKILL.md')
      if (!skillFile) {
        continue
      }

      const { frontmatter: skillFrontmatter } = splitFrontmatter(skillFile.contents.toString('utf8'))
      const skillName = skillFrontmatter.get('name')?.trim() || skillDir
      const canonicalSlug = slugifySkillName(skillName)
      const contentHash = hashBundleFiles(skillFiles)
      const skillSourceId = `${sourceId}/skills/${skillDir}`

      skillSourceIds.push(skillSourceId)
      parsedSkillSources.push({
        sourceId: skillSourceId,
        department,
        agentName,
        name: skillName,
        canonicalSlug,
        contentHash,
        files: skillFiles,
      })
    }

    parsedAgents.push({
      sourceId,
      department,
      name: agentName,
      description,
      personality,
      prompt,
      skillSourceIds,
    })
  }

  const uniqueSkillSources = new Map()
  for (const skillSource of parsedSkillSources) {
    const key = `${skillSource.canonicalSlug}:${skillSource.contentHash}`
    const current = uniqueSkillSources.get(key) ?? {
      name: skillSource.name,
      canonicalSlug: skillSource.canonicalSlug,
      contentHash: skillSource.contentHash,
      files: skillSource.files,
      sourceIds: [],
    }
    current.sourceIds.push(skillSource.sourceId)
    uniqueSkillSources.set(key, current)
  }

  const assignedHashBySlug = new Map()
  const sourceSlugMap = new Map()
  const skills = []
  for (const skill of [...uniqueSkillSources.values()].sort((a, b) => {
    const bySlug = a.canonicalSlug.localeCompare(b.canonicalSlug)
    return bySlug || a.contentHash.localeCompare(b.contentHash)
  })) {
    let slug = skill.canonicalSlug
    if (assignedHashBySlug.has(slug) && assignedHashBySlug.get(slug) !== skill.contentHash) {
      slug = `${slug}-${shortHash(skill.contentHash)}`
    }
    assignedHashBySlug.set(slug, skill.contentHash)
    skills.push({
      slug,
      files: skill.files,
      contentHash: skill.contentHash,
      sourceIds: skill.sourceIds,
    })
    for (const sourceId of skill.sourceIds) {
      sourceSlugMap.set(sourceId, slug)
    }
  }

  const manifest = {
    agents: parsedAgents.map(agent => ({
      sourceId: agent.sourceId,
      department: agent.department,
      name: agent.name,
      description: agent.description,
      personality: agent.personality,
      prompt: agent.prompt,
      skillSlugs: agent.skillSourceIds
        .map(sourceId => sourceSlugMap.get(sourceId))
        .filter(Boolean),
    })),
    skillAssets: skills.map(skill => ({ slug: skill.slug })),
    skillSources: parsedSkillSources.map(skill => ({
      sourceId: skill.sourceId,
      department: skill.department,
      slug: sourceSlugMap.get(skill.sourceId),
      contentHash: skill.contentHash,
    })),
  }

  await writeSeed(outputDir, manifest, skills)

  console.log(
    `Prepared agent bundle seed: ${manifest.agents.length} agents, ${parsedSkillSources.length} skill sources, ${skills.length} unique skills -> ${outputDir}`,
  )
}

await main()
