<script setup lang="ts">
const props = withDefaults(defineProps<{
  modelValue?: boolean
  disabled?: boolean
  label?: string
}>(), {
  modelValue: false,
  disabled: false,
  label: '',
})

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
}>()
</script>

<template>
  <label 
    class="inline-flex items-center gap-2.5 min-w-0 cursor-pointer"
    :class="{ 'opacity-50 cursor-not-allowed': props.disabled }"
  >
    <button
      type="button"
      role="switch"
      :aria-checked="props.modelValue"
      :disabled="props.disabled"
      class="relative inline-flex h-5 w-9 shrink-0 items-center rounded-full border border-transparent transition-colors duration-fast focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1 disabled:cursor-not-allowed"
      :class="[
        props.modelValue ? 'bg-primary' : 'bg-border-strong/60 dark:border-white/[0.1]'
      ]"
      @click="emit('update:modelValue', !props.modelValue)"
    >
      <span 
        class="pointer-events-none block h-3.5 w-3.5 rounded-full bg-white shadow-sm transition-transform duration-fast"
        :class="[
          props.modelValue ? 'translate-x-[18px]' : 'translate-x-0.5'
        ]"
      />
    </button>
    <span v-if="props.label || $slots.default" class="text-[13px] text-text-primary select-none">
      <slot>{{ props.label }}</slot>
    </span>
  </label>
</template>