<script setup lang="ts">
import { cn } from '../lib/utils'

export interface UiFilterChipItem {
  value: string
  label: string
  disabled?: boolean
}

const props = withDefaults(defineProps<{
  modelValue?: string
  items: UiFilterChipItem[]
  allowEmpty?: boolean
  testId?: string
  class?: string
}>(), {
  modelValue: '',
  allowEmpty: true,
  testId: '',
  class: '',
})

const emit = defineEmits<{
  'update:modelValue': [value: string]
  select: [value: string]
}>()

function selectItem(value: string) {
  const nextValue = props.allowEmpty && props.modelValue === value ? '' : value
  emit('update:modelValue', nextValue)
  emit('select', nextValue)
}
</script>

<template>
  <div
    :data-testid="props.testId || undefined"
    :class="cn('flex flex-wrap items-center gap-2', props.class)"
  >
    <button
      v-for="item in props.items"
      :key="item.value"
      type="button"
      :data-testid="`ui-filter-chip-${item.value}`"
      :disabled="item.disabled"
      :aria-pressed="props.modelValue === item.value ? 'true' : 'false'"
      :class="cn(
        'inline-flex min-h-9 items-center rounded-full border px-3 py-1.5 text-sm font-medium transition-all duration-fast ease-apple',
        props.modelValue === item.value
          ? 'border-primary/30 dark:border-white/[0.08] bg-primary/[0.08] text-text-primary shadow-xs'
          : 'border-border/70 dark:border-white/[0.03] bg-[color-mix(in_srgb,var(--bg-surface)_92%,transparent)] text-text-secondary hover:border-border/60 hover:text-text-primary',
        item.disabled && 'cursor-not-allowed opacity-50',
      )"
      @click="selectItem(item.value)"
    >
      {{ item.label }}
    </button>
  </div>
</template>
