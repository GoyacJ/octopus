<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { Blocks, FileText, FolderKanban, MessageSquare, Search, Waypoints } from 'lucide-vue-next'
import type { RouteLocationRaw } from 'vue-router'

import type { ConversationRecord, DeliverableSummary } from '@octopus/schema'
import { UiButton, UiDialog, UiInput, UiPanelFrame } from '@octopus/ui'

import {
  createProjectConversationTarget,
  createProjectSurfaceTarget,
  createWorkspaceConsoleSurfaceTarget,
  createWorkspaceOverviewTarget,
} from '@/i18n/navigation'
import { enumLabel } from '@/i18n/copy'
import { useArtifactStore } from '@/stores/artifact'
import { useRuntimeStore } from '@/stores/runtime'
import { useShellStore } from '@/stores/shell'
import { useWorkspaceStore } from '@/stores/workspace'

type SearchResultKind = 'conversation' | 'deliverable' | 'project' | 'workspace' | 'navigation'

interface SearchResult {
  id: string
  title: string
  subtitle: string
  section: string
  kind: SearchResultKind
  to: RouteLocationRaw
  keywords?: string[]
}

interface SearchResultGroup {
  section: string
  items: Array<SearchResult & { index: number }>
}

const { t } = useI18n()
const router = useRouter()
const shell = useShellStore()
const workspaceStore = useWorkspaceStore()
const runtime = useRuntimeStore()
const artifactStore = useArtifactStore()

const query = ref('')
const searchInput = ref<{ focus: () => void } | null>(null)
const activeResultIndex = ref(0)

const SEARCH_RESULTS_LIST_ID = 'workbench-search-overlay-results'

function buildConversationResult(
  conversation: Pick<ConversationRecord, 'sessionId' | 'title' | 'lastMessagePreview' | 'status' | 'workspaceId' | 'projectId' | 'id'>,
): SearchResult {
  return {
    id: `conversation:${conversation.sessionId}`,
    title: conversation.title,
    subtitle: conversation.lastMessagePreview ?? conversation.status,
    section: t('searchOverlay.sections.conversations'),
    kind: 'conversation',
    to: createProjectConversationTarget(
      conversation.workspaceId,
      conversation.projectId,
      conversation.id,
    ),
    keywords: ['conversation', 'session'],
  }
}

function buildDeliverableResult(
  deliverable: Pick<DeliverableSummary, 'id' | 'title' | 'latestVersion' | 'promotionState' | 'workspaceId' | 'projectId'>,
): SearchResult {
  return {
    id: `deliverable:${deliverable.id}`,
    title: deliverable.title,
    subtitle: t('projectDashboard.sections.deliverables.meta', {
      version: deliverable.latestVersion,
      state: enumLabel('deliverablePromotionState', deliverable.promotionState),
    }),
    section: t('searchOverlay.sections.deliverables'),
    kind: 'deliverable',
    to: {
      ...createProjectSurfaceTarget('project-deliverables', deliverable.workspaceId, deliverable.projectId),
      query: {
        deliverable: deliverable.id,
      },
    },
    keywords: ['deliverable', 'deliverables', 'artifact', 'artifacts', deliverable.id],
  }
}

