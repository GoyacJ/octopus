<script setup lang="ts">
import { cn } from '../lib/utils'

export interface UiRadioOption {
  label: string
  value: string
  disabled?: boolean
}

const props = withDefaults(defineProps<{
  modelValue?: string
  options: UiRadioOption[]
  name?: string
  direction?: 'vertical' | 'horizontal'
  class?: string
}>(), {
  direction: 'vertical',
  class: '',
})

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()

function select(value: string, disabled?: boolean) {
  if (disabled) return
  emit('update:modelValue', value)
}
</script>

<template>
  <div 
    :class="cn(
      'flex gap-3 min-w-0',
      props.direction === 'vertical' ? 'flex-col' : 'flex-wrap items-center',
      props.class
    )"
  >
    <label
      v-for="option in props.options"
      :key="option.value"
      :class="cn(
        'group inline-flex items-center gap-2 cursor-pointer select-none text-sm transition-opacity',
        option.disabled && 'opacity-50 cursor-not-allowed'
      )"
      @click.prevent="select(option.value, option.disabled)"
    >
      <div 
        class="relative flex size-[14px] shrink-0 items-center justify-center rounded-full border transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        :class="[
          props.modelValue === option.value ? 'border-primary bg-primary' : 'border-border/70 bg-transparent hover:border-text-tertiary'
        ]"
      >
        <div 
          class="size-1.5 rounded-full bg-white transition-transform duration-fast scale-0"
          :class="[
            props.modelValue === option.value && 'scale-100'
          ]"
        />
      </div>
      <span class="text-text-primary text-[13px] leading-tight">{{ option.label }}</span>
    </label>
  </div>
</template>
