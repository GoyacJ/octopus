<script setup lang="ts">
import { computed } from 'vue'
import { Network } from 'lucide-vue-next'

import type { AgentRecord } from '@octopus/schema'
import { UiBadge, UiButton, UiCombobox, UiDialog, UiField, UiInput, UiSearchableMultiSelect, UiSelect, UiSurface, UiTextarea, UiWorkflowCanvas } from '@octopus/ui'

import type { SelectOption, TeamFormState } from './useAgentCenter'
import { agentIdFromRef } from './agent-refs'

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
  contentReadonly: boolean
  statusReadonly: boolean
  canSave: boolean
  canCopy: boolean
  copyLabel: string
  canPromote: boolean
  promoting: boolean
}>()

const emit = defineEmits<{
  'update:open': [value: boolean]
  'pick-avatar': []
  'remove-avatar': []
  'save': []
  copy: []
  'promote': []
}>()

const openModel = computed({
  get: () => props.open,
  set: value => emit('update:open', value),
})

interface WorkflowNode {
  id: string
  label: string
  type?: string
  position: { x: number, y: number }
  data: { role: string }
}

interface WorkflowEdge {
  id: string
  source: string
  target: string
  animated?: boolean
}

const flowData = computed(() => {
  const nodes: WorkflowNode[] = []
  const edges: WorkflowEdge[] = []

  if (props.dialogTeamLeader) {
    nodes.push({
      id: props.dialogTeamLeader.id,
      label: props.dialogTeamLeader.name,
      type: 'input',
      position: { x: 250, y: 0 },
      data: { role: 'Leader' }
    })

    props.dialogTeamMembers.forEach((member, index) => {
      nodes.push({
        id: member.id,
        label: member.name,
        position: { x: 100 + (index % 3) * 150, y: 120 + Math.floor(index / 3) * 100 },
        data: { role: 'Member' }
      })

      edges.push({
        id: `e-${props.dialogTeamLeader!.id}-${member.id}`,
        source: props.dialogTeamLeader!.id,
        target: member.id,
        animated: true
      })
    })
  } else if (props.dialogTeamMembers.length > 0) {
    props.dialogTeamMembers.forEach((member, index) => {
      nodes.push({
        id: member.id,
        label: member.name,
        position: { x: 100 + (index % 3) * 150, y: 50 + Math.floor(index / 3) * 100 },
        data: { role: 'Member' }
      })
    })
  }

  return { nodes, edges }
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

const readonlyLeaderLabel = computed(() =>
  props.dialogTeamLeader?.name || agentIdFromRef(props.form.leaderRef) || '未设置负责人',
)

const readonlyMemberLabel = computed(() =>
  props.dialogTeamMembers.map(member => member.name).join('、')
    || props.form.memberRefs.map(agentIdFromRef).filter(Boolean).join('、')
    || '暂无成员',
)
</script>

<template>
  <UiDialog
    v-model:open="openModel"
    title="数字团队配置"
    description="配置数字团队负责人、核心成员以及协作背景。"
    content-class="max-w-3xl"
    body-class="max-h-[75vh] overflow-y-auto pr-1"
    content-test-id="agent-center-team-dialog"
  >
    <div class="flex flex-col gap-8 py-2">
      <UiSurface variant="subtle" padding="md" title="基础信息" subtitle="设置数字团队名称、头像和运行状态。" class="space-y-4">
        <div class="grid gap-4 md:grid-cols-2">
          <UiField label="数字团队名称">
            <UiInput v-model="form.name" :disabled="contentReadonly" placeholder="例如: 核心研发组" />
          </UiField>
          <UiField label="状态">
            <UiSelect v-model="form.status" :options="statusOptions" :disabled="statusReadonly" />
          </UiField>
          <UiField class="md:col-span-2" label="团队头像">
            <div class="flex items-center gap-4 rounded-[var(--radius-l)] border border-border bg-surface p-4">
              <div class="flex size-14 shrink-0 items-center justify-center overflow-hidden rounded-[var(--radius-l)] border border-border bg-subtle text-lg font-bold text-primary">
                <img v-if="avatarPreview" :src="avatarPreview" alt="" class="size-full object-cover">
                <span v-else>{{ initials(form.name || 'T') }}</span>
              </div>
              <div v-if="!contentReadonly" class="flex flex-col gap-2">
                <div class="flex gap-2">
                  <UiButton variant="outline" size="sm" class="h-8" @click="emit('pick-avatar')">上传新头像</UiButton>
                  <UiButton v-if="avatarPreview" variant="ghost" size="sm" class="h-8 text-text-tertiary" @click="emit('remove-avatar')">移除</UiButton>
                </div>
                <p class="text-xs text-text-tertiary">建议使用正方形图片，支持 PNG/JPG 格式。</p>
              </div>
            </div>
          </UiField>
          <UiField class="md:col-span-2" label="核心愿景">
            <UiInput v-model="form.personality" :disabled="contentReadonly" placeholder="定义团队的协作风格和长期目标" />
          </UiField>
        </div>
      </UiSurface>

      <UiSurface variant="subtle" padding="md" title="编组管理" subtitle="指定数字团队负责人 (Leader) 并选择参与协作的成员。" class="space-y-4">
        <div class="grid gap-4 md:grid-cols-2">
          <UiField label="负责人 (Leader)">
            <UiInput
              v-if="contentReadonly"
              :model-value="readonlyLeaderLabel"
              data-testid="agent-center-team-leader-display"
              disabled
            />
            <UiCombobox
              v-else
              v-model="form.leaderRef"
              :options="leaderOptions"
              placeholder="选择负责人"
            />
          </UiField>
          <UiField class="md:col-span-2" label="协作成员">
            <UiInput
              v-if="contentReadonly"
              :model-value="readonlyMemberLabel"
              disabled
            />
            <UiSearchableMultiSelect
              v-else
              v-model="form.memberRefs"
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

              <UiWorkflowCanvas 
                :nodes="flowData.nodes" 
                :edges="flowData.edges" 
                readonly 
                class="h-[300px]"
              />
            </UiSurface>
          </div>
        </div>
      </UiSurface>

      <UiSurface variant="subtle" padding="md" title="协作上下文" subtitle="定义团队的工作流程标签、协作摘要及核心指令。" class="space-y-4">
        <div class="grid gap-4">
          <UiField label="数字团队摘要">
            <UiTextarea v-model="form.description" :rows="2" :disabled="contentReadonly" placeholder="简述团队的主要工作职责和产出目标..." />
          </UiField>
          <UiField label="协作提示词 (Team Prompt)">
            <UiTextarea v-model="form.prompt" :rows="5" :disabled="contentReadonly" class="font-mono text-sm" placeholder="定义团队的协作 SOP 和核心指令..." />
          </UiField>
          <UiField label="工作流标签">
            <UiInput v-model="form.tagsText" :disabled="contentReadonly" placeholder="用逗号分隔，例如: 并行研发, 自动化测试" />
          </UiField>
        </div>
      </UiSurface>

      <UiSurface variant="subtle" padding="md" title="扩展能力" subtitle="为整个团队集成工具能力。" class="space-y-4">
        <div class="grid gap-4 md:grid-cols-2">
          <UiField label="内置工具">
            <UiSearchableMultiSelect v-model="form.builtinToolKeys" :options="builtinOptions" :disabled="contentReadonly" placeholder="选择内置工具" />
          </UiField>
          <UiField label="技能插件 (Skills)">
            <UiSearchableMultiSelect v-model="form.skillIds" :options="skillOptions" :disabled="contentReadonly" placeholder="选择 Skill" />
          </UiField>
          <UiField class="md:col-span-2" label="MCP 服务集成">
            <UiSearchableMultiSelect v-model="form.mcpServerNames" :options="mcpOptions" :disabled="contentReadonly" placeholder="选择 MCP 服务器" />
          </UiField>
        </div>
      </UiSurface>
    </div>
    <template #footer>
      <div class="flex w-full items-center justify-between">
        <div class="flex items-center gap-2">
          <span class="text-xs text-text-tertiary">数字团队更改将应用于所有成员的协作上下文</span>
          <UiButton
            v-if="canPromote && canSave"
            data-testid="agent-center-promote-team-button"
            variant="outline"
            size="sm"
            :loading="promoting"
            loading-label="Promoting"
            @click="emit('promote')"
          >
            提升到工作区
          </UiButton>
        </div>
        <div class="flex items-center gap-2">
          <UiButton variant="ghost" @click="emit('update:open', false)">{{ canSave ? '取消' : '关闭' }}</UiButton>
          <UiButton
            v-if="canCopy"
            data-testid="agent-center-copy-team-button"
            variant="outline"
            @click="emit('copy')"
          >
            {{ copyLabel }}
          </UiButton>
          <UiButton v-if="canSave" class="px-6" @click="emit('save')">保存配置</UiButton>
        </div>
      </div>
    </template>
  </UiDialog>
</template>
