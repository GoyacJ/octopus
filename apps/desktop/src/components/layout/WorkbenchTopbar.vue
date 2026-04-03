<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute } from 'vue-router'
import { 
  ChevronRight, 
  Menu, 
  Share2, 
  Star, 
  MoreHorizontal,
  Monitor,
  SunMedium,
  MoonStar,
  Check
} from 'lucide-vue-next'

import {
  UiButton,
  UiPopover,
} from '@octopus/ui'

import { resolveMockField } from '@/i18n/copy'
import { useShellStore } from '@/stores/shell'
import { useWorkbenchStore } from '@/stores/workbench'

const { t } = useI18n()
const route = useRoute()
const shell = useShellStore()
const workbench = useWorkbenchStore()

const workspaceLabel = computed(() =>
  workbench.activeWorkspace
    ? resolveMockField('workspace', workbench.activeWorkspace.id, 'name', workbench.activeWorkspace.name)
    : 'Octopus',
)

const projectName = computed(() => {
  const project = workbench.activeProject
  return project ? resolveMockField('project', project.id, 'name', project.name) : ''
})

const currentPageName = computed(() => {
  const name = String(route.name || '')
  if (name.includes('conversations')) return t('sidebar.workspaceMenu.items.conversations')
  if (name.includes('agents')) return t('sidebar.workspaceMenu.items.agents')
  if (name.includes('knowledge')) return t('sidebar.workspaceMenu.items.knowledge')
  if (name.includes('settings')) return t('topbar.settings')
  return ''
})

const themeIcons = {
  system: Monitor,
  light: SunMedium,
  dark: MoonStar,
}

async function selectTheme(theme: 'light' | 'dark' | 'system') {
  await shell.updatePreferences({ theme })
}
</script>

<template>
  <header
    class="flex h-12 items-center justify-between px-4 bg-background border-b border-border-subtle sticky top-0 z-30"
    data-testid="workbench-topbar"
  >
    <!-- Breadcrumbs -->
    <div class="flex items-center gap-2 overflow-hidden">
      <UiButton
        v-if="shell.leftSidebarCollapsed"
        variant="ghost"
        size="icon"
        class="h-7 w-7"
        @click="shell.toggleLeftSidebar()"
      >
        <Menu :size="16" />
      </UiButton>
      
      <nav class="flex items-center gap-1.5 text-sm text-text-secondary truncate">
        <span class="hover:bg-accent px-1.5 py-0.5 rounded cursor-pointer transition-colors">{{ workspaceLabel }}</span>
        <ChevronRight v-if="projectName" :size="14" class="shrink-0 opacity-40" />
        <span v-if="projectName" class="hover:bg-accent px-1.5 py-0.5 rounded cursor-pointer transition-colors truncate">{{ projectName }}</span>
        <ChevronRight v-if="currentPageName" :size="14" class="shrink-0 opacity-40" />
        <span v-if="currentPageName" class="font-medium text-text-primary px-1.5 py-0.5 truncate">{{ currentPageName }}</span>
      </nav>
    </div>

    <!-- Actions -->
    <div class="flex items-center gap-1">
      <UiButton variant="ghost" size="sm" class="text-xs gap-1.5 h-7">
        <Share2 :size="14" />
        <span>Share</span>
      </UiButton>
      
      <UiButton variant="ghost" size="icon" class="h-7 w-7">
        <Star :size="14" />
      </UiButton>

      <UiPopover align="end" class="w-48 p-1">
        <template #trigger>
          <UiButton variant="ghost" size="icon" class="h-7 w-7">
            <MoreHorizontal :size="16" />
          </UiButton>
        </template>
        
        <div class="flex flex-col gap-0.5">
          <div class="px-2 py-1 text-[10px] font-bold text-text-tertiary uppercase">{{ t('topbar.theme') }}</div>
          <button 
            v-for="(icon, key) in themeIcons" 
            :key="key"
            class="flex items-center justify-between w-full px-2 py-1.5 text-sm rounded hover:bg-accent text-left"
            @click="selectTheme(key)"
          >
            <div class="flex items-center gap-2">
              <component :is="icon" :size="14" />
              <span>{{ t(`topbar.themeModes.${key}`) }}</span>
            </div>
            <Check v-if="shell.preferences.theme === key" :size="14" class="text-primary" />
          </button>
        </div>
      </UiPopover>
    </div>
  </header>
</template>
