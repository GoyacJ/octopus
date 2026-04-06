<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { ArrowRight, Blocks, FolderKanban, MessageSquare, Search, Waypoints } from 'lucide-vue-next'
import type { RouteLocationRaw } from 'vue-router'

import {
  UiButton,
  UiDialog,
  UiInput,
  UiPanelFrame,
} from '@octopus/ui'

import { createProjectConversationTarget, createProjectSurfaceTarget, createWorkspaceOverviewTarget } from '@/i18n/navigation'
import { useShellStore } from '@/stores/shell'
import { useWorkbenchStore } from '@/stores/workbench'

type SearchResultKind = 'conversation' | 'project' | 'workspace' | 'navigation'

interface SearchResult {
  id: string
  title: string
  subtitle: string
  section: string
  keywords: string[]
  kind: SearchResultKind
  to: RouteLocationRaw
}

const { t } = useI18n()
const router = useRouter()
const shell = useShellStore()
const workbench = useWorkbenchStore()

const query = ref('')
const searchInput = ref<{ focus: () => void } | null>(null)
const workspaceConversationIds = computed(() =>
  new Set(workbench.workspaceProjects.flatMap((project) => project.conversationIds)),
)

const results = computed<SearchResult[]>(() => {
  const normalizedQuery = query.value.trim().toLowerCase()
  const conversations: SearchResult[] = workbench.conversations
    .filter((conversation) => workspaceConversationIds.value.has(conversation.id))
    .map((conversation) => ({
      id: `conversation-${conversation.id}`,
      title: workbench.conversationDisplayTitle(conversation.id),
      subtitle: workbench.conversationDisplaySummary(conversation.id),
      section: t('searchOverlay.sections.conversations'),
      keywords: ['conversation', 'chat', conversation.id],
      kind: 'conversation',
      to: createProjectConversationTarget(workbench.currentWorkspaceId, conversation.projectId, conversation.id),
    }))

  const projects: SearchResult[] = workbench.workspaceProjects.map((project) => ({
    id: `project-${project.id}`,
    title: workbench.projectDisplayName(project.id),
    subtitle: workbench.projectDisplaySummary(project.id),
    section: t('searchOverlay.sections.projects'),
    keywords: ['project', project.id],
    kind: 'project',
    to: createWorkspaceOverviewTarget(project.workspaceId, project.id),
  }))

  const workspaces: SearchResult[] = workbench.workspaces.map((workspace) => ({
    id: `workspace-${workspace.id}`,
    title: workbench.workspaceDisplayName(workspace.id),
    subtitle: workbench.workspaceDisplayDescription(workspace.id),
    section: t('searchOverlay.sections.workspaces'),
    keywords: ['workspace', workspace.id],
    kind: 'workspace',
    to: createWorkspaceOverviewTarget(workspace.id, workspace.projectIds[0]),
  }))

  const navigation: SearchResult[] = [
    {
      id: 'nav-console',
      title: t('sidebar.projectModules.console'),
      subtitle: t('searchOverlay.navigation.console'),
      section: t('searchOverlay.sections.navigation'),
      keywords: ['dashboard', 'console', '控制台', '仪表盘', 'home'],
      kind: 'navigation',
      to: createWorkspaceOverviewTarget(workbench.currentWorkspaceId, workbench.currentProjectId),
    },
    {
      id: 'nav-conversation',
      title: t('sidebar.projectModules.conversations'),
      subtitle: t('searchOverlay.navigation.conversation'),
      section: t('searchOverlay.sections.navigation'),
      keywords: ['conversation', 'chat', '会话'],
      kind: 'navigation',
      to: createProjectConversationTarget(
        workbench.currentWorkspaceId,
        workbench.currentProjectId,
        workbench.currentConversationId,
      ),
    },
    {
      id: 'nav-agents',
      title: t('sidebar.navigation.agents'),
      subtitle: t('searchOverlay.navigation.agents'),
      section: t('searchOverlay.sections.navigation'),
      keywords: ['agent', 'agents', '智能体'],
      kind: 'navigation',
      to: {
        name: 'workspace-agents',
        params: {
          workspaceId: workbench.currentWorkspaceId,
        },
      },
    },
    {
      id: 'nav-resources',
      title: t('sidebar.navigation.resources'),
      subtitle: t('searchOverlay.navigation.resources'),
      section: t('searchOverlay.sections.navigation'),
      keywords: ['resource', 'resources', '资源', 'file', 'folder'],
      kind: 'navigation',
      to: createProjectSurfaceTarget('project-resources', workbench.currentWorkspaceId, workbench.currentProjectId),
    },
    {
      id: 'nav-knowledge',
      title: t('sidebar.navigation.knowledge'),
      subtitle: t('searchOverlay.navigation.knowledge'),
      section: t('searchOverlay.sections.navigation'),
      keywords: ['knowledge', '知识'],
      kind: 'navigation',
      to: createProjectSurfaceTarget('project-knowledge', workbench.currentWorkspaceId, workbench.currentProjectId),
    },
    {
      id: 'nav-trace',
      title: t('sidebar.navigation.trace'),
      subtitle: t('searchOverlay.navigation.trace'),
      section: t('searchOverlay.sections.navigation'),
      keywords: ['trace', '追踪'],
      kind: 'navigation',
      to: createProjectSurfaceTarget('project-trace', workbench.currentWorkspaceId, workbench.currentProjectId),
    },
    {
      id: 'nav-models',
      title: t('sidebar.navigation.models'),
      subtitle: t('searchOverlay.navigation.models'),
      section: t('searchOverlay.sections.navigation'),
      keywords: ['model', 'models', '模型'],
      kind: 'navigation',
      to: {
        name: 'workspace-models',
        params: {
          workspaceId: workbench.currentWorkspaceId,
        },
      },
    },
    {
      id: 'nav-tools',
      title: t('sidebar.navigation.tools'),
      subtitle: t('searchOverlay.navigation.tools'),
      section: t('searchOverlay.sections.navigation'),
      keywords: ['tool', 'tools', 'skill', 'mcp', '工具'],
      kind: 'navigation',
      to: {
        name: 'workspace-tools',
        params: {
          workspaceId: workbench.currentWorkspaceId,
        },
      },
    },
    {
      id: 'nav-settings',
      title: t('sidebar.navigation.settings'),
      subtitle: t('searchOverlay.navigation.settings'),
      section: t('searchOverlay.sections.navigation'),
      keywords: ['settings', 'preferences', '设置'],
      kind: 'navigation',
      to: {
        name: 'app-settings',
      },
    },
  ]

  const combined = [...conversations, ...projects, ...workspaces, ...navigation]
  if (!normalizedQuery) {
    return combined.slice(0, 8)
  }

  return combined.filter((item) =>
    `${item.title} ${item.subtitle} ${item.section} ${item.keywords.join(' ')}`.toLowerCase().includes(normalizedQuery),
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
    await nextTick()
    searchInput.value?.focus()
  },
)

function resultIcon(kind: SearchResultKind) {
  if (kind === 'conversation') {
    return MessageSquare
  }
  if (kind === 'project') {
    return FolderKanban
  }
  if (kind === 'workspace') {
    return Blocks
  }
  return Waypoints
}

function closeOverlay() {
  shell.closeSearch()
}

async function selectResult(item: SearchResult) {
  await router.push(item.to)
  closeOverlay()
}
</script>

<template>
  <UiDialog
    :open="shell.searchOpen"
    :title="t('topbar.searchPlaceholder')"
    :description="t('searchOverlay.emptyDescription')"
    :close-label="t('common.cancel')"
    @update:open="(open) => { if (!open) closeOverlay() }"
  >
    <div data-testid="search-overlay-dialog">
      <UiPanelFrame
        data-testid="search-overlay-panel"
        variant="hero"
        padding="none"
      >
        <div class="space-y-4">
          <div class="border-b border-border/70 px-5 py-4">
            <div class="flex items-center gap-3 rounded-[calc(var(--radius-lg)+2px)] border border-border/70 bg-background/85 px-4 py-3 shadow-xs">
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
            <div v-if="results.length" class="space-y-2">
              <UiButton
                v-for="(item, index) in results"
                :key="item.id"
                variant="ghost"
                class="flex h-auto w-full items-center justify-start gap-3 rounded-[calc(var(--radius-lg)+2px)] px-3 py-3 text-left hover:bg-accent/80"
                :data-testid="`search-result-${index}`"
                :data-result-id="item.id"
                @click="selectResult(item)"
              >
                <span class="flex size-9 shrink-0 items-center justify-center rounded-full bg-primary/[0.12] text-primary">
                  <component :is="resultIcon(item.kind)" :size="16" />
                </span>
                <span class="min-w-0 flex-1">
                  <strong class="block truncate text-sm font-semibold text-text-primary">{{ item.title }}</strong>
                  <small class="block truncate pt-1 text-xs text-text-secondary">{{ item.section }} · {{ item.subtitle }}</small>
                </span>
                <ArrowRight :size="15" class="shrink-0 text-text-tertiary" />
              </UiButton>
            </div>

            <div v-else class="rounded-[calc(var(--radius-lg)+2px)] border border-dashed border-border/80 bg-background/70 px-4 py-6 text-center">
              <strong class="block text-sm font-semibold text-text-primary">{{ t('searchOverlay.emptyTitle') }}</strong>
              <small class="block pt-2 text-xs leading-6 text-text-secondary">{{ t('searchOverlay.emptyDescription') }}</small>
            </div>
          </div>
        </div>
      </UiPanelFrame>
    </div>
  </UiDialog>
</template>
