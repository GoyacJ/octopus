import type {
  JsonValue,
  WorkspaceDirectoryUploadEntry,
  WorkspaceMcpServerDocument,
  WorkspaceSkillDocument,
  WorkspaceSkillFileDocument,
  WorkspaceToolConsumerSummary,
  WorkspaceSkillTreeNode,
  WorkspaceToolCatalogEntry,
} from '@octopus/schema'

function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T
}

export function createManagementCapabilities(canDisable: boolean, canEdit: boolean, canDelete: boolean) {
  return {
    canDisable,
    canEdit,
    canDelete,
  }
}

export function skillSlugFromRelativePath(relativePath?: string): string {
  if (!relativePath) {
    return ''
  }
  const match = relativePath.match(/^(?:data\/skills|\.codex\/skills|\.claude\/skills)\/([^/]+)\/SKILL\.md$/)
  return match?.[1] ?? ''
}

export function skillNameFromContent(content: string, fallback: string): string {
  const nameLine = content
    .split(/\r?\n/)
    .find(line => line.trimStart().startsWith('name:'))
  return nameLine?.split(':').slice(1).join(':').trim() || fallback
}

export function skillDescriptionFromContent(content: string, fallback: string): string {
  const descriptionLine = content
    .split(/\r?\n/)
    .find(line => line.trimStart().startsWith('description:'))
  return descriptionLine?.split(':').slice(1).join(':').trim() || fallback
}

export function createSkillTemplate(slug: string) {
  return [
    '---',
    `name: ${slug}`,
    `description: Workspace skill ${slug}.`,
    '---',
    '',
    '# Overview',
  ].join('\n')
}

function createSkillTree(paths: string[]): WorkspaceSkillTreeNode[] {
  const root: WorkspaceSkillTreeNode[] = []

  for (const path of paths) {
    const segments = path.split('/').filter(Boolean)
    let currentNodes = root
    let currentPath = ''

    for (const [index, segment] of segments.entries()) {
      currentPath = currentPath ? `${currentPath}/${segment}` : segment
      const isFile = index === segments.length - 1
      let node = currentNodes.find(item => item.name === segment)
      if (!node) {
        node = isFile
          ? {
              path: currentPath,
              name: segment,
              kind: 'file',
              byteSize: 0,
              isText: true,
            }
          : {
              path: currentPath,
              name: segment,
              kind: 'directory',
              children: [],
            }
        currentNodes.push(node)
      }

      if (!isFile) {
        node.children ||= []
        currentNodes = node.children
      }
    }
  }

  const sortNodes = (nodes: WorkspaceSkillTreeNode[]): WorkspaceSkillTreeNode[] => nodes
    .map(node => node.kind === 'directory' && node.children
      ? { ...node, children: sortNodes(node.children) }
      : node)
    .sort((left, right) => {
      if (left.kind !== right.kind) {
        return left.kind === 'directory' ? -1 : 1
      }
      return left.path.localeCompare(right.path)
    })

  return sortNodes(root)
}

export function createSkillDocument(
  id: string,
  sourceKey: string,
  name: string,
  description: string,
  displayPath: string,
  workspaceOwned: boolean,
  files: Record<string, WorkspaceSkillFileDocument>,
  relativePath?: string,
): WorkspaceSkillDocument {
  const rootPath = displayPath.replace(/\/SKILL\.md$/, '')
  const content = files['SKILL.md']?.content ?? ''
  return {
    id,
    sourceKey,
    name,
    description,
    content,
    displayPath,
    rootPath,
    tree: createSkillTree(Object.keys(files)),
    sourceOrigin: 'skills_dir',
    workspaceOwned,
    relativePath,
  }
}

function inferContentType(path: string, isText: boolean) {
  if (!isText) {
    return 'application/octet-stream'
  }
  if (path.endsWith('.md')) {
    return 'text/markdown'
  }
  if (path.endsWith('.json')) {
    return 'application/json'
  }
  if (path.endsWith('.txt')) {
    return 'text/plain'
  }
  return 'text/plain'
}

