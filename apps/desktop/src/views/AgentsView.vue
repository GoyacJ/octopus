<script setup lang="ts">
import { computed, ref } from 'vue'
import { LayoutGrid, List, Plus, Search, Filter, ArrowUpDown, Bot, Users, MoreHorizontal } from 'lucide-vue-next'
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
  UiDropdownMenu,
  type UiMenuItem
} from '@octopus/ui'
import type { Agent, Team } from '@octopus/schema'

import { enumLabel } from '@/i18n/copy'
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
const viewMode = ref<'gallery' | 'list'>('gallery')
const dialogMode = ref<'create' | 'edit' | null>(null)
const editingId = ref('')

const allAgents = computed(() => isProjectScope.value ? [...workbench.projectReferencedAgents, ...workbench.projectOwnedAgents] : workbench.workspaceLevelAgents)
const allTeams = computed(() => isProjectScope.value ? [...workbench.projectReferencedTeams, ...workbench.projectOwnedTeams] : workbench.workspaceLevelTeams)

const filteredAgents = computed(() => allAgents.value.filter((agent) =>
  !searchQuery.value
  || workbench.agentDisplayName(agent.id).toLowerCase().includes(searchQuery.value.toLowerCase())
  || (agent.role && workbench.agentDisplayRole(agent.id).toLowerCase().includes(searchQuery.value.toLowerCase()))
))

const filteredTeams = computed(() => allTeams.value.filter((team) =>
  !searchQuery.value
  || workbench.teamDisplayName(team.id).toLowerCase().includes(searchQuery.value.toLowerCase())
))

const { pagedItems: pagedAgents, currentPage: agentPage, pageCount: agentPageCount, setPage: setAgentPage } = usePagination(filteredAgents, { pageSize })
const { pagedItems: pagedTeams, currentPage: teamPage, pageCount: teamPageCount, setPage: setTeamPage } = usePagination(filteredTeams, { pageSize })

const addItems: UiMenuItem[] = [
  { key: 'agent', label: t('agents.actions.addAgent') },
  { key: 'team', label: t('agents.actions.addTeam') },
]

async function setActiveTab(tab: 'agent' | 'team') {
  activeTab.value = tab
  await router.replace({ query: { ...route.query, kind: tab === 'team' ? 'team' : undefined } })
}

