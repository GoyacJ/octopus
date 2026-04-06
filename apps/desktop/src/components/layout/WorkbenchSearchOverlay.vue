<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { Blocks, FolderKanban, MessageSquare, Search, Waypoints } from 'lucide-vue-next'
import type { RouteLocationRaw } from 'vue-router'

import { UiButton, UiDialog, UiInput, UiPanelFrame } from '@octopus/ui'

import { createProjectConversationTarget, createProjectSurfaceTarget, createWorkspaceOverviewTarget } from '@/i18n/navigation'
import { useRuntimeStore } from '@/stores/runtime'
import { useShellStore } from '@/stores/shell'
import { useWorkspaceStore } from '@/stores/workspace'

type SearchResultKind = 'conversation' | 'project' | 'workspace' | 'navigation'

interface SearchResult {
  id: string
  title: string
  subtitle: string
  section: string
  kind: SearchResultKind
  to: RouteLocationRaw
}

const { t } = useI18n()
const router = useRouter()
const shell = useShellStore()
const workspaceStore = useWorkspaceStore()
const runtime = useRuntimeStore()

const query = ref('')
const searchInput = ref<{ focus: () => void } | null>(null)

const results = computed<SearchResult[]>(() => {
  const workspaceId = workspaceStore.currentWorkspaceId
  const projectId = workspaceStore.currentProjectId

  const conversations: SearchResult[] = runtime.sessions.map((session) => ({
    id: `conversation:${session.id}`,
    title: session.title,
    subtitle: session.lastMessagePreview ?? session.status,
    section: t('searchOverlay.sections.conversations'),
    kind: 'conversation',
    to: createProjectConversationTarget(workspaceId, session.projectId, session.conversationId),
  }))

  const projects: SearchResult[] = workspaceStore.projects.map((project) => ({
    id: `project:${project.id}`,
    title: project.name,
    subtitle: project.description,
    section: t('searchOverlay.sections.projects'),
    kind: 'project',
    to: createWorkspaceOverviewTarget(project.workspaceId, project.id),
  }))

  const workspaces: SearchResult[] = shell.workspaceConnections.map((connection) => ({
    id: `workspace:${connection.workspaceConnectionId}`,
    title: connection.label,
    subtitle: connection.baseUrl,
    section: t('searchOverlay.sections.workspaces'),
    kind: 'workspace',
    to: createWorkspaceOverviewTarget(connection.workspaceId),
  }))

  const navigation: SearchResult[] = [
    {
      id: 'nav-overview',
      title: t('sidebar.navigation.overview'),
      subtitle: t('searchOverlay.navigation.console'),
      section: t('searchOverlay.sections.navigation'),
      kind: 'navigation',
      to: createWorkspaceOverviewTarget(workspaceId, projectId || undefined),
    },
    {
      id: 'nav-resources',
      title: t('sidebar.navigation.resources'),
      subtitle: t('searchOverlay.navigation.resources'),
      section: t('searchOverlay.sections.navigation'),
      kind: 'navigation',
      to: projectId
        ? createProjectSurfaceTarget('project-resources', workspaceId, projectId)
        : { name: 'workspace-resources', params: { workspaceId } },
    },
    {
      id: 'nav-knowledge',
      title: t('sidebar.navigation.knowledge'),
      subtitle: t('searchOverlay.navigation.knowledge'),
      section: t('searchOverlay.sections.navigation'),
      kind: 'navigation',
      to: projectId
        ? createProjectSurfaceTarget('project-knowledge', workspaceId, projectId)
        : { name: 'workspace-knowledge', params: { workspaceId } },
    },
  ]

  const combined = [...conversations, ...projects, ...workspaces, ...navigation]
  const normalizedQuery = query.value.trim().toLowerCase()
  if (!normalizedQuery) {
    return combined.slice(0, 10)
  }

  return combined.filter(item =>
    `${item.title} ${item.subtitle} ${item.section}`.toLowerCase().includes(normalizedQuery),
  )
})

watch(
  () => shell.searchOpen,
  async (open) => {
    if (!open) {
      query.value = ''
      return
    }

    await nextTick()
    searchInput.value?.focus()
  },
)

function resultIcon(kind: SearchResultKind) {
  if (kind === 'conversation') return MessageSquare
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
          <div class="flex items-center gap-3 rounded-xl border border-border bg-background/85 px-4 py-3">
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
