<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { Blocks, FileText, FolderKanban, MessageSquare, Search, Waypoints } from 'lucide-vue-next'
import type { RouteLocationRaw } from 'vue-router'

import type { ConversationRecord, DeliverableSummary } from '@octopus/schema'
import { UiButton, UiDialog, UiInput, UiKbd, UiPanelFrame, cn } from '@octopus/ui'

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
      id: 'nav-tasks',
      title: t('sidebar.navigation.tasks'),
      subtitle: t('searchOverlay.navigation.tasks'),
      section: t('searchOverlay.sections.navigation'),
      kind: 'navigation',
      to: projectId
        ? createProjectSurfaceTarget('project-tasks', workspaceId, projectId)
        : createWorkspaceOverviewTarget(workspaceId),
      keywords: ['task', 'tasks', 'cowork'],
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
    'flex h-auto w-full items-center gap-3 rounded-[var(--radius-m)] border px-3 py-3 text-left shadow-none transition-[background-color,border-color,box-shadow,transform] duration-normal ease-apple motion-reduce:transition-none active:scale-[0.99] motion-reduce:active:scale-100',
    active
      ? 'border-border-strong bg-accent text-text-primary ring-1 ring-inset ring-border-strong hover:bg-accent'
      : 'border-transparent text-text-primary hover:border-border hover:bg-subtle',
  ].join(' ')
}

function resultIconClasses(active: boolean) {
  return active
    ? 'mt-0.5 rounded-[var(--radius-s)] bg-surface p-2 text-primary'
    : 'mt-0.5 rounded-[var(--radius-s)] bg-subtle p-2 text-text-secondary'
}

