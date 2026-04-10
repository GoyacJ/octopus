<script setup lang="ts">
import { ChevronDown, Trash2 } from 'lucide-vue-next'
import { computed } from 'vue'
import { UiBadge, UiButton, UiCheckbox, UiDropdownMenu } from '@octopus/ui'

const props = defineProps<{
  id: string
  name: string
  role: string
  summary: string
  recentTask: string
  avatar: string
  statusLabel: string
  statusTone: 'success' | 'warning' | 'default'
  skills: string[]
  metrics: Array<{ label: string; value: string }>
  originLabel?: string
  openLabel?: string
  removeLabel?: string
  openTestId?: string
  removeTestId?: string
  selected?: boolean
  selectionTestId?: string
  selectable?: boolean
  exportable?: boolean
  removable?: boolean
}>()

const emit = defineEmits<{
  open: [id: string]
  remove: [id: string]
  export: [format: 'folder' | 'zip']
  'update:selected': [value: boolean]
}>()

const statusBadgeTone = computed(() =>
  props.statusTone === 'success' ? 'success' : props.statusTone === 'warning' ? 'warning' : 'default',
)

const exportMenuItems = [
  { key: 'export-folder', label: '导出为文件夹' },
  { key: 'export-zip', label: '导出为 ZIP' },
]

function openCard() {
  emit('open', props.id)
}
</script>

<template>
  <div
    class="group flex min-h-[208px] w-full cursor-pointer flex-col justify-between gap-4 rounded-[var(--radius-l)] border border-border bg-surface p-4 transition-colors hover:border-primary/30 hover:bg-subtle/60"
    role="button"
    tabindex="0"
    @click="openCard"
    @keydown.enter.prevent="openCard"
    @keydown.space.prevent="openCard"
  >
    <div class="space-y-4">
      <div class="flex items-start gap-3">
        <div class="flex size-11 shrink-0 items-center justify-center overflow-hidden rounded-[var(--radius-m)] border border-border bg-subtle text-[13px] font-semibold text-text-secondary">
          <img v-if="props.avatar.startsWith('data:image/')" :src="props.avatar" alt="" class="size-full object-cover">
          <span v-else>{{ props.avatar.slice(0, 2).toUpperCase() }}</span>
        </div>

        <div class="min-w-0 flex-1 space-y-2">
          <div class="flex flex-wrap items-center gap-2">
            <h3 class="truncate text-[15px] font-semibold text-text-primary">{{ props.name }}</h3>
            <UiBadge :label="props.statusLabel" :tone="statusBadgeTone" />
            <UiBadge v-if="props.originLabel" :label="props.originLabel" subtle />
          </div>
          <p class="text-[11px] font-semibold uppercase tracking-[0.12em] text-text-tertiary">
            {{ props.role || 'Agent' }}
          </p>
        </div>

        <div v-if="props.selectable !== false" class="shrink-0" @click.stop @keydown.stop>
          <UiCheckbox
            :model-value="Boolean(props.selected)"
            :data-testid="props.selectionTestId"
            @update:model-value="emit('update:selected', Boolean($event))"
          />
        </div>
      </div>

      <p class="line-clamp-3 text-[13px] leading-6 text-text-secondary">
        {{ props.summary || props.recentTask }}
      </p>

      <div class="flex flex-wrap gap-2">
        <UiBadge
          v-for="skill in props.skills.slice(0, 3)"
          :key="skill"
          :label="skill"
          subtle
        />
      </div>
    </div>

    <div class="space-y-3 border-t border-border pt-3">
      <div class="grid grid-cols-2 gap-3">
        <div v-for="metric in props.metrics" :key="metric.label" class="space-y-1">
          <p class="text-[10px] font-semibold uppercase tracking-[0.12em] text-text-tertiary">
            {{ metric.label }}
          </p>
          <p class="text-[13px] font-semibold tabular-nums text-text-primary">
            {{ metric.value }}
          </p>
        </div>
      </div>

      <div class="flex items-center justify-between gap-3">
        <UiButton
          variant="ghost"
          size="sm"
          class="h-8 px-3 text-[12px]"
          :data-testid="props.openTestId"
          @click.stop="openCard"
        >
          {{ props.openLabel || '打开' }}
        </UiButton>
        <div class="flex items-center gap-1">
          <UiDropdownMenu v-if="props.exportable !== false" :items="exportMenuItems" @select="emit('export', $event === 'export-zip' ? 'zip' : 'folder')">
            <template #trigger>
              <UiButton
                variant="ghost"
                size="sm"
                class="h-8 px-3 text-[12px]"
                :aria-label="`导出 ${props.name}`"
                @click.stop
              >
                导出
                <ChevronDown :size="12" />
              </UiButton>
            </template>
          </UiDropdownMenu>
          <UiButton
            v-if="props.removable !== false"
            variant="ghost"
            size="icon"
            class="size-8 text-text-tertiary hover:text-status-error"
            :aria-label="props.removeLabel || '删除'"
            :data-testid="props.removeTestId"
            @click.stop="emit('remove', props.id)"
          >
            <Trash2 :size="14" />
            <span class="sr-only">{{ props.removeLabel || '删除' }}</span>
          </UiButton>
        </div>
      </div>
    </div>
  </div>
</template>
