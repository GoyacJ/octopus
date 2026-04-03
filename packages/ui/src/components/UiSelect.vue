<script setup lang="ts">
defineOptions({
  inheritAttrs: false,
})

import { computed, useAttrs } from 'vue'
import { ChevronDown } from 'lucide-vue-next'
import { cn } from '../lib/utils'

export interface UiSelectOption {
  label: string
  value: string
  disabled?: boolean
}

const props = withDefaults(defineProps<{
  modelValue?: string
  options: UiSelectOption[]
  disabled?: boolean
  class?: string
}>(), {
  modelValue: '',
  disabled: false,
  class: '',
})

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()

const attrs = useAttrs()

const classes = computed(() =>
  cn(
    'flex h-8 w-full appearance-none rounded-md border border-border-strong bg-background px-3 py-1.5 pr-8 text-sm text-text-primary shadow-[inset_0_1px_2px_rgba(15,15,15,0.05)] outline-none transition-all duration-fast focus-visible:border-primary focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50',
    props.class,
  ),
)
</script>

<template>
  <div class="relative w-full">
    <select
      v-bind="attrs"
      :value="props.modelValue"
      :disabled="props.disabled"
      :class="classes"
      @change="emit('update:modelValue', ($event.target as HTMLSelectElement).value)"
    >
      <option
        v-for="option in props.options"
        :key="option.value"
        :value="option.value"
        :disabled="option.disabled"
      >
        {{ option.label }}
      </option>
    </select>
    <div class="pointer-events-none absolute inset-y-0 right-0 flex items-center pr-2 text-text-tertiary">
      <ChevronDown :size="14" />
    </div>
  </div>
</template>