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
  sticky?: boolean
  testId?: string
}>(), {
  variant: 'default',
  sticky: false,
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
        'inline-flex min-w-0 flex-wrap gap-2 w-full transition-all duration-normal',
        props.sticky && 'sticky top-0 z-20 bg-background/60 backdrop-blur-xl border-b border-border/40 py-2 -mx-4 px-4 w-[calc(100%+2rem)]',
        props.variant === 'default' && !props.sticky && 'border-b border-border/50 pb-0',
        props.variant !== 'default' && 'rounded-[var(--radius-xl)] border border-border/60 bg-surface-muted/30 p-1 backdrop-blur-sm',
      )"
    >
      <TabsTrigger
        v-for="tab in props.tabs"
        :key="tab.value"
        :value="tab.value"
        type="button"
        :class="cn(
          'relative px-5 py-2 text-[13px] font-bold tracking-tight transition-all duration-normal ease-apple',
          'ui-focus-ring focus-visible:rounded-[var(--radius-m)]',
          props.variant === 'default' && 'pb-3',
          props.variant !== 'default' && 'rounded-[var(--radius-lg)]',
          'data-[state=active]:text-text-primary data-[state=inactive]:text-text-tertiary data-[state=inactive]:hover:text-text-secondary'
        )"
        :data-testid="`ui-tabs-trigger-${tab.value}`"
        @click="emitValue(tab.value)"
      >
        <span class="relative z-10 transition-colors duration-fast" :class="props.modelValue === tab.value ? 'text-white' : ''">
          {{ tab.label }}
        </span>
        
        <!-- Default Variant Underline (Hidden when sticky or non-default) -->
        <div 
          v-if="props.variant === 'default' && props.modelValue === tab.value && !props.sticky"
          class="absolute bottom-0 left-2 right-2 h-1 rounded-t-full bg-primary shadow-[0_0_12px_var(--color-primary)] transition-all"
        />

        <!-- Active Background (Pill Style) -->
        <div 
          v-if="(props.variant !== 'default' || props.sticky) && props.modelValue === tab.value"
          class="absolute inset-0 rounded-[var(--radius-lg)] bg-primary shadow-[var(--layer-glow-primary)] transition-all animate-in fade-in zoom-in-95 duration-fast"
        />
      </TabsTrigger>
    </TabsList>
  </TabsRoot>
</template>
