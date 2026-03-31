<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { RouterLink, useRoute, useRouter } from 'vue-router'

import { UiBadge, UiField, UiListRow, UiSectionHeading, UiSurface } from '@octopus/ui'

import { countLabel, enumLabel, resolveMockField, translate } from '@/i18n/copy'
import { createWorkspaceDashboardTarget, createWorkspaceSwitchTarget } from '@/i18n/navigation'
import { useWorkbenchStore } from '@/stores/workbench'

const route = useRoute()
const router = useRouter()
const { t } = useI18n()
const workbench = useWorkbenchStore()

const activeWorkspace = computed(() => workbench.activeWorkspace)
const activeProject = computed(() => workbench.activeProject)
const activeConversationId = computed(() => workbench.activeConversation?.id ?? activeProject.value?.conversationIds[0] ?? '')
const selectedWorkspaceId = computed({
  get: () => workbench.currentWorkspaceId,
  set: (workspaceId: string) => {
    if (!workspaceId || workspaceId === workbench.currentWorkspaceId) {
      return
    }

    workbench.selectWorkspace(workspaceId)
    void router.push(createWorkspaceSwitchTarget(workbench.workspaces, workspaceId))
  },
})

const topLevelNavigation = computed(() => [
  {
    label: t('sidebar.navigation.dashboard'),
    routeName: 'dashboard',
    to: {
      name: 'dashboard',
      params: { workspaceId: workbench.currentWorkspaceId },
      query: { project: activeProject.value?.id ?? workbench.currentProjectId },
    },
  },
  {
    label: t('sidebar.navigation.conversation'),
    routeName: 'conversation',
    to: {
      name: 'conversation',
      params: {
        workspaceId: workbench.currentWorkspaceId,
        projectId: activeProject.value?.id ?? workbench.currentProjectId,
        conversationId: activeConversationId.value,
      },
    },
  },
  {
    label: t('sidebar.navigation.knowledge'),
    routeName: 'knowledge',
    to: {
      name: 'knowledge',
      params: {
        workspaceId: workbench.currentWorkspaceId,
        projectId: activeProject.value?.id ?? workbench.currentProjectId,
      },
    },
  },
  {
    label: t('sidebar.navigation.trace'),
    routeName: 'trace',
    to: {
      name: 'trace',
      params: {
        workspaceId: workbench.currentWorkspaceId,
        projectId: activeProject.value?.id ?? workbench.currentProjectId,
      },
    },
  },
  {
    label: t('sidebar.navigation.agents'),
    routeName: 'agents',
    to: {
      name: 'agents',
      params: { workspaceId: workbench.currentWorkspaceId },
    },
  },
  {
    label: t('sidebar.navigation.teams'),
    routeName: 'teams',
    to: {
      name: 'teams',
      params: { workspaceId: workbench.currentWorkspaceId },
    },
  },
  {
    label: t('sidebar.navigation.settings'),
    routeName: 'settings',
    to: {
      name: 'settings',
      params: { workspaceId: workbench.currentWorkspaceId },
    },
  },
  {
    label: t('sidebar.navigation.automations'),
    routeName: 'automations',
    to: {
      name: 'automations',
      params: { workspaceId: workbench.currentWorkspaceId },
    },
  },
  {
    label: t('sidebar.navigation.connections'),
    routeName: 'connections',
    to: {
      name: 'connections',
    },
  },
])
</script>