const results = computed<SearchResult[]>(() => {
  const workspaceId = workspaceStore.currentWorkspaceId
  const projectId = workspaceStore.currentProjectId

  const conversationResults = new Map<string, SearchResult>()

  for (const conversation of workspaceStore.activeOverview?.recentConversations ?? []) {
    conversationResults.set(conversation.sessionId, buildConversationResult(conversation))
  }

  for (const conversation of workspaceStore.activeDashboard?.recentConversations ?? []) {
    conversationResults.set(conversation.sessionId, buildConversationResult(conversation))
  }

  for (const session of runtime.sessions.filter(session => session.sessionKind !== 'pet')) {
    conversationResults.set(session.id, {
      id: `conversation:${session.id}`,
      title: session.title,
      subtitle: session.lastMessagePreview ?? session.status,
        section: t('searchOverlay.sections.conversations'),
        kind: 'conversation',
        to: createProjectConversationTarget(workspaceId, session.projectId, session.conversationId),
        keywords: ['conversation', 'session'],
      })
  }

  const conversations = [...conversationResults.values()]
  const deliverables = projectId
    ? artifactStore.activeProjectDeliverables.map(buildDeliverableResult)
    : []

  const projects: SearchResult[] = workspaceStore.projects.map((project) => ({
    id: `project:${project.id}`,
    title: project.name,
    subtitle: project.description,
    section: t('searchOverlay.sections.projects'),
    kind: 'project',
    to: createWorkspaceOverviewTarget(project.workspaceId, project.id),
    keywords: ['project'],
  }))

  const workspaces: SearchResult[] = shell.workspaceConnections.map((connection) => ({
    id: `workspace:${connection.workspaceConnectionId}`,
    title: connection.label,
    subtitle: connection.baseUrl,
    section: t('searchOverlay.sections.workspaces'),
    kind: 'workspace',
    to: createWorkspaceOverviewTarget(connection.workspaceId),
    keywords: ['workspace'],
  }))

  const navigation: SearchResult[] = [
    {
      id: 'nav-overview',
      title: t('sidebar.navigation.overview'),
      subtitle: t('searchOverlay.navigation.console'),
      section: t('searchOverlay.sections.navigation'),
      kind: 'navigation',
      to: createWorkspaceOverviewTarget(workspaceId, projectId || undefined),
      keywords: ['overview', 'dashboard', 'workspace'],
    },
    {
      id: 'nav-deliverables',
      title: t('sidebar.navigation.deliverables'),
      subtitle: t('searchOverlay.navigation.deliverables'),
      section: t('searchOverlay.sections.navigation'),
      kind: 'navigation',
      to: projectId
        ? createProjectSurfaceTarget('project-deliverables', workspaceId, projectId)
        : createWorkspaceOverviewTarget(workspaceId),
      keywords: ['deliverable', 'deliverables', 'artifact', 'artifacts'],
    },
    {
      id: 'nav-resources',
      title: t('sidebar.navigation.resources'),
      subtitle: t('searchOverlay.navigation.resources'),
      section: t('searchOverlay.sections.navigation'),
      kind: 'navigation',
      to: projectId
        ? createProjectSurfaceTarget('project-resources', workspaceId, projectId)
        : createWorkspaceConsoleSurfaceTarget('workspace-console-resources', workspaceId),
      keywords: ['resource', 'resources'],
    },
    {
      id: 'nav-knowledge',
      title: t('sidebar.navigation.knowledge'),
      subtitle: t('searchOverlay.navigation.knowledge'),
      section: t('searchOverlay.sections.navigation'),
      kind: 'navigation',
      to: projectId
        ? createProjectSurfaceTarget('project-knowledge', workspaceId, projectId)
        : createWorkspaceConsoleSurfaceTarget('workspace-console-knowledge', workspaceId),
      keywords: ['knowledge'],
    },
  ]

  const combined = [...conversations, ...deliverables, ...projects, ...workspaces, ...navigation]
  const normalizedQuery = query.value.trim().toLowerCase()
  if (!normalizedQuery) {
    return combined.slice(0, 10)
  }

  return combined.filter(item =>
    `${item.id} ${item.title} ${item.subtitle} ${item.section} ${(item.keywords ?? []).join(' ')}`.toLowerCase().includes(normalizedQuery),
  )
})

const groupedResults = computed<SearchResultGroup[]>(() => {
  const groups = new Map<string, SearchResultGroup>()

  results.value.forEach((item, index) => {
    const existingGroup = groups.get(item.section)
    const entry = {
      ...item,
      index,
    }

    if (existingGroup) {
      existingGroup.items.push(entry)
      return
    }

    groups.set(item.section, {
      section: item.section,
      items: [entry],
    })
  })

  return [...groups.values()]
})