function handleAdd(key: string) {
  if (key === 'agent') {
    activeTab.value = 'agent'
    openCreateDialog()
  } else {
    activeTab.value = 'team'
    openCreateDialog()
  }
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
    <header class="space-y-4 px-2 pt-4">
      <div class="flex items-center gap-3">
        <div class="p-2.5 bg-primary/10 rounded-xl text-primary shadow-sm">
          <Bot v-if="activeTab === 'agent'" :size="32" />
          <Users v-else :size="32" />
        </div>
        <div>
          <h1 class="text-3xl font-bold text-text-primary tracking-tight">{{ t('agents.header.title') }}</h1>
          <p class="text-text-secondary font-medium">{{ isProjectScope ? t('agents.header.subtitleProject') : t('agents.header.subtitleWorkspace') }}</p>
        </div>
      </div>

      <!-- Simple Filter Bar -->
      <div class="flex items-center justify-between border-y border-border-subtle dark:border-white/[0.05] py-2">
        <div class="flex items-center gap-1">
          <UiButton 
            variant="ghost" 
            size="sm" 
            :class="activeTab === 'agent' ? 'bg-accent font-semibold text-text-primary' : 'text-text-secondary'"
            @click="setActiveTab('agent')"
          >
            {{ t('agents.tabs.agents') }}
          </UiButton>
          <UiButton 
            variant="ghost" 
            size="sm" 
            :class="activeTab === 'team' ? 'bg-accent font-semibold text-text-primary' : 'text-text-secondary'"
            @click="setActiveTab('team')"
          >
            {{ t('agents.tabs.teams') }}
          </UiButton>
        </div>

        <div class="flex items-center gap-2">
          <div class="flex items-center gap-1 bg-subtle p-0.5 rounded-md border border-border-subtle dark:border-white/[0.05] mr-2">
            <button 
              class="p-1.5 rounded transition-colors" 
              :class="viewMode === 'gallery' ? 'bg-background shadow-sm text-primary' : 'text-text-tertiary hover:text-text-secondary'"
              :title="t('agents.view.gallery')"
              @click="viewMode = 'gallery'"
            >
              <LayoutGrid :size="14" />
            </button>
            <button 
              class="p-1.5 rounded transition-colors" 
              :class="viewMode === 'list' ? 'bg-background shadow-sm text-primary' : 'text-text-tertiary hover:text-text-secondary'"
              :title="t('agents.view.list')"
              @click="viewMode = 'list'"
            >
              <List :size="14" />
            </button>
          </div>

          <div class="relative w-64 group">
            <Search :size="14" class="absolute left-3 top-1/2 -translate-y-1/2 text-text-tertiary group-focus-within:text-primary transition-colors z-10" />
            <UiInput 
              v-model="searchQuery" 
              :placeholder="t('agents.actions.searchPlaceholder')" 
              class="pl-9 h-8 text-xs bg-subtle border-none shadow-none focus:bg-accent transition-all focus:ring-1 focus:ring-primary/20" 
            />
          </div>
          
          <UiButton variant="ghost" size="sm" class="h-8 text-xs gap-1.5 text-text-secondary">
            <Filter :size="14" />
            <span>{{ t('agents.actions.filter') }}</span>
          </UiButton>
          
          <UiButton variant="ghost" size="sm" class="h-8 text-xs gap-1.5 text-text-secondary">
            <ArrowUpDown :size="14" />
            <span>{{ t('agents.actions.sort') }}</span>
          </UiButton>
          
          <div class="w-px h-4 bg-border-subtle mx-1"></div>
          
          <UiDropdownMenu :items="addItems" align="end" @select="handleAdd">
            <template #trigger>
              <UiButton variant="primary" size="sm" class="h-8 px-3 gap-1.5 shadow-sm">
                <Plus :size="14" />
                <span>{{ t('agents.actions.new') }}</span>
              </UiButton>
            </template>
          </UiDropdownMenu>
        </div>
      </div>
    </header>

    <!-- Content Area -->
    <main class="px-2">
      <!-- Gallery View -->
      <div v-if="viewMode === 'gallery'">
        <div v-if="activeTab === 'agent'" class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
          <div 
            v-for="agent in pagedAgents" 
            :key="agent.id"
            class="group flex flex-col border border-border-subtle dark:border-white/[0.08] rounded-xl bg-card hover:bg-accent/30 transition-all cursor-pointer overflow-hidden shadow-xs hover:shadow-md hover:-translate-y-0.5"
            @click="openEditDialog(agent.id)"
          >
            <div class="p-5 space-y-4">
              <div class="flex items-start justify-between">
                <div class="flex h-12 w-12 items-center justify-center rounded-lg bg-primary/5 text-primary text-2xl font-bold shadow-inner">
                  {{ workbench.actorDisplayInitial('agent', agent.id, agent.avatar, agent.name) }}
                </div>
                <UiBadge :label="enumLabel('runStatus', agent.status)" :variant="agent.status === 'running' ? 'primary' : 'outline'" subtle class="text-[10px] px-2 py-0.5" />
              </div>
              
              <div class="space-y-1.5">
                <h3 class="font-bold text-text-primary truncate text-base">{{ workbench.agentDisplayName(agent.id) }}</h3>
                <p class="text-xs text-text-secondary font-semibold uppercase tracking-wider">{{ agent.role ? workbench.agentDisplayRole(agent.id) : 'Standard Agent' }}</p>
              </div>
              
              <p class="text-sm text-text-tertiary line-clamp-2 leading-relaxed min-h-[40px]">
                {{ agent.description || agent.summary || 'No description provided.' }}
              </p>

              <div class="flex flex-wrap gap-1.5 pt-1">
                <span v-for="tag in agent.skillTags.slice(0, 3)" :key="tag" class="px-2 py-0.5 bg-secondary/50 text-[10px] font-medium rounded-full text-text-secondary border border-border-subtle/50">
                  {{ tag }}
                </span>
              </div>
            </div>
          </div>
        </div>

        <div v-else class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
          <div 
            v-for="team in pagedTeams" 
            :key="team.id"
            class="group flex flex-col border border-border-subtle dark:border-white/[0.08] rounded-xl bg-card hover:bg-accent/30 transition-all cursor-pointer overflow-hidden shadow-xs hover:shadow-md hover:-translate-y-0.5"
            @click="openEditDialog(team.id)"
          >
            <div class="p-5 space-y-4">
              <div class="flex items-start justify-between">
                <div class="flex h-12 w-12 items-center justify-center rounded-lg bg-primary/5 text-primary text-2xl font-bold shadow-inner">
                  {{ workbench.actorDisplayInitial('team', team.id, team.avatar, team.name) }}
                </div>
                <div class="flex -space-x-2">
                  <div v-for="i in 3" :key="i" class="w-6 h-6 rounded-full bg-accent border-2 border-card flex items-center justify-center text-[8px] font-bold">
                    {{ i }}
                  </div>
                </div>
              </div>
              
              <div class="space-y-1.5">
                <h3 class="font-bold text-text-primary truncate text-base">{{ workbench.teamDisplayName(team.id) }}</h3>
                <p class="text-xs text-text-secondary font-semibold uppercase tracking-wider">{{ enumLabel('teamMode', team.mode) }}</p>
              </div>
              
              <p class="text-sm text-text-tertiary line-clamp-2 leading-relaxed min-h-[40px]">
                {{ team.description || 'Collaborative intelligence squad.' }}
              </p>
            </div>
          </div>
        </div>
      </div>

      <!-- List View -->
      <div v-else class="border border-border-subtle dark:border-white/[0.05] rounded-xl overflow-hidden bg-card">
        <table class="w-full text-left border-collapse">
          <thead>
            <tr class="bg-subtle/50 text-[11px] font-bold uppercase tracking-wider text-text-tertiary border-b border-border-subtle dark:border-white/[0.05]">
              <th class="px-4 py-3">{{ t('agents.list.columns.name') }}</th>
              <th class="px-4 py-3">{{ t('agents.list.columns.role') }}</th>
              <th class="px-4 py-3">{{ t('agents.list.columns.status') }}</th>
              <th class="px-4 py-3">{{ t('agents.list.columns.tags') }}</th>
              <th class="px-4 py-3 text-right">{{ t('agents.list.columns.actions') }}</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-border-subtle dark:divide-white/[0.05]">
            <template v-if="activeTab === 'agent'">
              <tr 
                v-for="agent in pagedAgents" 
                :key="agent.id"
                class="group hover:bg-accent/20 cursor-pointer transition-colors"
                @click="openEditDialog(agent.id)"
              >
                <td class="px-4 py-3">
                  <div class="flex items-center gap-3">
                    <div class="flex h-8 w-8 items-center justify-center rounded bg-primary/5 text-primary text-sm font-bold">
                      {{ workbench.actorDisplayInitial('agent', agent.id, agent.avatar, agent.name) }}
                    </div>
                    <span class="font-semibold text-text-primary">{{ workbench.agentDisplayName(agent.id) }}</span>
                  </div>
                </td>
                <td class="px-4 py-3 text-sm text-text-secondary">
                  {{ agent.role ? workbench.agentDisplayRole(agent.id) : '—' }}
                </td>
                <td class="px-4 py-3">
                  <UiBadge :label="enumLabel('runStatus', agent.status)" :variant="agent.status === 'running' ? 'primary' : 'outline'" subtle class="text-[10px]" />
                </td>
                <td class="px-4 py-3">
                  <div class="flex gap-1">
                    <span v-for="tag in agent.skillTags.slice(0, 2)" :key="tag" class="px-1.5 py-0.5 bg-subtle text-[10px] rounded text-text-tertiary border border-border-subtle/30">
                      {{ tag }}
                    </span>
                  </div>
                </td>
                <td class="px-4 py-3 text-right">
                  <button class="p-1 rounded hover:bg-accent text-text-tertiary opacity-0 group-hover:opacity-100 transition-all">
                    <MoreHorizontal :size="14" />
                  </button>
                </td>
              </tr>
            </template>
            <template v-else>
              <tr 
                v-for="team in pagedTeams" 
                :key="team.id"
                class="group hover:bg-accent/20 cursor-pointer transition-colors"
                @click="openEditDialog(team.id)"
              >
                <td class="px-4 py-3">
                  <div class="flex items-center gap-3">
                    <div class="flex h-8 w-8 items-center justify-center rounded bg-primary/5 text-primary text-sm font-bold">
                      {{ workbench.actorDisplayInitial('team', team.id, team.avatar, team.name) }}
                    </div>
                    <span class="font-semibold text-text-primary">{{ workbench.teamDisplayName(team.id) }}</span>
                  </div>
                </td>
                <td class="px-4 py-3 text-sm text-text-secondary">
                  {{ enumLabel('teamMode', team.mode) }}
                </td>
                <td class="px-4 py-3">
                  <UiBadge label="Active" variant="outline" subtle class="text-[10px]" />
                </td>
                <td class="px-4 py-3 text-text-tertiary text-xs">
                  Collaborative Intelligence
                </td>
                <td class="px-4 py-3 text-right">
                  <button class="p-1 rounded hover:bg-accent text-text-tertiary opacity-0 group-hover:opacity-100 transition-all">
                    <MoreHorizontal :size="14" />
                  </button>
                </td>
              </tr>
            </template>
          </tbody>
        </table>
      </div>

      <!-- Pagination -->
      <div class="mt-8 flex justify-center">
        <UiPagination 
          v-if="activeTab === 'agent' && agentPageCount > 1" 
          :page="agentPage" 
          :page-count="agentPageCount" 
          @update:page="setAgentPage" 
        />
        <UiPagination 
          v-if="activeTab === 'team' && teamPageCount > 1" 
          :page="teamPage" 
          :page-count="teamPageCount" 
          @update:page="setTeamPage" 
        />
      </div>
    </main>
  </div>

  <!-- Agent Dialog -->
  <UiDialog
    v-model:open="dialogMode"
    :title="dialogMode === 'create' ? t('agents.dialog.createTitle') : t('agents.dialog.editTitle')"
    class="max-w-2xl"
  >
    <div class="space-y-6">
      <div class="flex items-center gap-6 pb-6 border-b border-border-subtle dark:border-white/[0.05]">
        <div class="flex h-20 w-20 items-center justify-center rounded-xl bg-primary/5 text-primary text-3xl font-bold shadow-inner">
          {{ activeTab === 'agent' ? 'AG' : 'TM' }}
        </div>
        <div class="flex-1 space-y-4">
          <UiField :label="t('agents.dialog.name')">
            <UiInput :placeholder="t('agents.dialog.namePlaceholder')" class="text-lg font-bold h-10 bg-subtle border-none" />
          </UiField>
        </div>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-2 gap-x-8 gap-y-4">
        <UiField :label="t('agents.dialog.role')">
          <UiInput :placeholder="t('agents.dialog.rolePlaceholder')" class="bg-subtle border-none" />
        </UiField>
        <UiField :label="t('agents.dialog.model')">
          <UiSelect :options="[{label: 'GPT-4o', value: 'gpt-4o'}]" class="bg-subtle border-none" />
        </UiField>
        <UiField :label="t('agents.dialog.tags')" class="md:col-span-2">
          <UiInput :placeholder="t('agents.dialog.tagsPlaceholder')" class="bg-subtle border-none" />
        </UiField>
        <UiField :label="t('agents.dialog.description')" class="md:col-span-2">
          <UiTextarea :rows="4" :placeholder="t('agents.dialog.descriptionPlaceholder')" class="bg-subtle border-none resize-none" />
        </UiField>
      </div>
    </div>
    
    <template #footer>
      <div class="flex justify-between w-full">
        <UiButton variant="ghost" class="text-destructive hover:bg-destructive/10">{{ t('common.delete') }}</UiButton>
        <div class="flex gap-2">
          <UiButton variant="ghost" @click="dialogMode = null">{{ t('common.cancel') }}</UiButton>
          <UiButton variant="primary" class="shadow-sm">{{ t('common.confirm') }}</UiButton>
        </div>
      </div>
    </template>
  </UiDialog>
</template>

<style scoped>
.line-clamp-2 {
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
</style>