<template>
  <aside class="sidebar-shell scroll-y">
    <UiSurface :eyebrow="t('sidebar.header.eyebrow')" :title="t('sidebar.header.title')" :subtitle="t('sidebar.header.subtitle')">
      <UiField :label="t('sidebar.workspace.label')" :hint="t('sidebar.workspace.hint')">
        <select v-model="selectedWorkspaceId">
          <option v-for="workspace in workbench.workspaces" :key="workspace.id" :value="workspace.id">
            {{ resolveMockField('workspace', workspace.id, 'name', workspace.name) }}
          </option>
        </select>
      </UiField>

      <div v-if="activeWorkspace" class="workspace-summary">
        <UiSectionHeading
          :eyebrow="t('sidebar.workspace.summaryTitle')"
          :title="resolveMockField('workspace', activeWorkspace.id, 'name', activeWorkspace.name)"
          :subtitle="resolveMockField('workspace', activeWorkspace.id, 'description', activeWorkspace.description)"
        />
        <div class="meta-row">
          <UiBadge
            :label="resolveMockField('workspace', activeWorkspace.id, 'roleSummary', activeWorkspace.roleSummary)"
            :tone="activeWorkspace.isLocal ? 'info' : 'default'"
            subtle
          />
          <UiBadge :label="activeWorkspace.isLocal ? t('sidebar.workspace.localLabel') : t('sidebar.workspace.sharedLabel')" subtle />
          <UiBadge :label="countLabel('common.members', activeWorkspace.memberCount)" subtle />
        </div>
      </div>
    </UiSurface>

    <UiSurface :title="t('sidebar.projectRail.title')" :subtitle="t('sidebar.projectRail.subtitle')">
      <div class="panel-list">
        <RouterLink
          v-for="project in workbench.workspaceProjects"
          :key="project.id"
          :to="createWorkspaceDashboardTarget(project.workspaceId, project.id)"
          class="project-link"
        >
          <UiListRow
            :title="resolveMockField('project', project.id, 'name', project.name)"
            :subtitle="resolveMockField('project', project.id, 'summary', project.summary)"
            :eyebrow="resolveMockField('project', project.id, 'phase', project.phase)"
            :active="project.id === workbench.currentProjectId"
            interactive
          >
            <template #meta>
              <UiBadge :label="enumLabel('projectStatus', project.status)" :tone="project.status === 'active' ? 'success' : 'default'" subtle />
            </template>
          </UiListRow>
        </RouterLink>
      </div>
    </UiSurface>

    <UiSurface :title="t('sidebar.navigation.title')" :subtitle="t('sidebar.navigation.subtitle')">
      <nav class="navigation-list">
        <RouterLink
          v-for="item in topLevelNavigation"
          :key="item.label"
          class="nav-link"
          :class="{ active: route.name === item.routeName }"
          :to="item.to"
        >
          <span>{{ item.label }}</span>
          <small v-if="item.routeName === 'conversation' && activeConversationId">{{ activeConversationId }}</small>
        </RouterLink>
      </nav>
    </UiSurface>

    <UiSurface
      v-if="activeWorkspace && activeProject"
      :title="t('sidebar.scope.title')"
      :subtitle="resolveMockField('project', activeProject.id, 'goal', activeProject.goal)"
    >
      <UiSectionHeading
        :title="resolveMockField('project', activeProject.id, 'name', activeProject.name)"
        :subtitle="resolveMockField('workspace', activeWorkspace.id, 'name', activeWorkspace.name)"
        :eyebrow="t('sidebar.scope.eyebrow')"
      />
      <div class="meta-row">
        <UiBadge :label="resolveMockField('project', activeProject.id, 'phase', activeProject.phase)" tone="info" />
        <UiBadge :label="countLabel('common.artifacts', activeProject.artifactIds.length)" subtle />
        <UiBadge :label="countLabel('common.teams', activeProject.teamIds.length)" subtle />
      </div>
      <p class="scope-copy">{{ resolveMockField('project', activeProject.id, 'recentDecision', activeProject.recentDecision) }}</p>
    </UiSurface>
  </aside>
</template>

<style scoped>
.sidebar-shell {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  padding: 1rem;
  border-right: 1px solid var(--border-subtle);
  background:
    linear-gradient(180deg, color-mix(in srgb, var(--bg-sidebar) 96%, white), var(--bg-sidebar)),
    var(--bg-sidebar);
}

.workspace-stack,
.navigation-list {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.workspace-summary {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  min-width: 0;
}

.workspace-link,
.project-link {
  display: block;
  min-width: 0;
}

.nav-link {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 1rem;
  min-width: 0;
  padding: 0.9rem 1rem;
  border-radius: var(--radius-l);
  border: 1px solid var(--border-subtle);
  background: color-mix(in srgb, var(--bg-subtle) 74%, transparent);
  color: var(--text-secondary);
  transition: transform var(--duration-fast) var(--ease-apple), border-color var(--duration-fast) var(--ease-apple);
}

.nav-link.active {
  border-color: color-mix(in srgb, var(--brand-primary) 35%, var(--border-subtle));
  background:
    radial-gradient(circle at top right, color-mix(in srgb, var(--brand-primary) 12%, transparent), transparent 45%),
    var(--bg-surface);
  color: var(--text-primary);
}

.nav-link:hover {
  transform: translateY(-1px);
}

.scope-copy {
  color: var(--text-secondary);
  line-height: 1.6;
  overflow-wrap: anywhere;
}

.nav-link small {
  min-width: 0;
  overflow-wrap: anywhere;
  text-align: right;
}
</style>