const activeResult = computed(() => results.value[activeResultIndex.value] ?? null)

watch(query, () => {
  activeResultIndex.value = 0
})

watch(
  () => [shell.activeWorkspaceConnectionId, workspaceStore.currentProjectId] as const,
  ([connectionId, projectId]) => {
    if (!connectionId || !projectId) {
      return
    }
    void artifactStore.ensureProjectDeliverables(projectId)
  },
  { immediate: true },
)

watch(
  results,
  (nextResults) => {
    if (!nextResults.length) {
      activeResultIndex.value = 0
      return
    }

    activeResultIndex.value = Math.min(activeResultIndex.value, nextResults.length - 1)
  },
  { immediate: true },
)

watch(
  () => shell.searchOpen,
  async (open) => {
    if (!open) {
      query.value = ''
      activeResultIndex.value = 0
      return
    }

    activeResultIndex.value = 0
    if (shell.activeWorkspaceConnectionId) {
      await workspaceStore.ensureWorkspaceBootstrap(shell.activeWorkspaceConnectionId)
    }
    if (workspaceStore.currentProjectId) {
      await Promise.all([
        workspaceStore.loadProjectDashboard(workspaceStore.currentProjectId),
        artifactStore.ensureProjectDeliverables(workspaceStore.currentProjectId),
      ])
    }
    await runtime.bootstrap()
    await nextTick()
    searchInput.value?.focus()
  },
)

function resultIcon(kind: SearchResultKind) {
  if (kind === 'conversation') return MessageSquare
  if (kind === 'deliverable') return FileText
  if (kind === 'project') return FolderKanban
  if (kind === 'workspace') return Blocks
  return Waypoints
}

function resultOptionId(id: string) {
  return `workbench-search-result-${id.replace(/[^a-z0-9_-]/gi, '-')}`
}

function setActiveResult(index: number) {
  if (!results.value.length) {
    activeResultIndex.value = 0
    return
  }

  activeResultIndex.value = Math.min(Math.max(index, 0), results.value.length - 1)
}

function moveActiveResult(delta: 1 | -1) {
  if (!results.value.length) {
    return
  }

  setActiveResult(activeResultIndex.value + delta)
}

function resultButtonClasses(active: boolean) {
  return [
    'flex h-auto w-full items-center gap-3 rounded-[var(--radius-m)] border px-3 py-3 text-left shadow-none',
    active
      ? 'border-border-strong bg-accent text-text-primary hover:bg-accent'
      : 'border-transparent text-text-primary hover:border-border hover:bg-subtle',
  ].join(' ')
}

function resultIconClasses(active: boolean) {
  return active
    ? 'mt-0.5 rounded-[var(--radius-s)] bg-surface p-2 text-primary'
    : 'mt-0.5 rounded-[var(--radius-s)] bg-subtle p-2 text-text-secondary'
}

function shortcutKeyClasses() {
  return 'inline-flex items-center rounded-full border border-border bg-surface px-1.5 py-0.5 text-[10px] font-semibold text-text-primary'
}

async function handleInputKeydown(event: KeyboardEvent) {
  if (event.key === 'ArrowDown') {
    event.preventDefault()
    moveActiveResult(1)
    return
  }

  if (event.key === 'ArrowUp') {
    event.preventDefault()
    moveActiveResult(-1)
    return
  }

  if (event.key === 'Enter' && activeResult.value) {
    event.preventDefault()
    await selectResult(activeResult.value)
  }
}

async function selectResult(item: SearchResult) {
  await router.push(item.to)
  shell.closeSearch()
}
</script>