function resultSubtitleClasses(active: boolean) {
  return active ? 'block truncate text-xs text-text-primary/78' : 'block truncate text-xs text-text-secondary'
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
    content-class="overflow-hidden p-0 border-primary/20 shadow-[0_0_40px_rgba(var(--color-primary-rgb),0.15)] bg-sidebar/80 backdrop-blur-xl max-w-3xl"
    body-class="p-0"
    @update:open="(open) => { if (!open) shell.closeSearch() }"
  >
    <template #header>
      <div class="flex items-center justify-between gap-4 px-6 py-4 bg-black/20">
        <div class="flex items-center gap-3 text-[13px] font-bold text-primary tracking-tight">
          <Search :size="16" class="animate-pulse" />
          <span class="uppercase tracking-widest">{{ t('common.search') }}</span>
        </div>
        <div class="hidden items-center gap-3 sm:flex">
          <div class="flex gap-1 items-center">
             <UiKbd :keys="shell.searchShortcutKeys" size="sm" class="border-primary/20 bg-primary/5 text-primary/80 font-bold" />
             <span class="text-[10px] text-text-tertiary uppercase font-bold tracking-tighter">Command</span>
          </div>
          <div class="flex gap-1 items-center">
             <UiKbd :keys="['Esc']" size="sm" class="border-border bg-subtle text-text-tertiary font-bold" />
             <span class="text-[10px] text-text-tertiary uppercase font-bold tracking-tighter">Exit</span>
          </div>
        </div>
      </div>
    </template>

    <div class="relative">
      <!-- Search Input Area -->
      <div class="p-6 border-b border-border/30 bg-black/5">
        <div class="group flex items-center gap-4 rounded-[var(--radius-xl)] border border-primary/20 bg-black/20 px-5 py-4 transition-all duration-normal focus-within:border-primary/50 focus-within:bg-black/40 focus-within:shadow-[0_0_15px_rgba(var(--color-primary-rgb),0.1)]">
          <Search :size="22" class="shrink-0 text-primary/60 group-focus-within:text-primary animate-in fade-in duration-500" />
          <UiInput
            ref="searchInput"
            v-model="query"
            data-testid="search-overlay-input"
            :placeholder="t('searchOverlay.placeholder')"
            role="combobox"
            class="h-auto border-0 bg-transparent px-0 py-0 text-lg font-medium placeholder:text-text-tertiary/50 focus-visible:ring-0"
            @keydown="handleInputKeydown"
          />
        </div>
      </div>

      <!-- Results Area -->
      <div class="px-3 py-4 max-h-[60vh] overflow-y-auto scroll-y">
        <div v-if="results.length" class="space-y-6">
          <section
            v-for="group in groupedResults"
            :key="group.section"
            class="space-y-2"
          >
            <div class="px-4 text-[10px] font-extrabold uppercase tracking-[0.2em] text-primary/60 flex items-center gap-2">
              <span class="size-1 rounded-full bg-primary/40" />
              {{ group.section }}
            </div>
            
            <div class="space-y-1">
              <button
                v-for="item in group.items"
                :id="resultOptionId(item.id)"
                :key="item.id"
                type="button"
                :class="resultButtonClasses(item.index === activeResultIndex)"
                @mouseenter="setActiveResult(item.index)"
                @click="selectResult(item)"
              >
                <div :class="cn(
                  'flex size-10 shrink-0 items-center justify-center rounded-xl transition-all duration-normal',
                  item.index === activeResultIndex ? 'bg-primary text-primary-foreground shadow-lg shadow-primary/30 rotate-3 scale-110' : 'bg-black/20 text-text-tertiary'
                )">
                  <component :is="resultIcon(item.kind)" :size="18" />
                </div>
                
                <div class="min-w-0 flex-1">
                  <div class="flex items-center gap-2">
                    <span class="block truncate text-[14px] font-bold tracking-tight">{{ item.title }}</span>
                    <UiBadge v-if="item.kind === 'project'" label="Pro" size="sm" class="text-[9px] bg-primary/10 text-primary border-primary/20" />
                  </div>
                  <span :class="resultSubtitleClasses(item.index === activeResultIndex)">{{ item.subtitle }}</span>
                </div>

                <div v-if="item.index === activeResultIndex" class="shrink-0 animate-in slide-in-from-right-2 duration-300">
                  <UiKbd :keys="['Enter']" size="sm" class="border-primary/30 bg-primary/10 text-primary font-bold" />
                </div>
              </button>
            </div>
          </section>
        </div>

        <!-- Empty State -->
        <div
          v-else
          class="flex flex-col items-center justify-center py-20 px-10 text-center animate-in fade-in zoom-in duration-500"
        >
          <div class="size-20 rounded-full bg-primary/5 flex items-center justify-center mb-6 relative">
            <Search :size="40" class="text-primary/20" />
            <div class="absolute inset-0 rounded-full border border-primary/10 animate-ping" />
          </div>
          <h3 class="text-lg font-bold text-text-primary tracking-tight mb-2">{{ t('searchOverlay.emptyTitle') }}</h3>
          <p class="text-sm text-text-tertiary max-w-xs mx-auto leading-relaxed">{{ t('searchOverlay.emptyDescription') }}</p>
        </div>
      </div>

      <!-- Footer Shortcuts -->
      <div class="px-6 py-3 bg-black/20 border-t border-border/30 flex items-center justify-between text-[11px] font-bold text-text-tertiary/60 uppercase tracking-wider">
        <div class="flex items-center gap-4">
           <div class="flex items-center gap-2">
             <div class="flex gap-1">
                <UiKbd :keys="['↑']" size="xs" class="border-border/50" />
                <UiKbd :keys="['↓']" size="xs" class="border-border/50" />
             </div>
             <span>{{ t('searchOverlay.shortcuts.navigate') }}</span>
           </div>
           <div class="flex items-center gap-2">
             <UiKbd :keys="['Enter']" size="xs" class="border-border/50" />
             <span>{{ t('searchOverlay.shortcuts.open') }}</span>
           </div>
        </div>
        <div class="flex items-center gap-2">
           <UiKbd :keys="['Esc']" size="xs" class="border-border/50" />
           <span>{{ t('common.cancel') }}</span>
        </div>
      </div>
    </div>
  </UiDialog>
</template>
