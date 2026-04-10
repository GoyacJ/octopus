<script setup lang="ts">
import { computed } from 'vue'
import { LayoutGrid, List, Trash2, UsersRound } from 'lucide-vue-next'

import type { AgentRecord, TeamRecord } from '@octopus/schema'
import { UiBadge, UiButton, UiEmptyState, UiInput, UiPagination, UiRecordCard, UiToolbarRow } from '@octopus/ui'

import type { ViewMode } from './useAgentCenter'
import TeamUnitCard from './TeamUnitCard.vue'

const props = defineProps<{
  query: string
  viewMode: ViewMode
  total: number
  page: number
  pageCount: number
  pagedTeams: TeamRecord[]
  currentAgents: AgentRecord[]
}>()

const emit = defineEmits<{
  'update:query': [value: string]
  'update:viewMode': [value: ViewMode]
  'update:page': [value: number]
  'create-team': []
  'open-team': [team: TeamRecord]
  'remove-team': [team: TeamRecord]
}>()

const queryModel = computed({
  get: () => props.query,
  set: value => emit('update:query', value),
})

function teamOriginLabel(team: TeamRecord) {
  return team.integrationSource ? 'Workspace Link' : undefined
}

function resolveAgentName(agentId?: string) {
  if (!agentId) {
    return '未设置负责人'
  }
  return props.currentAgents.find(agent => agent.id === agentId)?.name ?? agentId
}
</script>

<template>
  <section class="space-y-4">
    <UiToolbarRow>
      <template #search>
        <UiInput
          v-model="queryModel"
          placeholder="搜索团队名称、摘要或成员"
          class="max-w-md"
        />
      </template>
      <template #views>
        <UiButton
          variant="ghost"
          size="sm"
          :class="viewMode === 'list' ? 'bg-subtle text-text-primary' : ''"
          @click="emit('update:viewMode', 'list')"
        >
          <List :size="14" />
          列表
        </UiButton>
        <UiButton
          variant="ghost"
          size="sm"
          :class="viewMode === 'card' ? 'bg-subtle text-text-primary' : ''"
          @click="emit('update:viewMode', 'card')"
        >
          <LayoutGrid :size="14" />
          卡片
        </UiButton>
      </template>
      <template #actions>
        <UiButton size="sm" @click="emit('create-team')">
          新建数字团队
        </UiButton>
      </template>
    </UiToolbarRow>

    <div v-if="total" :class="viewMode === 'card' ? 'grid gap-3 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4' : 'space-y-2'">
      <template v-for="team in pagedTeams" :key="team.id">
        <TeamUnitCard
          v-if="viewMode === 'card'"
          :id="team.id"
          :name="team.name"
          :title="team.personality || 'Team'"
          :description="team.description"
          :lead-label="resolveAgentName(team.leaderAgentId)"
          :members="team.memberAgentIds.map(resolveAgentName)"
          :workflow="team.tags.slice(0, 3)"
          :recent-outcome="team.prompt || team.description"
          :origin-label="teamOriginLabel(team)"
          :open-label="team.integrationSource ? '查看' : '编辑'"
          :remove-label="team.integrationSource ? '移除接入' : '删除'"
          :open-test-id="`agent-center-open-team-${team.id}`"
          :remove-test-id="`agent-center-remove-team-${team.id}`"
          @open="emit('open-team', team)"
          @remove="emit('remove-team', team)"
        />

        <UiRecordCard
          v-else
          :title="team.name"
          interactive
          class="hover:bg-subtle/60"
          @click="emit('open-team', team)"
        >
          <template #leading>
            <div class="flex size-10 items-center justify-center overflow-hidden rounded-[var(--radius-m)] border border-border bg-subtle text-text-secondary">
              <UsersRound :size="18" />
            </div>
          </template>
          <template #badges>
            <div class="flex items-center gap-1.5">
              <div
                class="size-2 rounded-full"
                :class="team.status === 'active' ? 'bg-status-success' : 'bg-text-tertiary'"
              />
              <UiBadge v-if="team.integrationSource" label="Workspace" subtle />
            </div>
          </template>
          <div class="flex w-full items-center gap-8 overflow-hidden">
            <div class="flex min-w-0 flex-[2] flex-col gap-0.5">
              <span class="truncate text-[11px] font-semibold uppercase tracking-[0.12em] text-text-tertiary">{{ team.personality || '数字员工团队' }}</span>
              <p class="truncate text-sm text-text-secondary">
                {{ team.description }}
              </p>
            </div>
            <div class="hidden flex-1 shrink-0 items-center gap-1 overflow-hidden lg:flex">
              <span v-for="tag in team.tags.slice(0, 3)" :key="tag" class="truncate rounded-full border border-border bg-subtle px-2 py-0.5 text-[10px] font-medium text-text-tertiary">
                #{{ tag }}
              </span>
            </div>
            <div class="hidden shrink-0 items-center gap-6 md:flex">
              <div class="flex flex-col items-end">
                <span class="text-[9px] font-bold uppercase tracking-tighter text-text-tertiary/40">负责人</span>
                <span class="max-w-[80px] truncate text-xs font-bold text-text-primary/70">
                  {{ resolveAgentName(team.leaderAgentId) || '未设置' }}
                </span>
              </div>
              <div class="flex flex-col items-end">
                <span class="text-[9px] font-bold uppercase tracking-tighter text-text-tertiary/40">成员</span>
                <span class="text-xs font-bold tabular-nums text-text-primary/70">{{ team.memberAgentIds.length }}</span>
              </div>
            </div>
          </div>
          <template #actions>
            <div class="flex items-center gap-1">
              <UiButton size="sm" variant="ghost" class="h-8 px-3 text-[11px] font-semibold" @click.stop="emit('open-team', team)">
                配置
              </UiButton>
              <UiButton
                variant="ghost"
                size="icon"
                class="size-8 rounded-full text-text-tertiary/40 hover:bg-error/10 hover:text-error"
                @click.stop="emit('remove-team', team)"
              >
                <Trash2 :size="14" />
              </UiButton>
            </div>
          </template>
        </UiRecordCard>
      </template>
    </div>

    <UiEmptyState
      v-else
      title="暂无数字团队"
      description="创建工作区或项目数字团队。"
    />

    <UiPagination
      v-if="total > 6"
      :page="page"
      :page-count="pageCount"
      :meta-label="`共 ${total} 项`"
      @update:page="emit('update:page', $event)"
    />
  </section>
</template>
