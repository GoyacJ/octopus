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
    title="数字员工元数据配置"
    description="精确定义数字员工的认知架构、性格指纹及工具集连通性。"
    content-class="max-w-3xl"
    body-class="max-h-[75vh] overflow-y-auto pr-1"
    content-test-id="agent-center-agent-dialog"
  >
    <div class="flex flex-col gap-6 py-2">
      <UiSurface variant="glass" padding="md" title="身份标识" subtitle="设置员工的逻辑名称、状态及视觉识别特征。" class="space-y-4">
        <div class="grid gap-4 md:grid-cols-2">
          <UiField label="员工名称">
            <UiInput v-model="form.name" :disabled="contentReadonly" placeholder="例如: 研发专家" class="bg-black/10" />
          </UiField>
          <UiField label="运行状态">
            <UiSelect v-model="form.status" :options="statusOptions" :disabled="statusReadonly" class="bg-black/10" />
          </UiField>
          <UiField class="md:col-span-2" label="视觉识别指纹">
            <div class="flex items-center gap-6 rounded-[var(--radius-xl)] border border-border/50 bg-black/10 p-4">
              <div class="relative group">
                <div class="flex size-16 shrink-0 items-center justify-center overflow-hidden rounded-[var(--radius-l)] border border-primary/20 bg-primary/5 text-xl font-bold text-primary shadow-[0_0_15px_rgba(var(--color-primary-rgb),0.1)] transition-all group-hover:shadow-[0_0_20px_rgba(var(--color-primary-rgb),0.2)]">
                  <img v-if="avatarPreview" :src="avatarPreview" alt="" class="size-full object-cover">
                  <span v-else>{{ initials(form.name || 'A') }}</span>
                </div>
                <div class="absolute -inset-1 rounded-[var(--radius-l)] border border-primary/10 pointer-events-none opacity-50" />
              </div>
              <div v-if="!contentReadonly" class="flex flex-col gap-2">
                <div class="flex gap-2">
                  <UiButton variant="outline" size="sm" class="h-8 bg-surface/50" @click="emit('pick-avatar')">更换身份标识</UiButton>
                  <UiButton v-if="avatarPreview" variant="ghost" size="sm" class="h-8 text-text-tertiary" @click="emit('remove-avatar')">重置</UiButton>
                </div>
                <p class="text-[11px] text-text-tertiary leading-normal">识别特征将作为 Agent 在协作流中的唯一视觉锚点。</p>
              </div>
            </div>
          </UiField>
          <UiField class="md:col-span-2" label="职责纲要">
            <UiTextarea v-model="form.description" :rows="2" :disabled="contentReadonly" placeholder="简要描述该员工的主要职责..." class="bg-black/10" />
          </UiField>
        </div>
      </UiSurface>

      <UiSurface variant="glass" padding="md" title="认知逻辑与性格指纹" subtitle="定义员工的行事风格、专业背景和核心工作指令。" class="space-y-4">
        <div class="grid gap-4 md:grid-cols-1">
          <UiField label="性格指纹 (Personality Fingerprint)">
            <UiInput v-model="form.personality" :disabled="contentReadonly" placeholder="例如: 严谨、专业、具有深度逻辑推理能力" class="bg-black/10" />
          </UiField>
          <UiField label="核心系统指令 (System Prompt)">
            <UiTextarea v-model="form.prompt" :rows="8" :disabled="contentReadonly" class="font-mono text-[13px] bg-black/20 text-primary/90" placeholder="编写核心指令集..." />
          </UiField>
          <UiField label="领域标签">
            <UiInput v-model="form.tagsText" :disabled="contentReadonly" placeholder="用逗号分隔，例如: 开发, 代码审查, Rust" class="bg-black/10" />
          </UiField>
        </div>
      </UiSurface>

      <UiSurface variant="glass" padding="md" title="能力扩展集" subtitle="通过内置工具、扩展技能和 MCP 服务增强员工能力架构。" class="space-y-4">
        <div class="grid gap-4 md:grid-cols-2">
          <UiField label="原生能力集">
            <UiSearchableMultiSelect
              v-model="form.builtinToolKeys"
              :options="builtinOptions"
              :disabled="contentReadonly"
              placeholder="搜索并挂载工具"
              class="bg-black/10"
            />
          </UiField>
          <UiField label="扩展插件技能 (Skills)">
            <UiSearchableMultiSelect
              v-model="form.skillIds"
              :options="skillOptions"
              :disabled="contentReadonly"
              placeholder="搜索并挂载 Skill"
              class="bg-black/10"
            />
          </UiField>
          <UiField class="md:col-span-2" label="MCP 协议服务集成">
            <UiSearchableMultiSelect
              v-model="form.mcpServerNames"
              :options="mcpOptions"
              :disabled="contentReadonly"
              placeholder="搜索并挂载 MCP 服务器"
              class="bg-black/10"
            />
          </UiField>
        </div>
      </UiSurface>
    </div>
    <template #footer>
      <div class="flex w-full items-center justify-between">
        <div class="flex items-center gap-2">
          <span class="text-xs text-text-tertiary">所有更改将立即同步至 {{ scope }}</span>
          <UiButton
            v-if="canPromote && canSave"
            data-testid="agent-center-promote-agent-button"
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
            data-testid="agent-center-copy-agent-button"
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
