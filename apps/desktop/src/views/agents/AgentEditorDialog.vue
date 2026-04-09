<script setup lang="ts">
import { computed } from 'vue'

import { UiButton, UiDialog, UiField, UiInput, UiSearchableMultiSelect, UiSelect, UiSurface, UiTextarea } from '@octopus/ui'

import type { AgentFormState, SelectOption } from './useAgentCenter'

const props = defineProps<{
  open: boolean
  form: AgentFormState
  statusOptions: Array<{ value: string, label: string }>
  builtinOptions: SelectOption[]
  skillOptions: SelectOption[]
  mcpOptions: SelectOption[]
  avatarPreview: string
  scope: string
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
    title="员工配置"
    description="配置数字员工的基本信息、核心性格与工具能力。"
    content-class="max-w-3xl"
    body-class="max-h-[75vh] overflow-y-auto pr-1"
    content-test-id="agent-center-agent-dialog"
  >
    <div class="flex flex-col gap-8 py-2">
      <UiSurface variant="subtle" padding="md" title="基本信息" subtitle="设置员工的展示名称、状态和视觉标识。" class="space-y-4">
        <div class="grid gap-4 md:grid-cols-2">
          <UiField label="员工名称">
            <UiInput v-model="form.name" placeholder="例如: 研发专家" />
          </UiField>
          <UiField label="状态">
            <UiSelect v-model="form.status" :options="statusOptions" />
          </UiField>
          <UiField class="md:col-span-2" label="头像标识">
            <div class="flex items-center gap-4 rounded-[var(--radius-l)] border border-border bg-surface p-4">
              <div class="flex size-14 shrink-0 items-center justify-center overflow-hidden rounded-[var(--radius-l)] border border-border bg-subtle text-lg font-bold text-primary">
                <img v-if="avatarPreview" :src="avatarPreview" alt="" class="size-full object-cover">
                <span v-else>{{ initials(form.name || 'A') }}</span>
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
          <UiField class="md:col-span-2" label="核心摘要">
            <UiTextarea v-model="form.description" :rows="2" placeholder="简要描述该员工的主要职责..." />
          </UiField>
        </div>
      </UiSurface>

      <UiSurface variant="subtle" padding="md" title="性格与提示词" subtitle="定义员工的行事风格、专业背景和核心工作指令。" class="space-y-4">
        <div class="grid gap-4 md:grid-cols-1">
          <UiField label="性格设定">
            <UiInput v-model="form.personality" placeholder="例如: 严谨、专业、富有逻辑" />
          </UiField>
          <UiField label="系统提示词 (System Prompt)">
            <UiTextarea v-model="form.prompt" :rows="6" class="font-mono text-sm" placeholder="编写核心指令..." />
          </UiField>
          <UiField label="分类标签">
            <UiInput v-model="form.tagsText" placeholder="用逗号分隔，例如: 开发, 代码审查, Rust" />
          </UiField>
        </div>
      </UiSurface>

      <UiSurface variant="subtle" padding="md" title="能力与工具" subtitle="通过内置工具、扩展技能和 MCP 服务增强员工能力。" class="space-y-4">
        <div class="grid gap-4 md:grid-cols-2">
          <UiField label="内置工具">
            <UiSearchableMultiSelect
              v-model="form.builtinToolKeys"
              :options="builtinOptions"
              placeholder="搜索并选择工具"
            />
          </UiField>
          <UiField label="技能插件 (Skills)">
            <UiSearchableMultiSelect
              v-model="form.skillIds"
              :options="skillOptions"
              placeholder="搜索并选择技能"
            />
          </UiField>
          <UiField class="md:col-span-2" label="MCP 服务集成">
            <UiSearchableMultiSelect
              v-model="form.mcpServerNames"
              :options="mcpOptions"
              placeholder="搜索并选择 MCP 服务器"
            />
          </UiField>
        </div>
      </UiSurface>
    </div>
    <template #footer>
      <div class="flex w-full items-center justify-between">
        <span class="text-xs text-text-tertiary">所有更改将立即同步至 {{ scope }}</span>
        <div class="flex items-center gap-2">
          <UiButton variant="ghost" @click="emit('update:open', false)">取消</UiButton>
          <UiButton class="px-6" @click="emit('save')">保存配置</UiButton>
        </div>
      </div>
    </template>
  </UiDialog>
</template>
