<script setup lang="ts">
import { computed, ref } from 'vue'
import { Bot, Users, Plus, Search, Filter, ArrowUpDown } from 'lucide-vue-next'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'

import {
  UiBadge,
  UiButton,
  UiDialog,
  UiField,
  UiInput,
  UiPagination,
  UiSelect,
  UiTextarea,
} from '@octopus/ui'

import { usePagination } from '@/composables/usePagination'
import { useWorkbenchStore } from '@/stores/workbench'

const PAGE_SIZE = 12
const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const workbench = useWorkbenchStore()

const searchQuery = ref('')
const activeTab = ref<'agent' | 'team'>(route.query.kind === 'team' ? 'team' : 'agent')
const dialogMode = ref<'create' | 'edit' | null>(null)
const editingId = ref('')

// Workspace scope logic
const allAgents = computed(() => workbench.workspaceLevelAgents)
const allTeams = computed(() => workbench.workspaceLevelTeams)

const filteredItems = computed(() => {
  const items = activeTab.value === 'agent' ? allAgents.value : allTeams.value
  return items.filter((item) =>
    !searchQuery.value || workbench.actorDisplayName(activeTab.value, item.id, (item as any).name).toLowerCase().includes(searchQuery.value.toLowerCase())
  )
})

const { pagedItems, currentPage, pageCount, setPage } = usePagination(filteredItems, { pageSize: PAGE_SIZE })

async function setActiveTab(tab: 'agent' | 'team') {
  activeTab.value = tab
  await router.replace({ query: { ...route.query, kind: tab === 'team' ? 'team' : undefined } })
}

function openEditDialog(id: string) {
  editingId.value = id
  dialogMode.value = 'edit'
}
</script>

<template>
  <div class="w-full flex flex-col gap-8 pb-20">
    <header class="space-y-4 px-2">
      <div class="flex items-center gap-3">
        <div class="p-2 bg-primary/10 rounded-lg text-primary">
          <Bot v-if="activeTab === 'agent'" :size="32" />
          <Users v-else :size="32" />
        </div>
        <div>
          <h1 class="text-3xl font-bold text-text-primary">{{ activeTab === 'agent' ? 'Agents Library' : 'Team Library' }}</h1>
          <p class="text-text-secondary">Reusable intelligence assets available for all projects in this workspace.</p>
        </div>
      </div>

      <div class="flex items-center justify-between border-y border-border-subtle dark:border-white/[0.05] py-2">
        <div class="flex items-center gap-1">
          <UiButton variant="ghost" size="sm" :class="activeTab === 'agent' ? 'bg-accent font-medium text-text-primary' : ''" @click="setActiveTab('agent')">Agents</UiButton>
          <UiButton variant="ghost" size="sm" :class="activeTab === 'team' ? 'bg-accent font-medium text-text-primary' : ''" @click="setActiveTab('team')">Teams</UiButton>
        </div>

        <div class="flex items-center gap-2">
          <div class="relative w-64">
            <Search :size="14" class="absolute left-3 top-1/2 -translate-y-1/2 text-text-tertiary" />
            <UiInput v-model="searchQuery" placeholder="Search workspace library..." class="pl-9 h-8 bg-subtle/30" />
          </div>
          <UiButton variant="ghost" size="sm" class="h-8 text-xs gap-1.5"><Filter :size="14" /> Filter</UiButton>
          <div class="w-px h-4 bg-border-subtle mx-1"></div>
          <UiButton variant="primary" size="sm" class="h-8 gap-1.5"><Plus :size="14" /> Create New</UiButton>
        </div>
      </div>
    </header>

    <main class="px-2">
      <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 2xl:grid-cols-6 gap-4">
        <div 
          v-for="item in pagedItems" 
          :key="item.id"
          class="group flex flex-col border border-border-subtle dark:border-white/[0.08] rounded-xl bg-card hover:bg-accent/30 transition-all cursor-pointer overflow-hidden shadow-xs hover:shadow-sm"
          @click="openEditDialog(item.id)"
        >
          <div class="p-4 space-y-3">
            <div class="flex items-start justify-between">
              <div class="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/5 text-primary text-xl font-bold">
                {{ workbench.actorDisplayInitial(activeTab.value, item.id, (item as any).avatar, (item as any).name) }}
              </div>
              <UiBadge :label="(item as any).status" subtle class="text-[10px]" />
            </div>
            <div class="space-y-1">
              <h3 class="font-bold text-text-primary truncate">{{ workbench.actorDisplayName(activeTab.value, item.id, (item as any).name) }}</h3>
              <p class="text-xs text-text-secondary font-medium truncate">{{ (item as any).title }}</p>
            </div>
            <p class="text-xs text-text-tertiary line-clamp-2 leading-relaxed min-h-[32px]">{{ (item as any).description }}</p>
          </div>
        </div>
      </div>

      <div v-if="pageCount > 1" class="mt-10 flex justify-center">
        <UiPagination :page="currentPage" :page-count="pageCount" @update:page="setPage" />
      </div>
    </main>
  </div>
</template>
