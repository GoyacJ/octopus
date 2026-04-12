<script setup lang="ts">
import { computed } from 'vue'
import { Check } from 'lucide-vue-next'
import { cn } from '../lib/utils'

const props = withDefaults(defineProps<{
  modelValue?: boolean | string[]
  value?: string
  disabled?: boolean
  label?: string
  class?: string
}>(), {
  modelValue: false,
  value: '',
  disabled: false,
  label: '',
  class: '',
})

const emit = defineEmits<{
  'update:modelValue': [value: boolean | string[]]
}>()

const checked = computed(() => Array.isArray(props.modelValue)
  ? props.modelValue.includes(props.value)
  : Boolean(props.modelValue))

function onChange() {
  if (props.disabled) return

  if (Array.isArray(props.modelValue)) {
    const current = new Set(props.modelValue)
    if (!checked.value) {
      current.add(props.value)
    } else {
      current.delete(props.value)
    }

    emit('update:modelValue', Array.from(current))
    return
  }

  emit('update:modelValue', !props.modelValue)
}
</script>

<template>
  <label
    :class="cn(
      'inline-flex items-center gap-2 min-w-0 select-none cursor-pointer text-sm leading-none transition-opacity',
      props.disabled && 'opacity-50 cursor-not-allowed',
      props.class
    )"
  >
    <input
      type="checkbox"
      class="sr-only"
      :checked="checked"
      :disabled="props.disabled"
      @change.stop="onChange"
    >
    <div
      class="flex size-[14px] shrink-0 items-center justify-center rounded-sm border transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
      :class="[
        checked ? 'border-primary bg-primary text-primary-foreground' : 'border-border/70 bg-transparent hover:border-text-tertiary'
      ]"
    >
      <Check v-if="checked" :size="10" stroke-width="3" />
    </div>
    <span v-if="props.label || $slots.default" class="text-text-primary text-[13px]">
      <slot>{{ props.label }}</slot>
    </span>
  </label>
</template>
