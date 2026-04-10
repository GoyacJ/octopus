import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import type {
  JsonValue,
  WorkspaceDirectoryUploadEntry,
  WorkspaceFileUploadPayload,
  WorkspaceMcpServerDocument,
  WorkspaceSkillDocument,
  WorkspaceSkillFileDocument,
  WorkspaceSkillTreeDocument,
  WorkspaceSkillTreeNode,
  WorkspaceToolCatalogEntry,
  WorkspaceToolKind,
} from '@octopus/schema'

import { usePagination } from '@/composables/usePagination'
import { useCatalogStore } from '@/stores/catalog'
import { useShellStore } from '@/stores/shell'
import { pickSkillArchive, pickSkillFolder } from '@/tauri/client'

export type DraftMode = 'none' | 'new-skill' | 'new-mcp'
export type PendingSkillAction = 'copy' | 'import' | null
export type PendingSkillImportSource = 'archive' | 'folder' | null
export type SkillCatalogEntry = Extract<WorkspaceToolCatalogEntry, { kind: 'skill' }>

export interface SkillTreeRow {
  path: string
  name: string
  kind: WorkspaceSkillTreeNode['kind']
  depth: number
  byteSize?: number
  isText?: boolean
}

export interface PendingSkillImportItem {
  id: string
  source: PendingSkillImportSource
  label: string
  slug: string
  archive?: WorkspaceFileUploadPayload
  files?: WorkspaceDirectoryUploadEntry[]
}

export interface PendingSkillCopyItem {
  skillId: string
  sourceName: string
  targetName: string
}

const DEFAULT_MCP_CONFIG = JSON.stringify({
  type: 'http',
  url: 'https://example.com/mcp',
}, null, 2)

function createSkillTemplate(slug = 'new-skill') {
  return [
    '---',
    `name: ${slug}`,
    'description: Describe what this skill is for.',
    '---',
    '',
    '# Overview',
    '',
    'Explain when to use this skill.',
  ].join('\n')
}

function flattenSkillTree(nodes: WorkspaceSkillTreeNode[], depth = 0): SkillTreeRow[] {
  return nodes.flatMap((node) => {
    const row: SkillTreeRow = {
      path: node.path,
      name: node.name,
      kind: node.kind,
      depth,
      byteSize: node.byteSize,
      isText: node.isText,
    }
    if (node.kind === 'directory') {
      return [row, ...flattenSkillTree(node.children ?? [], depth + 1)]
    }
    return [row]
  })
}

function firstFilePath(nodes: WorkspaceSkillTreeNode[]): string | null {
  for (const node of nodes) {
    if (node.kind === 'file' && node.path === 'SKILL.md') {
      return node.path
    }
    const nested = firstFilePath(node.children ?? [])
    if (nested === 'SKILL.md') {
      return nested
    }
  }

  for (const node of nodes) {
    if (node.kind === 'file') {
      return node.path
    }
    const nested = firstFilePath(node.children ?? [])
    if (nested) {
      return nested
    }
  }
  return null
}