function inferLanguage(path: string) {
  if (path.endsWith('.md')) {
    return 'markdown'
  }
  if (path.endsWith('.json')) {
    return 'json'
  }
  if (path.endsWith('.ts')) {
    return 'typescript'
  }
  if (path.endsWith('.js')) {
    return 'javascript'
  }
  if (path.endsWith('.yml') || path.endsWith('.yaml')) {
    return 'yaml'
  }
  if (path.endsWith('.txt')) {
    return 'text'
  }
  return undefined
}

export function createSkillFileDocument(
  skillId: string,
  sourceKey: string,
  rootPath: string,
  path: string,
  options: {
    content?: string
    isText?: boolean
    readonly?: boolean
    contentType?: string
    language?: string
    byteSize?: number
  } = {},
): WorkspaceSkillFileDocument {
  const isText = options.isText ?? true
  const content = isText ? (options.content ?? '') : null
  return {
    skillId,
    sourceKey,
    path,
    displayPath: `${rootPath}/${path}`,
    byteSize: options.byteSize ?? (content?.length ?? 0),
    isText,
    content,
    contentType: options.contentType ?? inferContentType(path, isText),
    language: options.language ?? inferLanguage(path),
    readonly: options.readonly ?? false,
  }
}

export function cloneSkillFiles(files: Record<string, WorkspaceSkillFileDocument>) {
  return Object.fromEntries(Object.entries(files).map(([path, document]) => [path, clone(document)]))
}

export function createSkillAsset(input: {
  id: string
  sourceKey: string
  name: string
  description: string
  displayPath: string
  workspaceOwned: boolean
  files: Record<string, WorkspaceSkillFileDocument>
  relativePath?: string
}): { document: WorkspaceSkillDocument, files: Record<string, WorkspaceSkillFileDocument> } {
  const document = createSkillDocument(
    input.id,
    input.sourceKey,
    input.name,
    input.description,
    input.displayPath,
    input.workspaceOwned,
    input.files,
    input.relativePath,
  )
  return {
    document,
    files: cloneSkillFiles(input.files),
  }
}

export function createImportedSkillFiles(
  skillId: string,
  sourceKey: string,
  rootPath: string,
  slug: string,
  uploadedFiles?: WorkspaceDirectoryUploadEntry[],
) {
  const normalizedFiles = normalizeImportedFiles(uploadedFiles)
  if (!normalizedFiles?.length) {
    return {
      'SKILL.md': createSkillFileDocument(skillId, sourceKey, rootPath, 'SKILL.md', {
        content: [
          '---',
          `name: ${slug}`,
          `description: Imported ${slug} skill.`,
          '---',
          '',
          '# Overview',
        ].join('\n'),
      }),
    }
  }

  return Object.fromEntries(normalizedFiles.map((file) => {
    const isText = file.contentType.startsWith('text/')
      || /\.(md|txt|json|ya?ml|ts|js|mts|cts)$/i.test(file.relativePath)
    const decoded = isText ? atob(file.dataBase64) : undefined
    const content = file.relativePath === 'SKILL.md' && decoded
      ? normalizeSkillFrontmatterName(decoded, slug)
      : decoded
    return [file.relativePath, createSkillFileDocument(skillId, sourceKey, rootPath, file.relativePath, {
      content,
      isText,
      byteSize: file.byteSize,
      contentType: file.contentType,
    })]
  }))
}

export function normalizeSkillFrontmatterName(content: string, slug: string) {
  const endsWithNewline = content.endsWith('\n')
  const lines = content.replace(/\r\n/g, '\n').split('\n')
  if (endsWithNewline && lines.at(-1) === '') {
    lines.pop()
  }
  if (lines[0]?.trim() !== '---') {
    return content
  }

  const closingIndex = lines.findIndex((line, index) => index > 0 && line.trim() === '---')
  if (closingIndex === -1) {
    return content
  }

  const nameIndex = lines.findIndex((line, index) => index > 0 && index < closingIndex && line.trimStart().startsWith('name:'))
  const normalizedName = `name: ${slug}`
  if (nameIndex >= 0) {
    lines[nameIndex] = normalizedName
  } else {
    lines.splice(closingIndex, 0, normalizedName)
  }

  const updated = lines.join('\n')
  return endsWithNewline ? `${updated}\n` : updated
}

