<script setup lang="ts">
import { computed } from 'vue'
import { Network } from 'lucide-vue-next'

import type { AgentRecord } from '@octopus/schema'
import { UiBadge, UiButton, UiCombobox, UiDialog, UiField, UiInput, UiSearchableMultiSelect, UiSelect, UiSurface, UiTextarea } from '@octopus/ui'

import type { SelectOption, TeamFormState } from './useAgentCenter'

const props = defineProps<{
  open: boolean
  form: TeamFormState
  statusOptions: Array<{ value: string, label: string }>
  builtinOptions: SelectOption[]
  skillOptions: SelectOption[]
  mcpOptions: SelectOption[]
  leaderOptions: SelectOption[]
  teamAgentOptions: SelectOption[]
  avatarPreview: string
  dialogTeamLeader: AgentRecord | null | undefined
  dialogTeamMembers: AgentRecord[]
}>()

const emit = defineEmits<{
  'update:open': [value: boolean]
  'pick-avatar': []
  'remove-avatar': []
  'save': []
}>()

const openModel = computed({
  get: () => props.open,
  set: value => emit('update:open', value),
})

function initials(name: string) {
  return name
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map(part => part[0])
    .join('')
    .toUpperCase()
}
</script>

<template>
  <UiDialog
    v-model:open="openModel"
    title="团队配置"
    description="配置团队负责人、核心成员以及协作背景。"
    content-class="max-w-3xl"
    body-class="max-h-[75vh] overflow-y-auto pr-1"
    content-test-id="agent-center-team-dialog"
  >
    <div class="flex flex-col gap-8 py-2">
      <UiSurface variant="subtle" padding="md" title="基础信息" subtitle="设置团队名称、头像和运行状态。" class="space-y-4">
        <div class="grid gap-4 md:grid-cols-2">
          <UiField label="团队名称">
            <UiInput v-model="form.name" placeholder="例如: 核心研发组" />
          </UiField>
          <UiField label="状态">
            <UiSelect v-model="form.status" :options="statusOptions" />
          </UiField>
          <UiField class="md:col-span-2" label="团队头像">
            <div class="flex items-center gap-4 rounded-[var(--radius-l)] border border-border bg-surface p-4">
              <div class="flex size-14 shrink-0 items-center justify-center overflow-hidden rounded-[var(--radius-l)] border border-border bg-subtle text-lg font-bold text-primary">
                <img v-if="avatarPreview" :src="avatarPreview" alt="" class="size-full object-cover">
                <span v-else>{{ initials(form.name || 'T') }}</span>
              </div>
              <div class="flex flex-col gap-2">
                <div class="flex gap-2">
                  <UiButton variant="outline" size="sm" class="h-8" @click="emit('pick-avatar')">上传新头像</UiButton>
                  <UiButton v-if="avatarPreview" variant="ghost" size="sm" class="h-8 text-text-tertiary" @click="emit('remove-avatar')">移除</UiButton>
                </div>
                <p class="text-xs text-text-tertiary">建议使用正方形图片，支持 PNG/JPG 格式。</p>
              </div>
            </div>
          </UiField>
          <UiField class="md:col-span-2" label="核心愿景">
            <UiInput v-model="form.personality" placeholder="定义团队的协作风格和长期目标" />
          </UiField>
        </div>
      </UiSurface>

      <UiSurface variant="subtle" padding="md" title="编组管理" subtitle="指定团队负责人 (Leader) 并选择参与协作的成员。" class="space-y-4">
        <div class="grid gap-4 md:grid-cols-2">
          <UiField label="负责人 (Leader)">
            <UiCombobox
              v-model="form.leaderAgentId"
              :options="leaderOptions"
              placeholder="选择负责人"
            />
          </UiField>
          <UiField class="md:col-span-2" label="协作成员">
            <UiSearchableMultiSelect
              v-model="form.memberAgentIds"
              :options="teamAgentOptions"
              placeholder="搜索并添加数字员工成员"
            />
          </UiField>

          <div class="md:col-span-2">
            <UiSurface class="overflow-hidden border-border p-5">
              <div class="mb-5 flex items-center justify-between">
                <div class="flex items-center gap-2 text-sm font-semibold text-text-primary">
                  <Network :size="15" class="text-primary" />
                  组织结构预览
                </div>
                <UiBadge :label="`${dialogTeamMembers.length + (dialogTeamLeader ? 1 : 0)} 节点`" subtle />
              </div>

              <div class="flex flex-col items-center">
                <div v-if="dialogTeamLeader" class="relative">
                  <div class="min-w-[12rem] rounded-[var(--radius-l)] border border-border bg-surface px-4 py-3 text-center">
                    <div class="text-[10px] font-bold uppercase tracking-[0.16em] text-primary">Leader</div>
                    <div class="mt-1 text-sm font-bold text-text-primary">{{ dialogTeamLeader.name }}</div>
                    <div class="mt-1 line-clamp-1 text-[11px] text-text-tertiary">{{ dialogTeamLeader.personality }}</div>
                  </div>
                  <div class="mx-auto h-8 w-px bg-border" />
                </div>
                <div v-else class="py-6 text-sm text-text-tertiary italic">
                  尚未指定团队负责人
                </div>

                <div v-if="dialogTeamMembers.length > 0" class="grid w-full gap-3 pt-2 sm:grid-cols-2 lg:grid-cols-3">
                  <div
                    v-for="member in dialogTeamMembers"
                    :key="member.id"
                    class="group relative rounded-[var(--radius-m)] border border-border bg-surface p-3 transition-colors hover:border-border-strong"
                  >
                    <div class="flex items-center justify-between gap-2">
                      <strong class="truncate text-sm font-semibold text-text-primary group-hover:text-primary">{{ member.name }}</strong>
                      <UiBadge v-if="member.integrationSource" label="Linked" subtle />
                    </div>
                    <div class="mt-1 line-clamp-1 text-[11px] text-text-tertiary">{{ member.personality }}</div>
                  </div>
                </div>
                <div v-else-if="dialogTeamLeader" class="py-4 text-xs text-text-tertiary italic">
                  暂未添加其他成员
                </div>
              </div>
            </UiSurface>
          </div>
        </div>
      </UiSurface>

      <UiSurface variant="subtle" padding="md" title="协作上下文" subtitle="定义团队的工作流程标签、协作摘要及核心指令。" class="space-y-4">
        <div class="grid gap-4">
          <UiField label="团队摘要">
            <UiTextarea v-model="form.description" :rows="2" placeholder="简述团队的主要工作职责和产出目标..." />
          </UiField>
          <UiField label="协作提示词 (Team Prompt)">
            <UiTextarea v-model="form.prompt" :rows="5" class="font-mono text-sm" placeholder="定义团队的协作 SOP 和核心指令..." />
          </UiField>
          <UiField label="工作流标签">
            <UiInput v-model="form.tagsText" placeholder="用逗号分隔，例如: 并行研发, 自动化测试" />
          </UiField>
        </div>
      </UiSurface>

      <UiSurface variant="subtle" padding="md" title="扩展能力" subtitle="为整个团队集成工具能力。" class="space-y-4">
        <div class="grid gap-4 md:grid-cols-2">
          <UiField label="内置工具">
            <UiSearchableMultiSelect v-model="form.builtinToolKeys" :options="builtinOptions" placeholder="选择内置工具" />
          </UiField>
          <UiField label="技能插件 (Skills)">
            <UiSearchableMultiSelect v-model="form.skillIds" :options="skillOptions" placeholder="选择 Skill" />
          </UiField>
          <UiField class="md:col-span-2" label="MCP 服务集成">
            <UiSearchableMultiSelect v-model="form.mcpServerNames" :options="mcpOptions" placeholder="选择 MCP 服务器" />
          </UiField>
        </div>
      </UiSurface>
    </div>
    <template #footer>
      <div class="flex w-full items-center justify-between">
        <span class="text-xs text-text-tertiary">团队更改将应用于所有成员的协作上下文</span>
        <div class="flex items-center gap-2">
          <UiButton variant="ghost" @click="emit('update:open', false)">取消</UiButton>
          <UiButton class="px-6" @click="emit('save')">保存配置</UiButton>
        </div>
      </div>
    </template>
  </UiDialog>
</template>
