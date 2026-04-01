<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { ArrowRight, Blocks, FolderKanban, MessageSquare, Search, Waypoints } from 'lucide-vue-next'
import type { RouteLocationRaw } from 'vue-router'

import { resolveMockField } from '@/i18n/copy'
import { createProjectConversationTarget, createProjectSurfaceTarget, createWorkspaceOverviewTarget } from '@/i18n/navigation'
import { useShellStore } from '@/stores/shell'
import { useWorkbenchStore } from '@/stores/workbench'

interface SearchResult {
  id: string
  title: string
  subtitle: string
  section: string
  keywords: string[]
  to: RouteLocationRaw
}

const { t } = useI18n()
const router = useRouter()
const shell = useShellStore()
const workbench = useWorkbenchStore()

const query = ref('')
const searchInput = ref<HTMLInputElement | null>(null)
const workspaceConversationIds = computed(() =>
  new Set(workbench.workspaceProjects.flatMap((project) => project.conversationIds)),
)

const results = computed<SearchResult[]>(() => {
  const normalizedQuery = query.value.trim().toLowerCase()
  const conversations: SearchResult[] = workbench.conversations
    .filter((conversation) => workspaceConversationIds.value.has(conversation.id))
    .map((conversation) => ({
    id: `conversation-${conversation.id}`,
    title: resolveMockField('conversation', conversation.id, 'title', conversation.title),
    subtitle: resolveMockField('conversation', conversation.id, 'summary', conversation.summary),
    section: t('searchOverlay.sections.conversations'),
    keywords: ['conversation', 'chat', conversation.id],
    to: createProjectConversationTarget(workbench.currentWorkspaceId, conversation.projectId, conversation.id),
  }))

  const projects: SearchResult[] = workbench.workspaceProjects.map((project) => ({
    id: `project-${project.id}`,
    title: resolveMockField('project', project.id, 'name', project.name),
    subtitle: resolveMockField('project', project.id, 'summary', project.summary),
    section: t('searchOverlay.sections.projects'),
    keywords: ['project', project.id],
    to: createWorkspaceOverviewTarget(project.workspaceId, project.id),
  }))

  const workspaces: SearchResult[] = workbench.workspaces.map((workspace) => ({
    id: `workspace-${workspace.id}`,
    title: resolveMockField('workspace', workspace.id, 'name', workspace.name),
    subtitle: resolveMockField('workspace', workspace.id, 'description', workspace.description),
    section: t('searchOverlay.sections.workspaces'),
    keywords: ['workspace', workspace.id],
    to: createWorkspaceOverviewTarget(workspace.id, workspace.projectIds[0]),
  }))

  const navigation: SearchResult[] = [
    {
      id: 'nav-console',
      title: t('sidebar.projectModules.console'),
      subtitle: t('searchOverlay.navigation.console'),
      section: t('searchOverlay.sections.navigation'),
      keywords: ['dashboard', 'console', '控制台', '仪表盘', 'home'],
      to: createWorkspaceOverviewTarget(workbench.currentWorkspaceId, workbench.currentProjectId),
    },
    {
      id: 'nav-conversation',
      title: t('sidebar.projectModules.conversations'),
      subtitle: t('searchOverlay.navigation.conversation'),
      section: t('searchOverlay.sections.navigation'),
      keywords: ['conversation', 'chat', '会话'],
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
      to: {
        name: 'agents',
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
      to: createProjectSurfaceTarget('resources', workbench.currentWorkspaceId, workbench.currentProjectId),
    },
    {
      id: 'nav-knowledge',
      title: t('sidebar.navigation.knowledge'),
      subtitle: t('searchOverlay.navigation.knowledge'),
      section: t('searchOverlay.sections.navigation'),
      keywords: ['knowledge', '知识'],
      to: createProjectSurfaceTarget('knowledge', workbench.currentWorkspaceId, workbench.currentProjectId),
    },
    {
      id: 'nav-trace',
      title: t('sidebar.navigation.trace'),
      subtitle: t('searchOverlay.navigation.trace'),
      section: t('searchOverlay.sections.navigation'),
      keywords: ['trace', '追踪'],
      to: createProjectSurfaceTarget('trace', workbench.currentWorkspaceId, workbench.currentProjectId),
    },
    {
      id: 'nav-models',
      title: t('sidebar.navigation.models'),
      subtitle: t('searchOverlay.navigation.models'),
      section: t('searchOverlay.sections.navigation'),
      keywords: ['model', 'models', '模型'],
      to: {
        name: 'models',
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
      to: {
        name: 'tools',
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
      to: {
        name: 'settings',
        params: {
          workspaceId: workbench.currentWorkspaceId,
        },
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
    searchInput.value?.focus()
  },
)

async function selectResult(item: SearchResult) {
  await router.push(item.to)
  shell.closeSearch()
}
</script>

<template>
  <div
    v-if="shell.searchOpen"
    class="search-overlay-shell"
    data-testid="search-overlay"
  >
    <button type="button" class="search-overlay-backdrop" @click="shell.closeSearch()" />
    <section class="search-panel">
      <div class="search-panel-header">
        <Search :size="18" />
        <input
          ref="searchInput"
          v-model="query"
          data-testid="search-overlay-input"
          :placeholder="t('searchOverlay.placeholder')"
        />
      </div>

      <div v-if="results.length" class="search-result-list">
        <button
          v-for="(item, index) in results"
          :key="item.id"
          type="button"
          class="search-result"
          :data-testid="`search-result-${index}`"
          :data-result-id="item.id"
          @click="selectResult(item)"
        >
          <span class="search-result-icon">
            <MessageSquare v-if="item.section === t('searchOverlay.sections.conversations')" :size="16" />
            <FolderKanban v-else-if="item.section === t('searchOverlay.sections.projects')" :size="16" />
            <Blocks v-else-if="item.section === t('searchOverlay.sections.workspaces')" :size="16" />
            <Waypoints v-else :size="16" />
          </span>
          <span class="search-result-copy">
            <strong>{{ item.title }}</strong>
            <small>{{ item.section }} · {{ item.subtitle }}</small>
          </span>
          <ArrowRight :size="15" />
        </button>
      </div>
      <div v-else class="search-empty">
        <strong>{{ t('searchOverlay.emptyTitle') }}</strong>
        <small>{{ t('searchOverlay.emptyDescription') }}</small>
      </div>
    </section>
  </div>
</template>

<style scoped>
.search-overlay-shell {
  position: fixed;
  inset: 0;
  z-index: 40;
}

.search-overlay-backdrop {
  position: absolute;
  inset: 0;
  background: rgba(0, 0, 0, 0.52);
}

.search-panel {
  position: relative;
  z-index: 1;
  width: min(720px, calc(100vw - 2rem));
  margin: 7rem auto 0;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-xl);
  background: color-mix(in srgb, var(--bg-surface) 96%, transparent);
  box-shadow: var(--shadow-lg);
  overflow: hidden;
}

.search-panel-header {
  display: flex;
  align-items: center;
  gap: 0.85rem;
  padding: 1rem 1.1rem;
  border-bottom: 1px solid var(--border-subtle);
}

.search-panel-header input {
  border: none;
  background: transparent;
  padding: 0;
  box-shadow: none;
}

.search-result-list,
.search-empty {
  display: flex;
  flex-direction: column;
}

.search-result {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) auto;
  align-items: center;
  gap: 0.85rem;
  padding: 0.95rem 1.1rem;
  border-radius: 0;
  border-bottom: 1px solid var(--border-subtle);
  background: transparent;
  text-align: left;
}

.search-result:last-child {
  border-bottom: none;
}

.search-result-copy {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
  min-width: 0;
}

.search-result-copy small,
.search-empty small {
  color: var(--text-secondary);
}

.search-result-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 2rem;
  height: 2rem;
  border-radius: 999px;
  background: color-mix(in srgb, var(--brand-primary) 14%, transparent);
  color: var(--brand-primary);
}

.search-empty {
  gap: 0.45rem;
  padding: 1.25rem 1.1rem;
}
</style>
