<script setup lang="ts">
import { ref } from 'vue'
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

const pendingValue = ref<string | null>(null)

function emitValue(value: string) {
  if (pendingValue.value === value) {
    return
  }

  pendingValue.value = value
  emit('update:modelValue', value)
  queueMicrotask(() => {
    if (pendingValue.value === value) {
      pendingValue.value = null
    }
  })
}
</script>

<template>
  <TabsRoot
    :model-value="props.modelValue"
    :data-testid="props.testId || undefined"
    @update:model-value="emitValue"
  >
    <TabsList
      data-testid="ui-tabs-list"
      :class="cn(
        'inline-flex min-w-0 flex-wrap gap-2 w-full',
        props.variant === 'default' && 'border-b border-border/50 pb-0',
        props.variant !== 'default' && 'rounded-[var(--radius-l)] border border-border bg-subtle/50 p-1 backdrop-blur-sm',
      )"
    >
      <TabsTrigger
        v-for="tab in props.tabs"
        :key="tab.value"
        :value="tab.value"
        type="button"
        :class="cn(
          'relative px-4 py-2 text-[13px] font-bold tracking-tight transition-all duration-normal',
          'ui-focus-ring focus-visible:rounded-[var(--radius-m)]',
          props.variant === 'default' && 'pb-3',
          props.variant !== 'default' && 'rounded-[var(--radius-m)]',
          'data-[state=active]:text-text-primary data-[state=inactive]:text-text-tertiary data-[state=inactive]:hover:text-text-secondary'
        )"
        :data-testid="`ui-tabs-trigger-${tab.value}`"
        @click="emitValue(tab.value)"
      >
        <span class="relative z-10">{{ tab.label }}</span>
        
        <!-- Default Variant Underline -->
        <div 
          v-if="props.variant === 'default' && props.modelValue === tab.value"
          class="absolute bottom-0 left-0 right-0 h-0.5 bg-primary shadow-[0_0_8px_var(--color-primary)] transition-all"
        />

        <!-- Non-default Variant Background -->
        <div 
          v-if="props.variant !== 'default' && props.modelValue === tab.value"
          class="absolute inset-0 rounded-[var(--radius-m)] bg-primary shadow-sm shadow-primary/20 transition-all"
        />
      </TabsTrigger>
    </TabsList>
  </TabsRoot>
</template>
