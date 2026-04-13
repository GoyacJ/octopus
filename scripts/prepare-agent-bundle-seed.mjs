import { createHash } from 'node:crypto'
import { mkdir, readdir, readFile, rm, stat, writeFile } from 'node:fs/promises'
import { dirname, join, relative, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

import YAML from 'yaml'

const __dirname = dirname(fileURLToPath(import.meta.url))
const repoRoot = join(__dirname, '..')
const defaultConfig = {
  repoRoot,
  templatesRoot: join(repoRoot, 'templates'),
  outputRoot: join(repoRoot, 'crates', 'octopus-infra', 'seed', 'builtin-assets'),
  exampleRoot: join(repoRoot, 'example', 'agent'),
  avatarLibraryRoot: join(repoRoot, 'packages', 'assets', 'header'),
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

const SUPPORTED_AVATAR_EXTENSIONS = new Set(['.png', '.jpg', '.jpeg', '.webp'])

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
  const outputRoot = resolvePathOption(cliArguments.get('output-root'), defaultConfig.outputRoot)
  return {
    ...defaultConfig,
    templatesRoot: resolvePathOption(cliArguments.get('templates-root'), defaultConfig.templatesRoot),
    outputRoot,
    outputBundleRoot: join(outputRoot, 'bundle'),
    exampleRoot: resolvePathOption(cliArguments.get('example-root'), defaultConfig.exampleRoot),
    avatarLibraryRoot: resolvePathOption(cliArguments.get('avatar-library-root'), defaultConfig.avatarLibraryRoot),
  }
}

function normalizePath(value) {
  return value.replace(/\\/g, '/')
}

function isFrontmatterDelimiter(line) {
  const trimmed = line.trim()
  return trimmed.length >= 3 && /^-+$/.test(trimmed)
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
    frontmatterLines.push(line)
    index += 1
  }

  return {
    frontmatter: frontmatterLines.length > 0 ? (YAML.parse(frontmatterLines.join('\n')) ?? {}) : {},
    body: lines.slice(index).join('\n'),
  }
}

function renderMarkdown(frontmatter, body) {
  const normalizedBody = body.replace(/\r\n/g, '\n').replace(/\s+$/g, '')
  const renderedFrontmatter = YAML.stringify(frontmatter, {
    lineWidth: 0,
    defaultStringType: 'PLAIN',
  }).trimEnd()
  return `---\n${renderedFrontmatter}\n---\n\n${normalizedBody}\n`
}

function hashText(value) {
  return createHash('sha256').update(value).digest('hex')
}

function shortHash(value) {
  return hashText(value).slice(0, 8)
}

