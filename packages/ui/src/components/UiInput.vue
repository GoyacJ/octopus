<script setup lang="ts">
import { computed, ref } from 'vue'
import { cn } from '../lib/utils'

interface Props {
  modelValue?: string | number
  type?: string
  placeholder?: string
  disabled?: boolean
  readonly?: boolean
  class?: string
}

const props = withDefaults(defineProps<Props>(), {
  modelValue: '',
  type: 'text',
  placeholder: '',
  disabled: false,
  readonly: false,
  class: '',
})

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()

const inputElement = ref<HTMLInputElement | null>(null)

const classes = computed(() =>
  cn(
    'flex h-8 w-full rounded-md border border-border/60 dark:border-white/[0.03] bg-background px-3 py-1.5 text-sm text-text-primary shadow-[inset_0_1px_2px_rgba(15,15,15,0.03)] ring-offset-background file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-text-tertiary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:border-primary disabled:cursor-not-allowed disabled:opacity-50 transition-all duration-fast',
    props.class,
  ),
)

defineExpose({
  focus: () => inputElement.value?.focus(),
  el: inputElement,
})
</script>

<template>
  <input
    ref="inputElement"
    :value="modelValue"
    :type="type"
    :placeholder="placeholder"
    :disabled="disabled"
    :readonly="readonly"
    :class="classes"
    @input="emit('update:modelValue', ($event.target as HTMLInputElement).value)"
  >
</template>