<template>
  <UiDialog
    :open="shell.searchOpen"
    :title="t('common.search')"
    :description="t('searchOverlay.emptyDescription')"
    :close-label="t('common.cancel')"
    content-class="overflow-hidden p-0"
    body-class="p-0"
    @update:open="(open) => { if (!open) shell.closeSearch() }"
  >
    <template #header>
      <div class="flex items-center gap-2 text-[12px] font-semibold text-text-secondary">
        <Search :size="14" class="text-text-tertiary" />
        <span>{{ t('common.search') }}</span>
      </div>
    </template>

    <UiPanelFrame variant="hero" padding="none">
      <div data-testid="search-overlay-panel" class="bg-popover">
        <div class="border-b border-border px-5 py-4">
          <div class="flex items-center gap-3 rounded-[var(--radius-l)] border border-border bg-subtle px-4 py-3">
            <Search :size="18" class="shrink-0 text-text-secondary" />
            <UiInput
              ref="searchInput"
              v-model="query"
              data-testid="search-overlay-input"
              :placeholder="t('searchOverlay.placeholder')"
              role="combobox"
              aria-autocomplete="list"
              :aria-expanded="results.length ? 'true' : 'false'"
              :aria-controls="SEARCH_RESULTS_LIST_ID"
              :aria-activedescendant="activeResult ? resultOptionId(activeResult.id) : undefined"
              class="h-auto border-0 bg-transparent px-0 py-0 shadow-none focus-visible:ring-0"
              @keydown="handleInputKeydown"
            />
          </div>
        </div>

        <div class="px-3 pb-3 pt-3">
          <div
            v-if="results.length"
            :id="SEARCH_RESULTS_LIST_ID"
            data-testid="search-overlay-results"
            role="listbox"
            class="max-h-[26rem] space-y-3 overflow-y-auto"
          >
            <section
              v-for="group in groupedResults"
              :key="group.section"
              class="space-y-1.5"
            >
              <div class="px-2 text-[11px] font-semibold uppercase tracking-[0.08em] text-text-tertiary">
                {{ group.section }}
              </div>
              <UiButton
                v-for="item in group.items"
                :id="resultOptionId(item.id)"
                :key="item.id"
                variant="ghost"
                :class="resultButtonClasses(item.index === activeResultIndex)"
                :data-result-id="item.id"
                :data-active="item.index === activeResultIndex ? 'true' : 'false'"
                role="option"
                :aria-selected="item.index === activeResultIndex"
                @mouseenter="setActiveResult(item.index)"
                @click="selectResult(item)"
              >
                <span :class="resultIconClasses(item.index === activeResultIndex)">
                  <component :is="resultIcon(item.kind)" :size="14" />
                </span>
                <span class="min-w-0 flex-1">
                  <span class="block truncate text-sm font-semibold">{{ item.title }}</span>
                  <span class="block truncate text-xs text-text-secondary">{{ item.subtitle }}</span>
                </span>
                <span
                  v-if="item.index === activeResultIndex"
                  class="shrink-0 rounded-full border border-border bg-surface px-2 py-1 text-[10px] font-semibold text-text-secondary"
                >
                  {{ t('common.open') }}
                </span>
              </UiButton>
            </section>
          </div>
          <div
            v-if="results.length"
            data-testid="search-overlay-shortcuts"
            class="mt-3 flex flex-wrap items-center gap-3 border-t border-border bg-subtle px-3 py-3 text-[11px] text-text-secondary"
          >
            <span class="inline-flex items-center gap-2">
              <span class="inline-flex items-center gap-1 rounded-full border border-border bg-subtle px-1.5 py-1">
                <span :class="shortcutKeyClasses()">Up</span>
                <span :class="shortcutKeyClasses()">Down</span>
              </span>
              <span>{{ t('searchOverlay.shortcuts.navigate') }}</span>
            </span>
            <span class="inline-flex items-center gap-2">
              <span :class="shortcutKeyClasses()">Enter</span>
              <span>{{ t('searchOverlay.shortcuts.open') }}</span>
            </span>
          </div>
          <div
            v-else
            data-testid="search-overlay-empty"
            class="rounded-[var(--radius-l)] border border-border bg-subtle px-4 py-10 text-center text-sm text-text-secondary"
          >
            {{ t('searchOverlay.emptyDescription') }}
          </div>
        </div>
      </div>
    </UiPanelFrame>
  </UiDialog>
</template>
