<script setup lang="ts">
import { computed, ref } from 'vue'
import { MessageSquare, Save, X, Zap } from 'lucide-vue-next'
import { UiButton, UiSurface, UiTabs, UiTextarea, UiInput, UiField, UiSearchableMultiSelect, UiSelect } from '@octopus/ui'

const props = defineProps<{
  form: any
  statusOptions: any[]
  builtinOptions: any[]
  skillOptions: any[]
  mcpOptions: any[]
  avatarPreview: string
  readonly: boolean
  loading?: boolean
}>()

const emit = defineEmits<{
  close: []
  save: []
}>()

const activeTab = ref('config')
const tabs = [
  { value: 'config', label: '配置' },
  { value: 'chat', label: '热重载调试' }
]

function initials(name: string) {
  return (name || 'A').slice(0, 1).toUpperCase()
}
</script>

<template>
  <div class="flex h-full w-[450px] flex-col border-l border-border bg-sidebar/30 backdrop-blur-xl animate-in slide-in-from-right duration-300">
    <!-- Header -->
    <div class="flex items-center justify-between border-b border-border/50 px-5 py-4 bg-black/10">
      <div class="flex items-center gap-3">
        <div class="flex size-8 items-center justify-center rounded-lg border border-primary/20 bg-primary/10 text-primary font-bold">
           <img v-if="avatarPreview" :src="avatarPreview" class="size-full object-cover rounded-lg" />
           <span v-else>{{ initials(form.name) }}</span>
        </div>
        <div class="min-w-0">
          <div class="truncate text-sm font-bold text-text-primary">{{ form.name || '未命名 Agent' }}</div>
          <div class="text-[10px] font-bold uppercase tracking-wider text-text-tertiary">Inspector</div>
        </div>
      </div>
      <div class="flex items-center gap-2">
        <UiButton v-if="!readonly" variant="primary" size="sm" :loading="loading" @click="emit('save')">
          <Save :size="14" class="mr-1.5" />
          保存
        </UiButton>
        <UiButton variant="ghost" size="icon" class="h-8 w-8" @click="emit('close')">
          <X :size="16" />
        </UiButton>
      </div>
    </div>

    <!-- Tabs -->
    <div class="px-5 pt-4">
      <UiTabs v-model="activeTab" :tabs="tabs" variant="segmented" />
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-y-auto p-5 scroll-y">
      <div v-if="activeTab === 'config'" class="space-y-6">
        <UiSurface variant="glass" padding="md" title="认知逻辑" subtitle="System Prompt & Personality">
          <div class="space-y-4">
            <UiField label="性格设定">
              <UiInput v-model="form.personality" :disabled="readonly" class="bg-black/10" />
            </UiField>
            <UiField label="系统指令">
              <UiTextarea v-model="form.prompt" :rows="10" :disabled="readonly" class="font-mono text-xs bg-black/20 text-primary/80" />
            </UiField>
          </div>
        </UiSurface>

        <UiSurface variant="glass" padding="md" title="能力集" subtitle="Tools & Skills">
          <div class="space-y-4">
             <UiField label="内置工具">
               <UiSearchableMultiSelect v-model="form.builtinToolKeys" :options="builtinOptions" :disabled="readonly" />
             </UiField>
             <UiField label="扩展技能">
               <UiSearchableMultiSelect v-model="form.skillIds" :options="skillOptions" :disabled="readonly" />
             </UiField>
          </div>
        </UiSurface>
      </div>

      <div v-else-if="activeTab === 'chat'" class="h-full flex flex-col items-center justify-center text-center space-y-4">
        <div class="size-16 rounded-full bg-primary/5 flex items-center justify-center relative">
           <Zap :size="32" class="text-primary animate-pulse" />
           <div class="absolute inset-0 rounded-full border border-primary/20 animate-ping" />
        </div>
        <div class="space-y-1">
          <div class="text-sm font-bold text-text-primary">热重载调试台</div>
          <p class="text-xs text-text-tertiary max-w-[200px]">在此处直接与 Agent 对话，修改 Prompt 后立即生效。</p>
        </div>
        <UiButton variant="outline" size="sm" class="mt-4">
          <MessageSquare :size="14" class="mr-2" />
          启动测试会话
        </UiButton>
      </div>
    </div>
  </div>
</template>