export function useToolsView() {
  const { t } = useI18n()
  const catalogStore = useCatalogStore()
  const shell = useShellStore()

  const activeTab = ref<WorkspaceToolKind>('builtin')
  const searchQuery = ref('')
  const selectedEntryId = ref('')
  const selectedExternalSkillIds = ref<string[]>([])
  const draftMode = ref<DraftMode>('none')
  const loadingDetail = ref(false)
  const loadingSkillFile = ref(false)
  const submitting = ref(false)
  const deleting = ref(false)
  const toggling = ref(false)
  const panelError = ref('')

  const currentSkillDocument = ref<WorkspaceSkillDocument | null>(null)
  const currentSkillTree = ref<WorkspaceSkillTreeDocument | null>(null)
  const currentSkillFile = ref<WorkspaceSkillFileDocument | null>(null)
  const currentMcpDocument = ref<WorkspaceMcpServerDocument | null>(null)

  const selectedSkillFilePath = ref('')
  const skillFileDraft = ref('')
  const skillSlugDraft = ref('')
  const newSkillContentDraft = ref(createSkillTemplate())
  const mcpServerNameDraft = ref('')
  const mcpConfigDraft = ref(DEFAULT_MCP_CONFIG)
  const skillActionDialogOpen = ref(false)
  const pendingSkillAction = ref<PendingSkillAction>(null)
  const pendingSkillImportSource = ref<PendingSkillImportSource>(null)
  const pendingSkillImports = ref<PendingSkillImportItem[]>([])
  const pendingSkillCopies = ref<PendingSkillCopyItem[]>([])

  const tabOrder: WorkspaceToolKind[] = ['builtin', 'skill', 'mcp']

  const tabs = computed(() => tabOrder.map(kind => ({
    value: kind,
    label: t(`tools.tabs.${kind}`),
  })))

  const allEntries = computed(() => catalogStore.toolCatalogEntries)
  const activeTabEntries = computed(() => allEntries.value.filter(entry => entry.kind === activeTab.value))
  const activeTabCount = computed(() => activeTabEntries.value.length)
  const filteredEntries = computed(() => {
    const query = searchQuery.value.trim().toLowerCase()
    return activeTabEntries.value.filter((entry) => {
      if (!query) {
        return true
      }

      const haystack = [
        entry.name,
        entry.description,
        entry.displayPath,
        entry.sourceKey,
        entry.ownerLabel ?? '',
        ...(entry.consumers?.map(consumer => consumer.name) ?? []),
        entry.kind,
        entry.availability,
        entry.requiredPermission ?? '',
        entry.disabled ? 'disabled' : '',
        entry.kind === 'builtin' ? entry.builtinKey : '',
        entry.kind === 'skill' ? entry.shadowedBy ?? '' : '',
        entry.kind === 'skill' ? entry.sourceOrigin : '',
        entry.kind === 'skill' ? entry.relativePath ?? '' : '',
        entry.kind === 'mcp' ? entry.serverName : '',
        entry.kind === 'mcp' ? entry.endpoint : '',
        entry.kind === 'mcp' ? entry.toolNames.join(' ') : '',
        entry.kind === 'mcp' ? entry.statusDetail ?? '' : '',
        entry.kind === 'mcp' ? entry.scope : '',
        entry.ownerScope ?? '',
      ]
        .join(' ')
        .toLowerCase()

      return haystack.includes(query)
    })
  })
  const listPagination = usePagination(filteredEntries, {
    pageSize: 6,
    resetOn: [activeTab, searchQuery],
  })
  const pagedEntries = computed(() => listPagination.pagedItems.value)
  const listPage = computed(() => listPagination.currentPage.value)
  const listPageCount = computed(() => listPagination.pageCount.value)

  const selectedEntry = computed(() =>
    filteredEntries.value.find(entry => entry.id === selectedEntryId.value) ?? filteredEntries.value[0] ?? null,
  )

  const selectedSkillEntry = computed(() =>
    draftMode.value === 'none' && selectedEntry.value?.kind === 'skill' ? selectedEntry.value : null,
  )

  const selectedMcpEntry = computed(() =>
    draftMode.value === 'none' && selectedEntry.value?.kind === 'mcp' ? selectedEntry.value : null,
  )
  const selectableExternalSkillEntries = computed(() =>
    filteredEntries.value.filter(isExternalSkillEntry),
  )
  const selectedExternalSkillEntries = computed(() => {
    const selectedIds = new Set(selectedExternalSkillIds.value)
    return selectableExternalSkillEntries.value.filter(entry => selectedIds.has(entry.id))
  })

  const selectedSkillTreeRows = computed(() => flattenSkillTree(currentSkillTree.value?.tree ?? []))
  const canSaveSkillFile = computed(() =>
    Boolean(
      selectedSkillEntry.value?.management.canEdit
      && currentSkillFile.value?.isText
      && !currentSkillFile.value?.readonly,
    ),
  )
  const canCopySkillToManaged = computed(() =>
    Boolean(selectedSkillEntry.value && !selectedSkillEntry.value.workspaceOwned),
  )
  const canCopyMcpToManaged = computed(() =>
    Boolean(selectedMcpEntry.value && selectedMcpEntry.value.scope === 'builtin'),
  )
  const canCopySelectedSkillsToManaged = computed(() => selectedExternalSkillEntries.value.length > 0)
  const pendingSkillActionTitle = computed(() =>
    pendingSkillAction.value === 'copy'
      ? t('tools.actions.copyToManaged')
      : t('tools.actions.importSkill'),
  )
  const pendingSkillActionDescription = computed(() => {
    switch (pendingSkillAction.value) {
      case 'copy':
        return t('tools.editor.copySkillDescription')
      case 'import':
        if (pendingSkillImportSource.value === 'archive') {
          return t('tools.editor.importArchiveDescription')
        }
        if (pendingSkillImportSource.value === 'folder') {
          return t('tools.editor.importFolderDescription')
        }
        return t('tools.editor.importSkillDescription')
      default:
        return ''
    }
  })
  const pendingSkillSelectionLabel = computed(() => {
    if (pendingSkillAction.value === 'copy') {
      return pendingSkillCopies.value.map(item => item.sourceName).join('、')
    }
    return pendingSkillImports.value.map(item => item.label).join('、')
  })
  const pendingSkillImportTargets = computed(() => pendingSkillImports.value.map(item => ({
    id: item.id,
    label: item.label,
    slug: item.slug,
  })))
  const pendingSkillActionReady = computed(() => {
    if (pendingSkillAction.value === 'copy') {
      return Boolean(pendingSkillCopies.value.length)
    }
    return Boolean(pendingSkillImports.value.length)
  })

  watch(
    () => shell.activeWorkspaceConnectionId,
    (connectionId) => {
      if (connectionId) {
        void catalogStore.load(connectionId)
      }
    },
    { immediate: true },
  )

  watch(filteredEntries, (entries) => {
    if (!entries.length) {
      selectedEntryId.value = ''
      selectedExternalSkillIds.value = []
      return
    }
    if (!entries.some(entry => entry.id === selectedEntryId.value)) {
      selectedEntryId.value = entries[0].id
    }
    const visibleExternalSkillIds = new Set(entries.filter(isExternalSkillEntry).map(entry => entry.id))
    selectedExternalSkillIds.value = selectedExternalSkillIds.value.filter(id => visibleExternalSkillIds.has(id))
  }, { immediate: true })

  watch(activeTab, () => {
    draftMode.value = 'none'
    panelError.value = ''
    resetPendingSkillAction()
  })

  watch(skillActionDialogOpen, (open) => {
    if (!open) {
      panelError.value = ''
      resetPendingSkillAction()
    }
  })

  watch(
    () => [draftMode.value, selectedEntry.value?.id],
    async (_, __, onCleanup) => {
      let cancelled = false
      onCleanup(() => {
        cancelled = true
      })

      const entry = selectedEntry.value
      panelError.value = ''
      currentSkillDocument.value = null
      currentSkillTree.value = null
      currentSkillFile.value = null
      currentMcpDocument.value = null
      selectedSkillFilePath.value = ''
      skillFileDraft.value = ''

      if (draftMode.value !== 'none' || !entry) {
        loadingDetail.value = false
        return
      }

      if (entry.kind === 'builtin') {
        loadingDetail.value = false
        return
      }

      loadingDetail.value = true
      try {
        if (entry.kind === 'skill') {
          const [document, tree] = await Promise.all([
            catalogStore.getSkillDocument(entry.id),
            catalogStore.getSkillTreeDocument(entry.id),
          ])
          if (cancelled) {
            return
          }

          currentSkillDocument.value = document
          currentSkillTree.value = tree
          const initialPath = firstFilePath(tree.tree)
          selectedSkillFilePath.value = initialPath ?? ''
          return
        }

        if (entry.management.canEdit) {
          const document = await catalogStore.getMcpServerDocument(entry.serverName)
          if (cancelled) {
            return
          }
          currentMcpDocument.value = document
          mcpServerNameDraft.value = document.serverName
          mcpConfigDraft.value = JSON.stringify(document.config, null, 2)
          return
        }

        mcpServerNameDraft.value = entry.serverName
        mcpConfigDraft.value = ''
      } catch (error) {
        if (!cancelled) {
          panelError.value = toErrorMessage(error)
        }
      } finally {
        if (!cancelled) {
          loadingDetail.value = false
        }
      }
    },
    { immediate: true },
  )

  watch(
    () => [selectedSkillEntry.value?.id, selectedSkillFilePath.value, draftMode.value],
    async (_, __, onCleanup) => {
      let cancelled = false
      onCleanup(() => {
        cancelled = true
      })

      const entry = selectedSkillEntry.value
      if (draftMode.value !== 'none' || !entry || !selectedSkillFilePath.value) {
        currentSkillFile.value = null
        loadingSkillFile.value = false
        return
      }

      loadingSkillFile.value = true
      try {
        const document = await catalogStore.getSkillFileDocument(entry.id, selectedSkillFilePath.value)
        if (cancelled) {
          return
        }
        currentSkillFile.value = document
        skillFileDraft.value = document.content ?? ''
      } catch (error) {
        if (!cancelled) {
          panelError.value = toErrorMessage(error)
        }
      } finally {
        if (!cancelled) {
          loadingSkillFile.value = false
        }
      }
    },
    { immediate: true },
  )

  function toErrorMessage(error: unknown) {
    return error instanceof Error ? error.message : t('tools.editor.unknownError')
  }

  function availabilityTone(availability: WorkspaceToolCatalogEntry['availability']) {
    switch (availability) {
      case 'healthy':
        return 'success'
      case 'attention':
        return 'warning'
      default:
        return 'default'
    }
  }

  function kindLabel(kind: WorkspaceToolKind) {
    return t(`tools.tabs.${kind}`)
  }

  function availabilityLabel(availability: WorkspaceToolCatalogEntry['availability']) {
    return t(`tools.availability.${availability}`)
  }

  function permissionLabel(permission: WorkspaceToolCatalogEntry['requiredPermission']) {
    if (!permission) {
      return t('common.na')
    }
    return t(`tools.requiredPermissions.${permission}`)
  }

  function ownerScopeLabel(ownerScope: WorkspaceToolCatalogEntry['ownerScope']) {
    if (!ownerScope) {
      return t('common.na')
    }
    const translationKey = `tools.ownerScopes.${ownerScope}`
    const translated = t(translationKey)
    return translated === translationKey ? ownerScope : translated
  }

  function sourceOriginLabel(entry: Extract<WorkspaceToolCatalogEntry, { kind: 'skill' }>) {
    return t(`tools.sourceOrigins.${entry.sourceOrigin}`)
  }

  function skillStateLabel(entry: Extract<WorkspaceToolCatalogEntry, { kind: 'skill' }>) {
    return entry.active ? t('tools.states.active') : t('tools.states.shadowed')
  }

  function fileTypeLabel(file: WorkspaceSkillFileDocument | null) {
    if (!file) {
      return ''
    }
    return file.isText ? t('tools.editor.textFile') : t('tools.editor.binaryFile')
  }

  function isExternalSkillEntry(entry: WorkspaceToolCatalogEntry): entry is SkillCatalogEntry {
    return entry.kind === 'skill' && !entry.workspaceOwned
  }

  function skillDisplayPath(skillId: string) {
    return allEntries.value.find(entry => entry.id === skillId)?.displayPath ?? ''
  }

  function beginNewSkill() {
    draftMode.value = 'new-skill'
    panelError.value = ''
    skillSlugDraft.value = 'new-skill'
    newSkillContentDraft.value = createSkillTemplate(skillSlugDraft.value)
  }

  function beginNewMcp() {
    draftMode.value = 'new-mcp'
    panelError.value = ''
    mcpServerNameDraft.value = 'new-server'
    mcpConfigDraft.value = DEFAULT_MCP_CONFIG
  }

  async function saveCurrent() {
    panelError.value = ''
    submitting.value = true
    try {
      if (draftMode.value === 'new-skill') {
        const document = await catalogStore.createSkill({
          slug: skillSlugDraft.value.trim(),
          content: newSkillContentDraft.value,
        })
        draftMode.value = 'none'
        selectedEntryId.value = catalogStore.toolCatalogEntries.find(entry => entry.sourceKey === document.sourceKey)?.id ?? document.id
        return
      }

      if (draftMode.value === 'new-mcp') {
        const document = await catalogStore.createMcpServer({
          serverName: mcpServerNameDraft.value.trim(),
          config: parseJsonObjectDraft(mcpConfigDraft.value),
        })
        draftMode.value = 'none'
        selectedEntryId.value = catalogStore.toolCatalogEntries.find(entry => entry.sourceKey === document.sourceKey)?.id ?? ''
        return
      }

      if (selectedSkillEntry.value && canSaveSkillFile.value && currentSkillFile.value) {
        const document = await catalogStore.updateSkillFile(selectedSkillEntry.value.id, currentSkillFile.value.path, {
          content: skillFileDraft.value,
        })
        currentSkillFile.value = document
        skillFileDraft.value = document.content ?? ''
        currentSkillDocument.value = await catalogStore.getSkillDocument(selectedSkillEntry.value.id)
        currentSkillTree.value = await catalogStore.getSkillTreeDocument(selectedSkillEntry.value.id)
        return
      }

      if (selectedMcpEntry.value?.management.canEdit) {
        await catalogStore.updateMcpServer(selectedMcpEntry.value.serverName, {
          serverName: mcpServerNameDraft.value.trim(),
          config: parseJsonObjectDraft(mcpConfigDraft.value),
        })
      }
    } catch (error) {
      panelError.value = toErrorMessage(error)
    } finally {
      submitting.value = false
    }
  }

  async function deleteCurrent() {
    panelError.value = ''
    deleting.value = true
    try {
      if (selectedSkillEntry.value?.management.canDelete) {
        const removedId = selectedSkillEntry.value.id
        await catalogStore.deleteSkill(removedId)
        if (selectedEntryId.value === removedId) {
          selectedEntryId.value = ''
        }
        return
      }

      if (selectedMcpEntry.value?.management.canDelete) {
        const removedServerName = selectedMcpEntry.value.serverName
        await catalogStore.deleteMcpServer(removedServerName)
        if (selectedMcpEntry.value?.serverName === removedServerName) {
          selectedEntryId.value = ''
        }
      }
    } catch (error) {
      panelError.value = toErrorMessage(error)
    } finally {
      deleting.value = false
    }
  }

  async function toggleDisabled(entry: WorkspaceToolCatalogEntry, disabled: boolean) {
    panelError.value = ''
    toggling.value = true
    try {
      await catalogStore.setToolDisabled({
        sourceKey: entry.sourceKey,
        disabled,
      })
    } catch (error) {
      panelError.value = toErrorMessage(error)
    } finally {
      toggling.value = false
    }
  }

  function openImportSkillDialog() {
    panelError.value = ''
    draftMode.value = 'none'
    pendingSkillAction.value = 'import'
    pendingSkillImportSource.value = null
    skillActionDialogOpen.value = true
  }

  async function importArchiveSkill() {
    panelError.value = ''
    const archives = await pickSkillArchive()
    if (!archives?.length) {
      return
    }
    pendingSkillAction.value = 'import'
    pendingSkillImportSource.value = 'archive'
    pendingSkillImports.value = [
      ...pendingSkillImports.value,
      ...archives.map((archive, index) => ({
        id: `archive-${archive.fileName}-${index}`,
        source: 'archive' as const,
        label: archive.fileName,
        slug: suggestSlug(archive.fileName),
        archive,
      })),
    ]
    skillActionDialogOpen.value = true
  }

  async function importFolderSkill() {
    panelError.value = ''
    const folderGroups = await pickSkillFolder()
    if (!folderGroups?.length) {
      return
    }
    pendingSkillAction.value = 'import'
    pendingSkillImportSource.value = 'folder'
    pendingSkillImports.value = [
      ...pendingSkillImports.value,
      ...folderGroups.map((files, index) => {
        const label = folderLabelFromFiles(files)
        return {
          id: `folder-${label}-${index}`,
          source: 'folder' as const,
          label,
          slug: suggestSlugFromFolder(files),
          files,
        }
      }),
    ]
    skillActionDialogOpen.value = true
  }

  async function copySelectedSkillToManaged() {
    if (!selectedSkillEntry.value || !isExternalSkillEntry(selectedSkillEntry.value)) {
      return
    }

    openSkillCopyDialog([selectedSkillEntry.value])
  }

  async function copySelectedMcpToManaged() {
    if (!selectedMcpEntry.value || selectedMcpEntry.value.scope !== 'builtin') {
      return
    }

    panelError.value = ''
    submitting.value = true
    try {
      const document = await catalogStore.copyMcpServerToManaged(selectedMcpEntry.value.serverName)
      selectedEntryId.value = catalogStore.toolCatalogEntries
        .find(entry => entry.kind === 'mcp' && entry.serverName === document.serverName)?.id ?? ''
    } catch (error) {
      panelError.value = toErrorMessage(error)
    } finally {
      submitting.value = false
    }
  }

  function copySelectedSkillsToManaged() {
    if (!selectedExternalSkillEntries.value.length) {
      return
    }

    openSkillCopyDialog(selectedExternalSkillEntries.value)
  }

  function openSkillCopyDialog(entries: SkillCatalogEntry[]) {
    panelError.value = ''
    pendingSkillImportSource.value = null
    pendingSkillAction.value = 'copy'
    pendingSkillCopies.value = entries.map(entry => ({
      skillId: entry.id,
      sourceName: entry.name,
      targetName: resolveSkillTargetSlug(entry),
    }))
    skillActionDialogOpen.value = true
  }

  function selectEntry(entryId: string) {
    draftMode.value = 'none'
    resetPendingSkillAction()
    selectedEntryId.value = entryId
  }

  function selectSkillFile(path: string) {
    if (selectedSkillFilePath.value === path) {
      return
    }
    selectedSkillFilePath.value = path
  }

  function resetPendingSkillAction() {
    pendingSkillAction.value = null
    pendingSkillImportSource.value = null
    pendingSkillImports.value = []
    pendingSkillCopies.value = []
  }

  async function submitPendingSkillAction() {
    if (!pendingSkillAction.value) {
      return
    }

    panelError.value = ''
    submitting.value = true
    try {
      if (pendingSkillAction.value === 'copy') {
        validatePendingSkillCopies(pendingSkillCopies.value)
        let document: WorkspaceSkillDocument | null = null
        for (const item of pendingSkillCopies.value) {
          document = await catalogStore.copySkillToManaged(item.skillId, {
            slug: suggestSlug(item.targetName),
          })
        }
        skillActionDialogOpen.value = false
        selectedExternalSkillIds.value = selectedExternalSkillIds.value
          .filter(id => !pendingSkillCopies.value.some(item => item.skillId === id))
        resetPendingSkillAction()
        if (document) {
          selectedEntryId.value = catalogStore.toolCatalogEntries.find(entry => entry.sourceKey === document.sourceKey)?.id ?? document.id
        }
        return
      }

      if (pendingSkillAction.value === 'import') {
        validatePendingSkillImports(pendingSkillImports.value)
        let document: WorkspaceSkillDocument | null = null
        for (const item of pendingSkillImports.value) {
          if (item.source === 'archive' && item.archive) {
            document = await catalogStore.importSkillArchive({
              slug: item.slug,
              archive: item.archive,
            })
            continue
          }
          if (item.source === 'folder' && item.files) {
            document = await catalogStore.importSkillFolder({
              slug: item.slug,
              files: item.files,
            })
          }
        }
        skillActionDialogOpen.value = false
        resetPendingSkillAction()
        activeTab.value = 'skill'
        draftMode.value = 'none'
        if (document) {
          selectedEntryId.value = catalogStore.toolCatalogEntries.find(entry => entry.sourceKey === document.sourceKey)?.id ?? document.id
        }
      }
    } catch (error) {
      panelError.value = toErrorMessage(error)
    } finally {
      submitting.value = false
    }
  }

  function validatePendingSkillCopies(items: PendingSkillCopyItem[]) {
    if (!items.length) {
      throw new Error(t('tools.editor.skillNameRequired'))
    }
    const seen = new Set<string>()
    for (const item of items) {
      const slug = suggestSlug(item.targetName)
      if (!slug) {
        throw new Error(t('tools.editor.skillNameRequired'))
      }
      if (seen.has(slug)) {
        throw new Error(t('tools.editor.skillCopyDuplicate'))
      }
      seen.add(slug)
    }
  }

  function validatePendingSkillImports(items: PendingSkillImportItem[]) {
    if (!items.length) {
      throw new Error(t('tools.editor.skillImportRequired'))
    }
    const seen = new Set<string>()
    for (const item of items) {
      if (!item.slug.trim()) {
        throw new Error(t('tools.editor.skillSlugRequired'))
      }
      if (seen.has(item.slug)) {
        throw new Error(t('tools.editor.skillImportDuplicate'))
      }
      seen.add(item.slug)
    }
  }

  function suggestSlug(value: string) {
    return (value || '')
      .replace(/\.zip$/i, '')
      .trim()
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-+|-+$/g, '')
      || 'new-skill'
  }

  function suggestSlugFromFolder(files: { relativePath: string }[]) {
    const firstSegment = folderLabelFromFiles(files)
    if (firstSegment.toLowerCase() === 'skill.md') {
      return 'new-skill'
    }
    return suggestSlug(firstSegment)
  }

  function folderLabelFromFiles(files: { relativePath: string }[]) {
    return files[0]?.relativePath.split('/')[0] ?? 'new-skill'
  }

  function resolveSkillTargetSlug(entry: Extract<WorkspaceToolCatalogEntry, { kind: 'skill' }>) {
    const relativePath = entry.relativePath ?? ''
    const segments = relativePath.split('/').filter(Boolean)
    const baseName = segments.at(-2) || entry.name
    return suggestSlug(baseName)
  }

  function parseJsonObjectDraft(value: string): Record<string, JsonValue> {
    const trimmed = value.trim() || '{}'
    const parsed = JSON.parse(trimmed) as JsonValue
    if (!parsed || Array.isArray(parsed) || typeof parsed !== 'object') {
      throw new Error(t('tools.editor.jsonObjectRequired'))
    }
    return parsed as Record<string, JsonValue>
  }

  return {
    t,
    catalogStore,
    activeTab,
    searchQuery,
    selectedEntryId,
    selectedExternalSkillIds,
    draftMode,
    loadingDetail,
    loadingSkillFile,
    submitting,
    deleting,
    toggling,
    panelError,
    currentSkillDocument,
    currentSkillTree,
    currentSkillFile,
    currentMcpDocument,
    selectedSkillFilePath,
    skillFileDraft,
    skillSlugDraft,
    newSkillContentDraft,
    mcpServerNameDraft,
    mcpConfigDraft,
    skillActionDialogOpen,
    pendingSkillAction,
    pendingSkillImportSource,
    pendingSkillImports,
    pendingSkillCopies,
    tabs,
    allEntries,
    activeTabEntries,
    activeTabCount,
    filteredEntries,
    pagedEntries,
    listPage,
    listPageCount,
    selectedEntry,
    selectedSkillEntry,
    selectedMcpEntry,
    selectableExternalSkillEntries,
    selectedExternalSkillEntries,
    selectedSkillTreeRows,
    canSaveSkillFile,
    canCopySkillToManaged,
    canCopyMcpToManaged,
    canCopySelectedSkillsToManaged,
    pendingSkillActionTitle,
    pendingSkillActionDescription,
    pendingSkillSelectionLabel,
    pendingSkillImportTargets,
    pendingSkillActionReady,
    availabilityTone,
    kindLabel,
    availabilityLabel,
    permissionLabel,
    ownerScopeLabel,
    sourceOriginLabel,
    skillStateLabel,
    fileTypeLabel,
    isExternalSkillEntry,
    skillDisplayPath,
    beginNewSkill,
    beginNewMcp,
    saveCurrent,
    deleteCurrent,
    toggleDisabled,
    openImportSkillDialog,
    importArchiveSkill,
    importFolderSkill,
    copySelectedSkillToManaged,
    copySelectedMcpToManaged,
    copySelectedSkillsToManaged,
    selectEntry,
    listPagination,
    selectSkillFile,
    submitPendingSkillAction,
    suggestSlug,
  }
}