export function normalizeImportedFiles(files?: WorkspaceDirectoryUploadEntry[]) {
  if (!files?.length) {
    return null
  }

  const normalized = files.map((file) => ({
    ...file,
    relativePath: file.relativePath.replace(/^\/+/, '').replace(/\\/g, '/'),
  }))
  const direct = normalized.some(file => file.relativePath === 'SKILL.md')
  if (direct) {
    return normalized
  }

  const firstSegments = new Set(normalized.map(file => file.relativePath.split('/')[0]).filter(Boolean))
  if (firstSegments.size !== 1) {
    return normalized
  }

  const prefix = [...firstSegments][0]
  return normalized.map((file) => ({
    ...file,
    relativePath: file.relativePath.startsWith(`${prefix}/`)
      ? file.relativePath.slice(prefix.length + 1)
      : file.relativePath,
  }))
}

function mcpEndpointFromConfig(config: Record<string, JsonValue>): string {
  const url = config.url
  if (typeof url === 'string' && url) {
    return url
  }

  const command = config.command
  if (typeof command === 'string' && command) {
    return command
  }

  return 'configured'
}

export function createSkillCatalogEntry(
  workspaceId: string,
  document: WorkspaceSkillDocument,
  disabled = false,
  options: {
    management?: ReturnType<typeof createManagementCapabilities>
    ownerScope?: 'builtin' | 'workspace' | 'project' | string
    ownerId?: string
    ownerLabel?: string
    consumers?: WorkspaceToolConsumerSummary[]
  } = {},
): WorkspaceToolCatalogEntry {
  return {
    id: document.id,
    workspaceId,
    kind: 'skill',
    name: document.name,
    description: document.description,
    availability: 'healthy',
    requiredPermission: null,
    sourceKey: document.sourceKey,
    displayPath: document.displayPath,
    disabled,
    management: options.management ?? createManagementCapabilities(true, document.workspaceOwned, document.workspaceOwned),
    active: true,
    sourceOrigin: document.sourceOrigin,
    workspaceOwned: document.workspaceOwned,
    relativePath: document.relativePath,
    ownerScope: options.ownerScope,
    ownerId: options.ownerId,
    ownerLabel: options.ownerLabel,
    consumers: options.consumers,
  }
}

export function createMcpCatalogEntry(
  workspaceId: string,
  document: WorkspaceMcpServerDocument,
  disabled = false,
  options: {
    management?: ReturnType<typeof createManagementCapabilities>
    ownerScope?: 'builtin' | 'workspace' | 'project' | string
    ownerId?: string
    ownerLabel?: string
    consumers?: WorkspaceToolConsumerSummary[]
    toolNames?: string[]
    statusDetail?: string
    description?: string
  } = {},
): WorkspaceToolCatalogEntry {
  return {
    id: `mcp-${document.serverName}`,
    workspaceId,
    kind: 'mcp',
    name: document.serverName,
    description: options.description ?? 'Configured MCP server.',
    availability: 'configured',
    requiredPermission: null,
    sourceKey: document.sourceKey,
    displayPath: document.displayPath,
    disabled,
    management: options.management ?? createManagementCapabilities(true, true, true),
    serverName: document.serverName,
    endpoint: mcpEndpointFromConfig(document.config),
    toolNames: options.toolNames ?? [],
    scope: document.scope,
    statusDetail: options.statusDetail,
    ownerScope: options.ownerScope,
    ownerId: options.ownerId,
    ownerLabel: options.ownerLabel,
    consumers: options.consumers,
  }
}