function slugify(value, fallbackPrefix = 'asset') {
  let slug = ''
  let lastWasSeparator = false
  for (const character of value) {
    if (/[A-Za-z0-9]/.test(character)) {
      slug += character.toLowerCase()
      lastWasSeparator = false
      continue
    }
    if (slug && !lastWasSeparator && /[-_. ]/.test(character)) {
      slug += '-'
      lastWasSeparator = true
    }
  }
  slug = slug.replace(/-+$/g, '')
  return slug || `${fallbackPrefix}-${shortHash(value)}`
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

async function collectFiles(root, current, files = []) {
  const entries = await readdir(current, { withFileTypes: true })
  for (const entry of entries.sort((left, right) => left.name.localeCompare(right.name))) {
    if (FILTERED_DIR_NAMES.has(entry.name)) {
      continue
    }
    const absolutePath = join(current, entry.name)
    if (entry.isDirectory()) {
      await collectFiles(root, absolutePath, files)
      continue
    }
    files.push({
      relativePath: normalizePath(relative(root, absolutePath)),
      bytes: await readFile(absolutePath),
    })
  }
  return files
}

async function writeBundleFiles(root, files) {
  for (const file of files) {
    const targetPath = join(root, file.relativePath)
    await ensureDirectory(dirname(targetPath))
    await writeFile(targetPath, file.bytes)
  }
}

function toSourceMetadataPath(config, targetPath) {
  const relativePath = normalizePath(relative(config.repoRoot, targetPath))
  if (relativePath && !relativePath.startsWith('..')) {
    return relativePath
  }
  return normalizePath(targetPath)
}

async function loadAvatarLibrary(kind, config) {
  const dir = join(config.avatarLibraryRoot, kind === 'team' ? 'leader' : 'employee')
  const entries = await readdir(dir, { withFileTypes: true })
  const files = []
  for (const entry of entries.sort((left, right) => left.name.localeCompare(right.name))) {
    if (!entry.isFile()) {
      continue
    }
    files.push({
      fileName: entry.name,
      bytes: await readFile(join(dir, entry.name)),
    })
  }
  if (files.length === 0) {
    throw new Error(`No default avatar assets found for ${kind}`)
  }
  return files
}

function deterministicIndex(seedKey, size) {
  if (size === 0) {
    return 0
  }
  const digest = createHash('sha256').update(seedKey).digest()
  let value = 0n
  for (const byte of digest.subarray(0, 8)) {
    value = (value << 8n) + BigInt(byte)
  }
  return Number(value % BigInt(size))
}

async function resolveOwnerAvatar(templateDir, ownerKind, sourceId, explicitAvatarName, avatarLibraries) {
  const entries = await readdir(templateDir, { withFileTypes: true })
  const avatarCandidates = entries
    .filter(entry => entry.isFile())
    .map(entry => entry.name)
    .filter(name => SUPPORTED_AVATAR_EXTENSIONS.has(name.slice(name.lastIndexOf('.')).toLowerCase()))
    .sort((left, right) => left.localeCompare(right))

  if (explicitAvatarName && avatarCandidates.includes(explicitAvatarName)) {
    return {
      fileName: explicitAvatarName,
      bytes: await readFile(join(templateDir, explicitAvatarName)),
      generated: false,
    }
  }

  if (avatarCandidates.length > 0) {
    const fileName = avatarCandidates[0]
    return {
      fileName,
      bytes: await readFile(join(templateDir, fileName)),
      generated: false,
    }
  }

  const library = avatarLibraries[ownerKind]
  const selected = library[deterministicIndex(`builtin:${ownerKind}:${sourceId}`, library.length)]
  return {
    fileName: selected.fileName,
    bytes: selected.bytes,
    generated: true,
  }
}

async function loadSkillDefinitions(config) {
  const skillsRoot = join(config.templatesRoot, 'skills')
  const entries = await readdir(skillsRoot, { withFileTypes: true })
  const definitions = []
  for (const entry of entries.sort((left, right) => left.name.localeCompare(right.name))) {
    if (!entry.isDirectory()) {
      continue
    }
    const sourceId = entry.name
    const root = join(skillsRoot, sourceId)
    const skillFile = join(root, 'SKILL.md')
    const contents = await readFile(skillFile, 'utf8')
    const { frontmatter } = splitFrontmatter(contents)
    const name = String(frontmatter.name ?? sourceId).trim()
    const description = typeof frontmatter.description === 'string'
      ? frontmatter.description.trim()
      : ''
    definitions.push({
      sourceId,
      root,
      name,
      description,
      requestedSlug: slugify(name, 'skill'),
      files: await collectFiles(root, root),
    })
  }

  const assigned = new Map()
  for (const definition of definitions) {
    let slug = definition.requestedSlug
    if (assigned.has(slug)) {
      slug = `${slug}-${shortHash(definition.sourceId)}`
    }
    assigned.set(slug, definition.sourceId)
    definition.slug = slug
  }
  return definitions
}

async function loadMcpDefinitions(config) {
  const mcpsRoot = join(config.templatesRoot, 'mcps')
  const entries = await readdir(mcpsRoot, { withFileTypes: true })
  const definitions = []
  for (const entry of entries.sort((left, right) => left.name.localeCompare(right.name))) {
    if (!entry.isFile() || !entry.name.endsWith('.json')) {
      continue
    }
    const serverName = entry.name.slice(0, -'.json'.length)
    const bytes = await readFile(join(mcpsRoot, entry.name))
    JSON.parse(bytes.toString('utf8'))
    definitions.push({
      serverName,
      fileName: entry.name,
      bytes,
    })
  }
  return definitions
}

async function readTemplateMarkdown(templateDir, baseName) {
  const markdownPath = join(templateDir, `${baseName}.md`)
  const contents = await readFile(markdownPath, 'utf8')
  const { frontmatter, body } = splitFrontmatter(contents)
  return { markdownPath, frontmatter, body }
}

async function materializeOwner({
  ownerKind,
  sourceId,
  directoryPath = sourceId,
  fileStem = sourceId,
  outputDir,
  templateDir,
  frontmatter,
  body,
  skillDefinitionsById,
  mcpDefinitionsByServer,
  avatarLibraries,
}) {
  const skillRefs = stableStringList(frontmatter.skills)
  const mcpRefs = stableStringList(frontmatter.mcps)

  for (const skillRef of skillRefs) {
    if (!skillDefinitionsById.has(skillRef)) {
      throw new Error(`${ownerKind} '${sourceId}' references missing skill '${skillRef}'`)
    }
  }
  for (const serverName of mcpRefs) {
    if (!mcpDefinitionsByServer.has(serverName)) {
      throw new Error(`${ownerKind} '${sourceId}' references missing MCP '${serverName}'`)
    }
  }

  const avatar = await resolveOwnerAvatar(
    templateDir,
    ownerKind,
    sourceId,
    typeof frontmatter.avatar === 'string' ? frontmatter.avatar.trim() : '',
    avatarLibraries,
  )
  const nextFrontmatter = { ...frontmatter, avatar: avatar.fileName }

  const files = [{
    relativePath: `${directoryPath}/${fileStem}.md`,
    bytes: Buffer.from(renderMarkdown(nextFrontmatter, body), 'utf8'),
  }, {
    relativePath: `${directoryPath}/${avatar.fileName}`,
    bytes: avatar.bytes,
  }]

  for (const skillRef of skillRefs) {
    const definition = skillDefinitionsById.get(skillRef)
    for (const file of definition.files) {
      files.push({
        relativePath: `${directoryPath}/skills/${skillRef}/${file.relativePath}`,
        bytes: file.bytes,
      })
    }
  }

  for (const serverName of mcpRefs) {
    const definition = mcpDefinitionsByServer.get(serverName)
    files.push({
      relativePath: `${directoryPath}/mcps/${definition.fileName}`,
      bytes: definition.bytes,
    })
  }

  await writeBundleFiles(outputDir, files)

  return {
    sourceId,
    name: String(frontmatter.name ?? sourceId).trim(),
    taskDomains: stableStringList(frontmatter.tag),
    avatar: avatar.fileName,
    generatedAvatar: avatar.generated,
    skillRefs,
    mcpRefs,
    files,
  }
}

async function buildStandaloneAgents(context) {
  const agentsRoot = join(context.config.templatesRoot, 'agents')
  const entries = await readdir(agentsRoot, { withFileTypes: true })
  const agents = []
  for (const entry of entries.sort((left, right) => left.name.localeCompare(right.name))) {
    if (!entry.isDirectory()) {
      continue
    }
    const sourceId = entry.name
    const templateDir = join(agentsRoot, sourceId)
    const { frontmatter, body } = await readTemplateMarkdown(templateDir, sourceId)
    const agent = await materializeOwner({
      ownerKind: 'agent',
      sourceId,
      outputDir: context.config.outputBundleRoot,
      templateDir,
      frontmatter,
      body,
      skillDefinitionsById: context.skillDefinitionsById,
      mcpDefinitionsByServer: context.mcpDefinitionsByServer,
      avatarLibraries: context.avatarLibraries,
    })
    agents.push({
      ...agent,
      bundlePath: `bundle/${sourceId}`,
    })
  }
  return agents
}

async function buildTeams(context) {
  const teamsRoot = join(context.config.templatesRoot, 'teams')
  const entries = await readdir(teamsRoot, { withFileTypes: true })
  const teams = []
  for (const entry of entries.sort((left, right) => left.name.localeCompare(right.name))) {
    if (!entry.isDirectory()) {
      continue
    }
    const sourceId = entry.name
    const templateDir = join(teamsRoot, sourceId)
    const { frontmatter, body } = await readTemplateMarkdown(templateDir, sourceId)
    const team = await materializeOwner({
      ownerKind: 'team',
      sourceId,
      outputDir: context.config.outputBundleRoot,
      templateDir,
      frontmatter,
      body,
      skillDefinitionsById: context.skillDefinitionsById,
      mcpDefinitionsByServer: context.mcpDefinitionsByServer,
      avatarLibraries: context.avatarLibraries,
    })

    const memberEntries = await readdir(templateDir, { withFileTypes: true })
    const memberIds = []
    for (const memberEntry of memberEntries.sort((left, right) => left.name.localeCompare(right.name))) {
      if (!memberEntry.isDirectory()) {
        continue
      }
      const memberId = memberEntry.name
      const memberDir = join(templateDir, memberId)
      const memberMarkdownPath = join(memberDir, `${memberId}.md`)
      const metadata = await stat(memberMarkdownPath).catch(() => null)
      if (!metadata?.isFile()) {
        continue
      }
      const { frontmatter: memberFrontmatter, body: memberBody } = await readTemplateMarkdown(memberDir, memberId)
      const member = await materializeOwner({
        ownerKind: 'agent',
        sourceId: `${sourceId}/${memberId}`,
        directoryPath: `${sourceId}/${memberId}`,
        fileStem: memberId,
        outputDir: context.config.outputBundleRoot,
        templateDir: memberDir,
        frontmatter: memberFrontmatter,
        body: memberBody,
        skillDefinitionsById: context.skillDefinitionsById,
        mcpDefinitionsByServer: context.mcpDefinitionsByServer,
        avatarLibraries: context.avatarLibraries,
      })
      memberIds.push(memberId)
    }

    teams.push({
      ...team,
      memberIds,
      bundlePath: `bundle/${sourceId}`,
    })
  }
  return teams
}

async function copyCanonicalSkillLibrary(skillDefinitions, config) {
  for (const definition of skillDefinitions) {
    for (const file of definition.files) {
      await writeBundleFiles(config.outputRoot, [{
        relativePath: `skills/${definition.slug}/${file.relativePath}`,
        bytes: file.bytes,
      }])
    }
  }
}

async function copyCanonicalMcpLibrary(mcpDefinitions, config) {
  for (const definition of mcpDefinitions) {
    await writeBundleFiles(config.outputRoot, [{
      relativePath: `mcps/${definition.fileName}`,
      bytes: definition.bytes,
    }])
  }
}

function buildManifest({ bundleRoot, agents, teams, skillDefinitions, mcpDefinitions, config }) {
  const assetPath = sourcePath => (bundleRoot === '.' ? sourcePath : `bundle/${sourcePath}`)
  return {
    version: 2,
    bundleKind: 'octopus-asset-bundle',
    bundleRoot,
    assets: [
      ...agents
      .map(agent => ({
        assetKind: 'agent',
        sourceId: agent.sourceId,
        displayName: agent.name,
        sourcePath: assetPath(`${agent.sourceId}/${agent.sourceId}.md`),
        manifestRevision: 'asset-manifest/v2',
        taskDomains: agent.taskDomains.length > 0 ? agent.taskDomains : ['finance'],
        translationMode: 'native',
      }))
      .sort((left, right) => left.sourceId.localeCompare(right.sourceId)),
      ...teams
      .map(team => ({
        assetKind: 'team',
        sourceId: team.sourceId,
        displayName: team.name,
        sourcePath: assetPath(`${team.sourceId}/${team.sourceId}.md`),
        manifestRevision: 'asset-manifest/v2',
        taskDomains: team.taskDomains.length > 0 ? team.taskDomains : ['finance'],
        translationMode: 'native',
      }))
      .sort((left, right) => left.sourceId.localeCompare(right.sourceId)),
      ...skillDefinitions
      .map(definition => ({
        assetKind: 'skill',
        sourceId: definition.sourceId,
        displayName: definition.name,
        sourcePath: `skills/${definition.slug}/SKILL.md`,
        manifestRevision: 'asset-manifest/v2',
        taskDomains: ['finance'],
        translationMode: 'native',
      }))
      .sort((left, right) => left.sourceId.localeCompare(right.sourceId)),
      ...mcpDefinitions
      .map(definition => ({
        assetKind: 'mcp-server',
        sourceId: definition.serverName,
        displayName: definition.serverName,
        sourcePath: `mcps/${definition.fileName}`,
        manifestRevision: 'asset-manifest/v2',
        taskDomains: [],
        translationMode: 'native',
      }))
      .sort((left, right) => left.sourceId.localeCompare(right.sourceId)),
    ],
    dependencies: [],
    trustMetadata: {
      publisher: 'octopus',
      origin: 'builtin-seed',
      signatureState: 'unsigned',
      trustLevel: 'trusted',
      trustWarnings: [],
    },
    compatibilityMapping: {
      supportedTargets: ['octopus'],
      downgradedFeatures: [],
      rejectedFeatures: [],
      translatorVersion: 'phase-1',
    },
    policyDefaults: {
      defaultModelStrategy: {
        selectionMode: 'session-selected',
        preferredModelRef: null,
        fallbackModelRefs: [],
        allowTurnOverride: true,
      },
      permissionEnvelope: {
        defaultMode: 'workspace-write',
        maxMode: 'workspace-write',
        escalationAllowed: false,
        allowedResourceScopes: ['agent-private', 'project-shared'],
      },
      memoryPolicy: {
        durableScopes: ['user-private', 'agent-private'],
        writeRequiresApproval: true,
        allowWorkspaceSharedWrite: false,
        maxSelections: 6,
        freshnessRequired: true,
      },
      delegationPolicy: {
        mode: 'leader-orchestrated',
        allowBackgroundRuns: true,
        allowParallelWorkers: true,
        maxWorkerCount: 4,
      },
      approvalPreference: {
        toolExecution: 'require-approval',
        mcpAuth: 'require-approval',
        memoryWrite: 'require-approval',
        teamSpawn: 'require-approval',
        workflowEscalation: 'require-approval',
      },
    },
    registryMetadata: {
      publisher: 'octopus',
      revision: 'builtin-seed',
      releaseChannel: 'stable',
      tags: ['finance', 'builtin'],
    },
  }
}

async function writeManifest(root, manifest) {
  await writeBundleFiles(root, [{
    relativePath: '.octopus/manifest.json',
    bytes: Buffer.from(`${JSON.stringify(manifest, null, 2)}\n`, 'utf8'),
  }])
}

async function syncExampleFromBundle(config) {
  await rm(config.exampleRoot, { recursive: true, force: true })
  await ensureDirectory(config.exampleRoot)
  const files = await collectFiles(config.outputBundleRoot, config.outputBundleRoot)
  await writeBundleFiles(config.exampleRoot, files)
}

async function main() {
  const config = resolveConfig()
  const avatarLibraries = {
    agent: await loadAvatarLibrary('agent', config),
    team: await loadAvatarLibrary('team', config),
  }
  const skillDefinitions = await loadSkillDefinitions(config)
  const skillDefinitionsById = new Map(skillDefinitions.map(item => [item.sourceId, item]))
  const mcpDefinitions = await loadMcpDefinitions(config)
  const mcpDefinitionsByServer = new Map(mcpDefinitions.map(item => [item.serverName, item]))

  await rm(config.outputRoot, { recursive: true, force: true })
  await ensureDirectory(config.outputRoot)
  await ensureDirectory(config.outputBundleRoot)

  const context = {
    config,
    avatarLibraries,
    skillDefinitionsById,
    mcpDefinitionsByServer,
  }

  const agents = await buildStandaloneAgents(context)
  const teams = await buildTeams(context)

  await copyCanonicalSkillLibrary(skillDefinitions, config)
  await copyCanonicalMcpLibrary(mcpDefinitions, config)

  const rootManifest = buildManifest({
    bundleRoot: 'bundle',
    agents,
    teams,
    skillDefinitions,
    mcpDefinitions,
    config,
  })
  const bundleManifest = buildManifest({
    bundleRoot: '.',
    agents,
    teams,
    skillDefinitions,
    mcpDefinitions,
    config,
  })

  await writeManifest(config.outputRoot, rootManifest)
  await writeManifest(config.outputBundleRoot, bundleManifest)

  await syncExampleFromBundle(config)
  await writeManifest(config.exampleRoot, bundleManifest)
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : error)
  process.exit(1)
})
