<script setup lang="ts">
import { computed } from 'vue'

import { cn } from '../lib/utils'

const props = withDefaults(defineProps<{
  modelValue?: string
  rows?: number
  placeholder?: string
  disabled?: boolean
  class?: string
}>(), {
  modelValue: '',
  rows: 3,
  placeholder: '',
  disabled: false,
  class: '',
})

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()

const classes = computed(() =>
  cn(
    'flex w-full resize-y rounded-md border border-border-strong bg-background px-3 py-2 text-sm text-text-primary shadow-[inset_0_1px_2px_rgba(15,15,15,0.05)] placeholder:text-text-tertiary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:border-primary disabled:cursor-not-allowed disabled:opacity-50 transition-all duration-fast',
    props.class,
  ),
)
</script>

<template>
  <textarea
    :value="props.modelValue"
    :rows="props.rows"
    :placeholder="props.placeholder"
    :disabled="props.disabled"
    :class="classes"
    @input="emit('update:modelValue', ($event.target as HTMLTextAreaElement).value)"
  />
</template>