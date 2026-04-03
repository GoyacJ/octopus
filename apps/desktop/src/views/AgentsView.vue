<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { LayoutGrid, List, Plus, Search, Filter, ArrowUpDown } from 'lucide-vue-next'
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
  UiSurface,
  UiTextarea,
} from '@octopus/ui'
import type { Agent, Team } from '@octopus/schema'

import { enumLabel, resolveMockField } from '@/i18n/copy'
import { usePagination } from '@/composables/usePagination'
import { useWorkbenchStore } from '@/stores/workbench'

const pageSize = 12
const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const workbench = useWorkbenchStore()

const isProjectScope = computed(() => route.name === 'project-agents')
const searchQuery = ref('')
const activeTab = ref<'agent' | 'team'>(route.query.kind === 'team' ? 'team' : 'agent')
const viewMode = ref<'icon' | 'list'>('icon')
const dialogMode = ref<'create' | 'edit' | null>(null)
const editingId = ref('')

const allAgents = computed(() => isProjectScope.value ? [...workbench.projectReferencedAgents, ...workbench.projectOwnedAgents] : workbench.workspaceLevelAgents)
const allTeams = computed(() => isProjectScope.value ? [...workbench.projectReferencedTeams, ...workbench.projectOwnedTeams] : workbench.workspaceLevelTeams)

const filteredAgents = computed(() => allAgents.value.filter(a => 
  !searchQuery.value || resolveMockField('agent', a.id, 'name', a.name).toLowerCase().includes(searchQuery.value.toLowerCase())
))

const { pagedItems: pagedAgents, currentPage: agentPage, pageCount: agentPageCount, setPage: setAgentPage } = usePagination(filteredAgents, { pageSize })

async function setActiveTab(tab: 'agent' | 'team') {
  activeTab.value = tab
  await router.replace({ query: { ...route.query, kind: tab === 'team' ? 'team' : undefined } })
}

function openCreateDialog() {
  dialogMode.value = 'create'
}

function openEditDialog(id: string) {
  editingId.value = id
  dialogMode.value = 'edit'
}
</script>

