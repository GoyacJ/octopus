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
    'flex h-8 w-full appearance-none rounded-[var(--radius-xs)] border border-input bg-background px-3 pr-8 text-label text-text-primary outline-none transition-colors duration-fast focus-visible:border-primary focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background disabled:cursor-not-allowed disabled:opacity-50',
    props.class
  )
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
