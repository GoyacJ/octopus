<script setup lang="ts">
import { TabsList, TabsRoot, TabsTrigger } from 'reka-ui'
import { cn } from '../lib/utils'

export interface UiTabItem {
  value: string
  label: string
}

const props = withDefaults(defineProps<{
  modelValue: string
  tabs: UiTabItem[]
  variant?: 'default' | 'pill' | 'segmented'
  testId?: string
}>(), {
  variant: 'default',
  testId: '',
})

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()
</script>

<template>
  <TabsRoot
    :model-value="props.modelValue"
    :data-testid="props.testId || undefined"
    @update:model-value="emit('update:modelValue', $event)"
  >
    <TabsList
      data-testid="ui-tabs-list"
      :class="cn(
        'inline-flex min-w-0 flex-wrap gap-1 w-full',
        props.variant === 'default' && 'border-b border-border',
        props.variant !== 'default' && 'rounded-[var(--radius-m)] border border-border bg-surface p-1',
      )"
    >
      <TabsTrigger
        v-for="tab in props.tabs"
        :key="tab.value"
        :value="tab.value"
        type="button"
        :class="cn(
          'min-h-[2rem] px-3 text-[13px] font-medium transition-colors',
          'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:rounded-[var(--radius-xs)]',
          props.variant === 'default' && 'border-b-2 -mb-px pb-1.5',
          props.variant !== 'default' && 'rounded-[var(--radius-xs)]',
          props.variant === 'default' && 'data-[state=active]:border-primary data-[state=active]:text-text-primary data-[state=inactive]:border-transparent',
          props.variant !== 'default' && 'data-[state=active]:bg-accent data-[state=active]:text-text-primary data-[state=active]:shadow-xs',
          'data-[state=inactive]:text-text-tertiary data-[state=inactive]:hover:text-text-secondary'
        )"
        :data-testid="`ui-tabs-trigger-${tab.value}`"
        @click="emit('update:modelValue', tab.value)"
      >
        {{ tab.label }}
      </TabsTrigger>
    </TabsList>
  </TabsRoot>
</template>
