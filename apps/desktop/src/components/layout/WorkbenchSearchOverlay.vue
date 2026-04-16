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

const { t } = useI18n()
const router = useRouter()
const shell = useShellStore()
const workspaceStore = useWorkspaceStore()
const runtime = useRuntimeStore()
const artifactStore = useArtifactStore()

const query = ref('')
const searchInput = ref<{ focus: () => void } | null>(null)

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

watch(
  () => [shell.activeWorkspaceConnectionId, workspaceStore.currentProjectId] as const,
  ([connectionId, projectId]) => {
    if (!connectionId || !projectId) {
      return
    }
    void artifactStore.loadProjectDeliverables(projectId)
  },
  { immediate: true },
)

watch(
  () => shell.searchOpen,
  async (open) => {
    if (!open) {
      query.value = ''
      return
    }

    if (shell.activeWorkspaceConnectionId) {
      await workspaceStore.ensureWorkspaceBootstrap(shell.activeWorkspaceConnectionId)
    }
    if (workspaceStore.currentProjectId) {
      await Promise.all([
        workspaceStore.loadProjectDashboard(workspaceStore.currentProjectId),
        artifactStore.loadProjectDeliverables(workspaceStore.currentProjectId),
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

async function selectResult(item: SearchResult) {
  await router.push(item.to)
  shell.closeSearch()
}
</script>

<template>
  <UiDialog
    :open="shell.searchOpen"
    :title="t('topbar.searchPlaceholder')"
    :description="t('searchOverlay.emptyDescription')"
    :close-label="t('common.cancel')"
    @update:open="(open) => { if (!open) shell.closeSearch() }"
  >
    <UiPanelFrame variant="hero" padding="none">
        <div data-testid="search-overlay-panel" class="space-y-4">
        <div class="border-b border-border px-5 py-4">
          <div class="flex items-center gap-3 rounded-[var(--radius-l)] border border-border bg-background px-4 py-3">
            <Search :size="18" class="shrink-0 text-text-secondary" />
            <UiInput
              ref="searchInput"
              v-model="query"
              data-testid="search-overlay-input"
              :placeholder="t('searchOverlay.placeholder')"
              class="h-auto border-0 bg-transparent px-0 py-0 shadow-none focus-visible:ring-0"
            />
          </div>
        </div>

        <div class="px-3 pb-3">
          <div v-if="results.length" data-testid="search-overlay-results" class="space-y-2">
            <UiButton
              v-for="item in results"
              :key="item.id"
              variant="ghost"
              class="flex h-auto w-full items-center justify-between rounded-xl px-3 py-3 text-left"
              :data-result-id="item.id"
              @click="selectResult(item)"
            >
              <span class="flex min-w-0 items-start gap-3">
                <span class="mt-0.5 rounded-md bg-primary/10 p-2 text-primary">
                  <component :is="resultIcon(item.kind)" :size="14" />
                </span>
                <span class="min-w-0">
                  <span class="block truncate text-sm font-semibold text-text-primary">{{ item.title }}</span>
                  <span class="block truncate text-xs text-text-secondary">{{ item.subtitle }}</span>
                </span>
              </span>
              <span class="shrink-0 text-[11px] uppercase tracking-wider text-text-tertiary">{{ item.section }}</span>
            </UiButton>
          </div>
          <div v-else data-testid="search-overlay-empty" class="py-10 text-center text-sm text-text-secondary">
            {{ t('searchOverlay.emptyDescription') }}
          </div>
        </div>
      </div>
    </UiPanelFrame>
  </UiDialog>
</template>