<template>
  <div class="flex flex-col gap-8 pb-20">
    <!-- Notion Style Page Header -->
    <header class="space-y-4 px-2">
      <div class="flex items-center gap-3">
        <div class="p-2 bg-primary/10 rounded-lg text-primary">
          <Bot v-if="activeTab === 'agent'" :size="32" />
          <Users v-else :size="32" />
        </div>
        <div>
          <h1 class="text-3xl font-bold text-text-primary">{{ activeTab === 'agent' ? 'Agents' : 'Teams' }}</h1>
          <p class="text-text-secondary">{{ isProjectScope ? 'Manage agents for this project' : 'Manage your workspace agents' }}</p>
        </div>
      </div>

      <!-- Simple Filter Bar -->
      <div class="flex items-center justify-between border-y border-border-subtle py-2">
        <div class="flex items-center gap-1">
          <UiButton 
            variant="ghost" 
            size="sm" 
            :class="activeTab === 'agent' ? 'bg-accent font-medium' : ''"
            @click="setActiveTab('agent')"
          >
            Agents
          </UiButton>
          <UiButton 
            variant="ghost" 
            size="sm" 
            :class="activeTab === 'team' ? 'bg-accent font-medium' : ''"
            @click="setActiveTab('team')"
          >
            Teams
          </UiButton>
        </div>

        <div class="flex items-center gap-2">
          <div class="relative w-48">
            <Search :size="14" class="absolute left-2.5 top-1/2 -translate-y-1/2 text-text-tertiary" />
            <UiInput v-model="searchQuery" placeholder="Search..." class="pl-8 h-7 text-xs bg-transparent border-none shadow-none focus:bg-accent" />
          </div>
          <UiButton variant="ghost" size="sm" class="h-7 text-xs gap-1.5">
            <Filter :size="14" />
            <span>Filter</span>
          </UiButton>
          <UiButton variant="ghost" size="sm" class="h-7 text-xs gap-1.5">
            <ArrowUpDown :size="14" />
            <span>Sort</span>
          </UiButton>
          <div class="w-px h-4 bg-border-subtle mx-1"></div>
          <UiButton variant="primary" size="sm" class="h-7 px-3 gap-1.5" @click="openCreateDialog">
            <Plus :size="14" />
            <span>New</span>
          </UiButton>
        </div>
      </div>
    </header>

    <!-- Gallery View -->
    <main class="px-2">
      <div v-if="activeTab === 'agent'" class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
        <div 
          v-for="agent in pagedAgents" 
          :key="agent.id"
          class="group flex flex-col border border-border-subtle rounded-lg bg-white hover:bg-accent/30 transition-all cursor-pointer overflow-hidden shadow-xs hover:shadow-sm"
          @click="openEditDialog(agent.id)"
        >
          <!-- Card Content -->
          <div class="p-4 space-y-3">
            <div class="flex items-start justify-between">
              <div class="flex h-10 w-10 items-center justify-center rounded-md bg-primary/5 text-primary text-xl font-bold">
                {{ agent.avatar?.length < 3 ? agent.avatar : resolveMockField('agent', agent.id, 'name', agent.name).slice(0, 1) }}
              </div>
              <UiBadge :label="agent.status" subtle class="text-[10px]" />
            </div>
            
            <div class="space-y-1">
              <h3 class="font-bold text-text-primary truncate">{{ resolveMockField('agent', agent.id, 'name', agent.name) }}</h3>
              <p class="text-xs text-text-secondary font-medium">{{ agent.title || resolveMockField('agent', agent.id, 'role', agent.role) }}</p>
            </div>
            
            <p class="text-xs text-text-tertiary line-clamp-2 leading-relaxed min-h-[32px]">
              {{ agent.description || agent.summary }}
            </p>

            <div class="flex flex-wrap gap-1 pt-1">
              <span v-for="tag in agent.skillTags.slice(0, 3)" :key="tag" class="px-1.5 py-0.5 bg-subtle text-[10px] rounded text-text-secondary">
                {{ tag }}
              </span>
            </div>
          </div>
        </div>
      </div>

      <!-- Pagination -->
      <div v-if="activeTab === 'agent' && agentPageCount > 1" class="mt-8 flex justify-center">
        <UiPagination :page="agentPage" :page-count="agentPageCount" @update:page="setAgentPage" />
      </div>
    </main>
  </div>

  <!-- Agent Dialog (Notion Style Side Peek or Dialog) -->
  <UiDialog
    v-model:open="dialogMode"
    :title="dialogMode === 'create' ? 'Create Agent' : 'Agent Details'"
    class="max-w-2xl"
  >
    <div class="space-y-6">
      <div class="flex items-center gap-6 pb-6 border-b border-border-subtle">
        <div class="flex h-20 w-20 items-center justify-center rounded-xl bg-primary/5 text-primary text-3xl font-bold">
          AG
        </div>
        <div class="flex-1 space-y-4">
          <UiField label="Name">
            <UiInput placeholder="Agent name" class="text-lg font-bold h-10" />
          </UiField>
        </div>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-2 gap-x-8 gap-y-4">
        <UiField label="Role">
          <UiInput placeholder="e.g. Frontend Expert" />
        </UiField>
        <UiField label="Model">
          <UiSelect :options="[{label: 'GPT-4o', value: 'gpt-4o'}]" />
        </UiField>
        <UiField label="Tags" class="md:col-span-2">
          <UiInput placeholder="Add tags..." />
        </UiField>
        <UiField label="Description" class="md:col-span-2">
          <UiTextarea :rows="4" placeholder="What does this agent do?" />
        </UiField>
      </div>
    </div>
    
    <template #footer>
      <div class="flex justify-between w-full">
        <UiButton variant="ghost" class="text-destructive hover:bg-destructive/10">Delete</UiButton>
        <div class="flex gap-2">
          <UiButton variant="ghost" @click="dialogMode = null">Cancel</UiButton>
          <UiButton variant="primary">Save Changes</UiButton>
        </div>
      </div>
    </template>
  </UiDialog>
</template>
