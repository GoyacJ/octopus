<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { RouterLink, useRoute } from 'vue-router'
import {
  Bot,
  Cpu,
  LayoutDashboard,
  Settings,
  ShieldCheck,
  Wrench,
  UserCircle
} from 'lucide-vue-next'
import { useWorkspaceStore } from '@/stores/workspace'
import { useShellStore } from '@/stores/shell'
import {
  createWorkspaceOverviewTarget,
  createWorkspaceConsoleTarget
} from '@/i18n/navigation'

const { t } = useI18n()
const route = useRoute()
const workspaceStore = useWorkspaceStore()
const shell = useShellStore()

const currentWorkspaceId = computed(() => workspaceStore.currentWorkspaceId)

const railItems = computed(() => {
  const workspaceId = currentWorkspaceId.value
  if (!workspaceId) return []

  return [
    {
      id: 'overview',
      icon: LayoutDashboard,
      label: t('sidebar.navigation.overview'),
      to: createWorkspaceOverviewTarget(workspaceId, workspaceStore.currentProjectId || undefined),
      routeNames: ['workspace-overview']
    },
    {
      id: 'agents',
      icon: Bot,
      label: t('sidebar.navigation.agents'),
      to: { name: 'workspace-console-agents', params: { workspaceId } },
      routeNames: ['workspace-console-agents']
    },
    {
      id: 'models',
      icon: Cpu,
      label: t('sidebar.navigation.models'),
      to: { name: 'workspace-console-models', params: { workspaceId } },
      routeNames: ['workspace-console-models']
    },
    {
      id: 'tools',
      icon: Wrench,
      label: t('sidebar.navigation.tools'),
      to: { name: 'workspace-console-tools', params: { workspaceId } },
      routeNames: ['workspace-console-tools']
    },
    {
      id: 'access',
      icon: ShieldCheck,
      label: t('sidebar.navigation.accessControl'),
      to: { name: 'workspace-access-control', params: { workspaceId } },
      routeNames: ['workspace-access-control', 'workspace-access-control-members', 'workspace-access-control-access', 'workspace-access-control-governance']
    }
  ]
})

function isItemActive(item: any) {
  return item.routeNames.includes(String(route.name ?? ''))
}
</script>

<template>
  <nav class="flex h-full w-[64px] flex-col items-center border-r border-border bg-sidebar py-4 gap-4">
    <!-- Logo -->
    <div class="mb-2 px-3">
      <img src="/logo.png" class="h-8 w-8 rounded-[var(--radius-m)] object-cover" alt="Octopus" />
    </div>

    <!-- Main Navigation -->
    <div class="flex flex-1 flex-col items-center gap-2 w-full px-2">
      <RouterLink
        v-for="item in railItems"
        :key="item.id"
        :to="item.to"
        class="group relative flex h-10 w-10 items-center justify-center rounded-[var(--radius-m)] transition-all duration-fast"
        :class="isItemActive(item) ? 'bg-primary text-primary-foreground shadow-sm shadow-primary/20' : 'text-text-tertiary hover:bg-subtle hover:text-text-secondary'"
        v-tooltip="{ content: item.label, placement: 'right' }"
      >
        <component :is="item.icon" :size="20" />
        
        <!-- Active Indicator -->
        <div 
          v-if="isItemActive(item)"
          class="absolute -left-2 h-5 w-1 rounded-r-full bg-primary"
        />
      </RouterLink>
    </div>

    <!-- Bottom Actions -->
    <div class="flex flex-col items-center gap-2 w-full px-2 mt-auto">
      <RouterLink
        to="/app-settings"
        class="group flex h-10 w-10 items-center justify-center rounded-[var(--radius-m)] text-text-tertiary transition-all duration-fast hover:bg-subtle hover:text-text-secondary"
        :class="route.name === 'app-settings' ? 'bg-subtle text-text-primary' : ''"
        v-tooltip="{ content: t('sidebar.navigation.settings'), placement: 'right' }"
      >
        <Settings :size="20" />
      </RouterLink>

      <div class="h-8 w-8 rounded-full bg-accent flex items-center justify-center text-primary overflow-hidden border border-border/50">
        <UserCircle :size="24" />
      </div>
    </div>
  </nav>
</template>

<style scoped>
/* Tooltip implementation might be needed or use a library */
</style>
