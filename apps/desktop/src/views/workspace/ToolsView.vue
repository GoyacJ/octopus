<script setup lang="ts">
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
import {
  UiBadge,
  UiButton,
  UiCheckbox,
  UiCodeEditor,
  UiDialog,
  UiEmptyState,
  UiField,
  UiInput,
  UiRecordCard,
  UiSectionHeading,
  UiSwitch,
  UiTabs,
  UiToolbarRow,
} from '@octopus/ui'

import { useCatalogStore } from '@/stores/catalog'
import { useShellStore } from '@/stores/shell'
import { pickSkillArchive, pickSkillFolder } from '@/tauri/client'

type DraftMode = 'none' | 'new-skill' | 'new-mcp'
type PendingSkillAction = 'copy' | 'import' | null
type PendingSkillImportSource = 'archive' | 'folder' | null
type SkillCatalogEntry = Extract<WorkspaceToolCatalogEntry, { kind: 'skill' }>

interface SkillTreeRow {
  path: string
  name: string
  kind: WorkspaceSkillTreeNode['kind']
  depth: number
  byteSize?: number
  isText?: boolean
}

interface PendingSkillImportItem {
  id: string
  source: PendingSkillImportSource
  label: string
  slug: string
  archive?: WorkspaceFileUploadPayload
  files?: WorkspaceDirectoryUploadEntry[]
}

interface PendingSkillCopyItem {
  skillId: string
  sourceName: string
  targetName: string
}

const DEFAULT_MCP_CONFIG = JSON.stringify({
  type: 'http',
  url: 'https://example.com/mcp',
}, null, 2)

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
    ]
      .join(' ')
      .toLowerCase()

    return haystack.includes(query)
  })
})

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

      const document = await catalogStore.getMcpServerDocument(entry.serverName)
      if (cancelled) {
        return
      }
      currentMcpDocument.value = document
      mcpServerNameDraft.value = document.serverName
      mcpConfigDraft.value = JSON.stringify(document.config, null, 2)
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
</script>

<template>
  <div class="flex w-full flex-col gap-6 pb-20">
    <header class="px-2">
      <UiSectionHeading
        :eyebrow="t('tools.header.eyebrow')"
        :title="t('sidebar.navigation.tools')"
        :subtitle="catalogStore.error || t('tools.header.subtitle')"
      />
    </header>

    <section class="px-2">
      <UiToolbarRow test-id="workspace-tools-toolbar">
        <template #search>
          <UiInput
            v-model="searchQuery"
            :placeholder="t('tools.search.placeholder')"
          />
        </template>

        <template #tabs>
          <UiTabs v-model="activeTab" :tabs="tabs" />
        </template>

        <template #actions>
          <div class="flex flex-wrap items-center justify-end gap-2">
            <span class="text-[12px] text-text-tertiary">
              {{ t('tools.summary.results', { count: filteredEntries.length, total: activeTabCount }) }}
            </span>
            <UiButton
              v-if="activeTab === 'skill'"
              variant="ghost"
              size="sm"
              @click="beginNewSkill"
            >
              {{ t('tools.actions.newSkill') }}
            </UiButton>
            <UiButton
              v-if="activeTab === 'skill'"
              variant="ghost"
              size="sm"
              @click="openImportSkillDialog"
            >
              {{ t('tools.actions.importSkill') }}
            </UiButton>
            <UiButton
              v-if="activeTab === 'skill' && canCopySelectedSkillsToManaged"
              variant="ghost"
              size="sm"
              data-testid="tools-copy-selected-skills-button"
              @click="copySelectedSkillsToManaged"
            >
              {{ `${t('tools.actions.copyToManaged')} (${selectedExternalSkillEntries.length})` }}
            </UiButton>
            <UiButton
              v-if="activeTab === 'mcp'"
              variant="ghost"
              size="sm"
              @click="beginNewMcp"
            >
              {{ t('tools.actions.newMcp') }}
            </UiButton>
          </div>
        </template>
      </UiToolbarRow>
    </section>

    <div class="grid gap-6 px-2 xl:grid-cols-[minmax(0,1fr)_520px]">
      <section class="space-y-3">
        <UiRecordCard
          v-for="entry in filteredEntries"
          :key="entry.id"
          :title="entry.name"
          :description="entry.description"
          :active="draftMode === 'none' && selectedEntry?.id === entry.id"
          :test-id="`tool-entry-${entry.id}`"
          interactive
          @click="selectEntry(entry.id)"
        >
          <template #eyebrow>
            {{ kindLabel(entry.kind) }}
          </template>

          <template #badges>
            <UiBadge :label="availabilityLabel(entry.availability)" :tone="availabilityTone(entry.availability)" />
            <UiBadge v-if="entry.disabled" :label="t('tools.states.disabled')" tone="warning" />
            <UiBadge v-if="entry.kind === 'builtin' && entry.requiredPermission" :label="permissionLabel(entry.requiredPermission)" subtle />
            <UiBadge v-if="entry.kind === 'skill'" :label="skillStateLabel(entry)" subtle />
            <UiBadge v-if="entry.kind === 'skill' && entry.workspaceOwned" :label="t('tools.states.managed')" subtle />
            <UiBadge v-if="entry.kind === 'skill' && !entry.workspaceOwned" :label="t('tools.states.readonly')" subtle />
            <UiBadge v-if="entry.kind === 'skill' && !entry.workspaceOwned" :label="t('tools.states.external')" subtle />
            <UiBadge v-if="entry.kind === 'mcp' && entry.toolNames.length" :label="`${entry.toolNames.length} tools`" subtle />
          </template>

          <div class="space-y-1">
            <p class="line-clamp-1 text-[12px] text-text-secondary">
              {{ entry.displayPath }}
            </p>
            <p
              v-if="entry.kind === 'mcp' && entry.endpoint"
              class="line-clamp-1 font-mono text-[11px] text-text-tertiary"
            >
              {{ entry.endpoint }}
            </p>
            <p
              v-else-if="entry.kind === 'skill' && entry.shadowedBy"
              class="line-clamp-1 text-[11px] text-text-tertiary"
            >
              {{ t('tools.detail.shadowedBy') }}: {{ entry.shadowedBy }}
            </p>
          </div>

          <template #meta>
            <span
              v-if="entry.kind === 'mcp' && entry.statusDetail"
              class="text-[11px] text-status-warning"
            >
              {{ entry.statusDetail }}
            </span>
            <span
              v-else-if="entry.kind === 'skill'"
              class="text-[11px] text-text-tertiary"
            >
              {{ sourceOriginLabel(entry) }}
            </span>
            <span
              v-else-if="entry.kind === 'builtin' && entry.builtinKey"
              class="font-mono text-[11px] text-text-tertiary"
            >
              {{ entry.builtinKey }}
            </span>
          </template>

          <template #actions>
            <div
              v-if="isExternalSkillEntry(entry)"
              class="flex items-center"
              @click.stop
              @keydown.stop
            >
              <UiCheckbox
                v-model="selectedExternalSkillIds"
                :value="entry.id"
                :label="t('tools.actions.selectForCopy')"
                :class="'text-[12px] text-text-secondary'"
                :data-testid="`tool-entry-select-${entry.id}`"
              />
            </div>
          </template>
        </UiRecordCard>

        <UiEmptyState
          v-if="!filteredEntries.length"
          :title="searchQuery ? t('tools.empty.filteredTitle') : t('tools.empty.title')"
          :description="searchQuery ? t('tools.empty.filteredDescription') : t('tools.empty.description')"
        />
      </section>

      <section>
        <UiRecordCard
          v-if="draftMode === 'new-skill'"
          :title="t('tools.actions.newSkill')"
          :description="t('tools.editor.skillCreateDescription')"
        >
          <template #eyebrow>
            {{ t('tools.detail.title') }}
          </template>

          <div class="space-y-4">
            <UiField :label="t('tools.editor.skillSlug')">
              <UiInput v-model="skillSlugDraft" />
            </UiField>

            <UiField :label="t('tools.editor.skillContent')">
              <UiCodeEditor
                language="markdown"
                theme="octopus"
                :model-value="newSkillContentDraft"
                @update:model-value="newSkillContentDraft = $event"
              />
            </UiField>

            <div v-if="panelError" class="rounded-md border border-status-error/20 bg-status-error/5 px-3 py-2 text-[12px] text-status-error">
              {{ panelError }}
            </div>

            <div class="flex gap-2">
              <UiButton :loading="submitting" @click="saveCurrent">
                {{ t('common.save') }}
              </UiButton>
            </div>
          </div>
        </UiRecordCard>

        <UiRecordCard
          v-else-if="draftMode === 'new-mcp'"
          :title="t('tools.actions.newMcp')"
          :description="t('tools.editor.mcpCreateDescription')"
        >
          <template #eyebrow>
            {{ t('tools.detail.title') }}
          </template>

          <div class="space-y-4">
            <UiField :label="t('tools.editor.mcpServerName')">
              <UiInput v-model="mcpServerNameDraft" />
            </UiField>

            <UiField :label="t('tools.editor.mcpConfig')">
              <UiCodeEditor
                language="json"
                theme="octopus"
                :model-value="mcpConfigDraft"
                @update:model-value="mcpConfigDraft = $event"
              />
            </UiField>

            <div v-if="panelError" class="rounded-md border border-status-error/20 bg-status-error/5 px-3 py-2 text-[12px] text-status-error">
              {{ panelError }}
            </div>

            <div class="flex gap-2">
              <UiButton :loading="submitting" @click="saveCurrent">
                {{ t('common.save') }}
              </UiButton>
            </div>
          </div>
        </UiRecordCard>

        <UiRecordCard
          v-else-if="selectedEntry"
          :title="selectedEntry.name"
          :description="selectedEntry.description"
        >
          <template #eyebrow>
            {{ t('tools.detail.title') }}
          </template>

          <div class="space-y-4">
            <template v-if="selectedEntry">
              <div class="grid gap-3 border-b border-border/40 pb-4 sm:grid-cols-[minmax(0,1fr)_auto]">
                <div class="space-y-1">
                  <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
                    {{ kindLabel(selectedEntry.kind) }}
                  </div>
                  <div class="text-[12px] text-text-secondary">
                    {{ selectedEntry.description }}
                  </div>
                </div>

                <div class="flex min-h-10 min-w-[196px] flex-wrap content-start justify-end gap-1.5">
                  <UiBadge :label="availabilityLabel(selectedEntry.availability)" :tone="availabilityTone(selectedEntry.availability)" />
                  <UiBadge v-if="selectedEntry.disabled" :label="t('tools.states.disabled')" tone="warning" />
                  <UiBadge v-if="selectedEntry.kind === 'skill'" :label="skillStateLabel(selectedEntry)" subtle />
                  <UiBadge v-if="selectedEntry.kind === 'skill' && selectedEntry.workspaceOwned" :label="t('tools.states.managed')" subtle />
                  <UiBadge v-if="selectedEntry.kind === 'skill' && !selectedEntry.workspaceOwned" :label="t('tools.states.readonly')" subtle />
                  <UiBadge v-if="selectedEntry.kind === 'skill' && !selectedEntry.workspaceOwned" :label="t('tools.states.external')" subtle />
                  <UiBadge v-if="selectedEntry.kind === 'mcp' && selectedEntry.toolNames.length" :label="`${selectedEntry.toolNames.length} tools`" subtle />
                  <UiBadge v-if="selectedEntry.kind === 'builtin' && selectedEntry.requiredPermission" :label="permissionLabel(selectedEntry.requiredPermission)" subtle />
                </div>
              </div>

              <div class="space-y-3">
                <div class="space-y-1">
                  <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
                    {{ t('tools.detail.sourcePath') }}
                  </div>
                  <div class="break-all font-mono text-[12px] text-text-secondary">
                    {{ selectedEntry.displayPath }}
                  </div>
                </div>

                <div class="space-y-1">
                  <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
                    {{ t('tools.detail.sourceKey') }}
                  </div>
                  <div class="break-all font-mono text-[12px] text-text-secondary">
                    {{ selectedEntry.sourceKey }}
                  </div>
                </div>

                <div v-if="selectedEntry.kind === 'builtin' && selectedEntry.requiredPermission" class="space-y-1">
                  <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
                    {{ t('tools.detail.requiredPermission') }}
                  </div>
                  <div class="text-[13px] text-text-primary">
                    {{ permissionLabel(selectedEntry.requiredPermission) }}
                  </div>
                </div>

                <div class="space-y-1">
                  <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
                    {{ t('tools.detail.disabled') }}
                  </div>
                  <UiSwitch
                    :model-value="selectedEntry.disabled"
                    :disabled="toggling || !selectedEntry.management.canDisable"
                    :label="t('tools.actions.disable')"
                    @update:model-value="toggleDisabled(selectedEntry, $event)"
                  />
                </div>
              </div>
            </template>

            <template v-if="!selectedEntry">
              <UiEmptyState
                :title="t('tools.empty.selectionTitle')"
                :description="t('tools.empty.selectionDescription')"
              />
            </template>

            <template v-else>
              <template v-if="selectedEntry.kind === 'builtin'">
                <div class="space-y-1">
                  <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
                    {{ t('tools.detail.builtinKey') }}
                  </div>
                  <div class="font-mono text-[12px] text-text-secondary">
                    {{ selectedEntry.builtinKey }}
                  </div>
                </div>
              </template>

              <template v-else-if="selectedEntry.kind === 'skill'">
                <div class="space-y-3">
                  <div class="space-y-1">
                    <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
                      {{ t('tools.detail.activeState') }}
                    </div>
                    <div class="text-[13px] text-text-primary">
                      {{ skillStateLabel(selectedEntry) }}
                    </div>
                  </div>

                  <div class="space-y-1">
                    <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
                      {{ t('tools.detail.sourceOrigin') }}
                    </div>
                    <div class="text-[13px] text-text-primary">
                      {{ sourceOriginLabel(selectedEntry) }}
                    </div>
                  </div>

                  <div class="space-y-1">
                    <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
                      {{ t('tools.detail.workspaceOwned') }}
                    </div>
                    <div class="text-[13px] text-text-primary">
                      {{ selectedEntry.workspaceOwned ? t('tools.detail.workspaceOwnedYes') : t('tools.detail.workspaceOwnedNo') }}
                    </div>
                  </div>

                  <div class="space-y-1">
                    <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
                      {{ t('tools.detail.relativePath') }}
                    </div>
                    <div class="break-all font-mono text-[12px] text-text-secondary">
                      {{ selectedEntry.relativePath ?? t('common.na') }}
                    </div>
                  </div>
                </div>

                <div v-if="selectedEntry.shadowedBy" class="space-y-1">
                  <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
                    {{ t('tools.detail.shadowedBy') }}
                  </div>
                  <div class="text-[13px] text-text-primary">
                    {{ selectedEntry.shadowedBy }}
                  </div>
                </div>

                <div v-if="loadingDetail" class="rounded-md border border-border/40 bg-subtle/20 px-3 py-3 text-[12px] text-text-secondary">
                  {{ t('tools.editor.loading') }}
                </div>

                <div v-else class="space-y-4">
                  <div class="rounded-xl border border-border/40 bg-surface/80 p-2">
                    <div class="mb-2 px-2 text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
                      {{ t('tools.editor.skillTree') }}
                    </div>

                    <div v-if="selectedSkillTreeRows.length" class="space-y-1">
                      <button
                        v-for="row in selectedSkillTreeRows"
                        :key="row.path"
                        type="button"
                        class="flex w-full flex-col items-start gap-1 rounded-lg px-2 py-2 text-left text-[12px] transition hover:bg-subtle/60"
                        :class="[
                          row.kind === 'directory' ? 'cursor-default text-text-secondary' : 'text-text-primary',
                          row.kind === 'file' && selectedSkillFilePath === row.path ? 'bg-subtle/80' : '',
                        ]"
                        :style="{ paddingInlineStart: `${row.depth * 14 + 12}px` }"
                        @click="row.kind === 'file' ? selectSkillFile(row.path) : undefined"
                      >
                        <span class="font-mono">{{ row.name }}</span>
                        <span class="break-all text-[11px] text-text-tertiary">
                          {{ row.path }}
                        </span>
                      </button>
                    </div>

                    <UiEmptyState
                      v-else
                      :title="t('tools.empty.selectionTitle')"
                      :description="t('tools.editor.noSkillFiles')"
                    />
                  </div>

                  <div class="space-y-4">
                    <div
                      v-if="loadingSkillFile"
                      class="rounded-md border border-border/40 bg-subtle/20 px-3 py-3 text-[12px] text-text-secondary"
                    >
                      {{ t('tools.editor.loadingFile') }}
                    </div>

                    <template v-else-if="currentSkillFile">
                      <div class="space-y-2 rounded-xl border border-border/40 bg-surface/80 px-4 py-3">
                        <div class="space-y-3">
                          <div class="space-y-1">
                            <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
                              {{ t('tools.editor.selectedFile') }}
                            </div>
                            <div class="break-all font-mono text-[12px] text-text-primary">
                              {{ currentSkillFile.path }}
                            </div>
                          </div>

                          <div class="space-y-1">
                            <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
                              {{ t('tools.detail.sourcePath') }}
                            </div>
                            <div class="break-all text-[12px] text-text-secondary">
                              {{ currentSkillFile.displayPath }}
                            </div>
                          </div>

                          <div class="space-y-3">
                            <div class="space-y-1">
                              <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
                                {{ t('tools.editor.fileKind') }}
                              </div>
                              <div class="text-[12px] text-text-secondary">
                                {{ fileTypeLabel(currentSkillFile) }}
                              </div>
                            </div>

                            <div class="space-y-1">
                              <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
                                {{ t('tools.editor.fileLanguage') }}
                              </div>
                              <div class="text-[12px] text-text-secondary">
                                {{ currentSkillFile.language || t('common.na') }}
                              </div>
                            </div>

                            <div class="space-y-1">
                              <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
                                {{ t('tools.editor.fileSize') }}
                              </div>
                              <div class="text-[12px] text-text-secondary">
                                {{ currentSkillFile.byteSize }} B
                              </div>
                            </div>
                          </div>

                          <div
                            v-if="currentSkillFile.readonly"
                            class="text-[12px] text-text-tertiary"
                          >
                            {{ t('tools.states.readonly') }}
                          </div>
                        </div>
                      </div>

                      <UiCodeEditor
                        v-if="currentSkillFile.isText"
                        :language="currentSkillFile.language || 'markdown'"
                        theme="octopus"
                        :readonly="!canSaveSkillFile"
                        :model-value="skillFileDraft"
                        @update:model-value="skillFileDraft = $event"
                      />

                      <div
                        v-else
                        class="rounded-xl border border-border/40 bg-surface/80 px-4 py-4"
                      >
                        <div class="space-y-2 text-[13px] text-text-secondary">
                          <div>{{ t('tools.editor.binaryReadonly') }}</div>
                          <div>{{ currentSkillFile.contentType || 'application/octet-stream' }}</div>
                        </div>
                      </div>
                    </template>

                    <UiEmptyState
                      v-else
                      :title="t('tools.editor.noSkillFileSelectedTitle')"
                      :description="t('tools.editor.noSkillFileSelectedDescription')"
                    />
                  </div>
                </div>

                <div v-if="panelError" class="rounded-md border border-status-error/20 bg-status-error/5 px-3 py-2 text-[12px] text-status-error">
                  {{ panelError }}
                </div>

                <div class="flex flex-wrap gap-2">
                  <UiButton
                    v-if="canSaveSkillFile"
                    :loading="submitting"
                    @click="saveCurrent"
                  >
                    {{ t('common.save') }}
                  </UiButton>
                  <UiButton
                    v-if="selectedEntry.management.canDelete"
                    variant="ghost"
                    :loading="deleting"
                    @click="deleteCurrent"
                  >
                    {{ t('common.delete') }}
                  </UiButton>
                  <UiButton
                    v-if="canCopySkillToManaged"
                    variant="ghost"
                    :loading="submitting"
                    @click="copySelectedSkillToManaged"
                  >
                    {{ t('tools.actions.copyToManaged') }}
                  </UiButton>
                </div>
              </template>

              <template v-else>
                <div class="space-y-3">
                  <div class="space-y-1">
                    <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
                      {{ t('tools.detail.serverName') }}
                    </div>
                    <div class="text-[13px] text-text-primary">
                      {{ selectedEntry.serverName }}
                    </div>
                  </div>

                  <div class="space-y-1">
                    <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
                      {{ t('tools.detail.scope') }}
                    </div>
                    <div class="text-[13px] text-text-primary">
                      {{ selectedEntry.scope }}
                    </div>
                  </div>
                </div>

                <div class="space-y-1">
                  <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
                    {{ t('tools.detail.endpoint') }}
                  </div>
                  <div class="break-all font-mono text-[12px] text-text-secondary">
                    {{ selectedEntry.endpoint }}
                  </div>
                </div>

                <div class="space-y-1">
                  <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
                    {{ t('tools.detail.toolNames') }}
                  </div>
                  <div
                    v-if="selectedEntry.toolNames.length"
                    class="flex flex-wrap gap-1.5"
                  >
                    <UiBadge
                      v-for="toolName in selectedEntry.toolNames"
                      :key="toolName"
                      :label="toolName"
                      subtle
                    />
                  </div>
                  <div v-else class="text-[13px] text-text-secondary">
                    {{ t('common.na') }}
                  </div>
                </div>

                <div v-if="selectedEntry.statusDetail" class="space-y-1">
                  <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
                    {{ t('tools.detail.statusDetail') }}
                  </div>
                  <div class="text-[13px] text-status-warning">
                    {{ selectedEntry.statusDetail }}
                  </div>
                </div>

                <div v-if="loadingDetail" class="rounded-md border border-border/40 bg-subtle/20 px-3 py-3 text-[12px] text-text-secondary">
                  {{ t('tools.editor.loading') }}
                </div>

                <template v-else>
                  <UiField :label="t('tools.editor.mcpServerName')">
                    <UiInput :model-value="mcpServerNameDraft" disabled />
                  </UiField>

                  <UiField :label="t('tools.editor.mcpConfig')">
                    <UiCodeEditor
                      language="json"
                      theme="octopus"
                      :model-value="mcpConfigDraft"
                      @update:model-value="mcpConfigDraft = $event"
                    />
                  </UiField>
                </template>

                <div v-if="panelError" class="rounded-md border border-status-error/20 bg-status-error/5 px-3 py-2 text-[12px] text-status-error">
                  {{ panelError }}
                </div>

                <div class="flex gap-2">
                  <UiButton :loading="submitting" @click="saveCurrent">
                    {{ t('common.save') }}
                  </UiButton>
                  <UiButton
                    variant="ghost"
                    :loading="deleting"
                    @click="deleteCurrent"
                  >
                    {{ t('common.delete') }}
                  </UiButton>
                </div>
              </template>
            </template>
          </div>
        </UiRecordCard>

        <UiEmptyState
          v-else
          :title="t('tools.empty.selectionTitle')"
          :description="t('tools.empty.selectionDescription')"
        />
      </section>
    </div>
  </div>

  <UiDialog
    v-model:open="skillActionDialogOpen"
    :title="pendingSkillActionTitle"
    :description="pendingSkillActionDescription"
    content-test-id="tools-skill-action-dialog"
  >
    <div class="space-y-4">
      <div
        v-if="pendingSkillAction === 'import'"
        class="space-y-3"
      >
        <div class="flex flex-wrap gap-2">
          <UiButton
            variant="ghost"
            :loading="submitting && pendingSkillImportSource === 'archive'"
            @click="importArchiveSkill"
          >
            {{ t('tools.actions.importArchive') }}
          </UiButton>
          <UiButton
            variant="ghost"
            :loading="submitting && pendingSkillImportSource === 'folder'"
            @click="importFolderSkill"
          >
            {{ t('tools.actions.importFolder') }}
          </UiButton>
        </div>

        <div v-if="pendingSkillSelectionLabel" class="space-y-1">
          <div class="text-[11px] uppercase tracking-[0.22em] text-text-tertiary">
            {{ t('tools.editor.selectedImportSource') }}
          </div>
          <div class="space-y-1">
            <div
              v-for="item in pendingSkillImportTargets"
              :key="item.id"
              class="rounded-lg border border-border/40 bg-surface/70 px-3 py-2 text-[13px] text-text-secondary"
            >
              <div class="break-all text-text-primary">
                {{ item.label }}
              </div>
              <div class="break-all text-[12px] text-text-tertiary">
                {{ item.slug }}
              </div>
            </div>
          </div>
        </div>
      </div>

      <div
        v-else-if="pendingSkillAction === 'copy'"
        class="space-y-3"
      >
        <div
          v-for="item in pendingSkillCopies"
          :key="item.skillId"
          :data-testid="`tools-skill-action-copy-item-${item.skillId}`"
          class="space-y-3 rounded-lg border border-border/40 bg-surface/70 px-3 py-3"
        >
          <div class="space-y-1">
            <div class="text-[13px] text-text-primary">
              {{ item.sourceName }}
            </div>
            <div class="break-all text-[12px] text-text-tertiary">
              {{ skillDisplayPath(item.skillId) }}
            </div>
          </div>

          <UiField :label="t('tools.editor.skillName')">
            <UiInput v-model="item.targetName" />
          </UiField>

          <div class="text-[12px] text-text-tertiary">
            {{ suggestSlug(item.targetName) }}
          </div>
        </div>
      </div>

      <div v-if="panelError" class="rounded-md border border-status-error/20 bg-status-error/5 px-3 py-2 text-[12px] text-status-error">
        {{ panelError }}
      </div>
    </div>

    <template #footer>
      <UiButton variant="ghost" @click="skillActionDialogOpen = false">
        {{ t('common.cancel') }}
      </UiButton>
      <UiButton :loading="submitting" :disabled="!pendingSkillActionReady" @click="submitPendingSkillAction">
        {{ t('common.confirm') }}
      </UiButton>
    </template>
  </UiDialog>
</template>
