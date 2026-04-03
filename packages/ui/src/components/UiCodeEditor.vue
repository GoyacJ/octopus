<script setup lang="ts">
import { computed } from 'vue'

import UiTextarea from './UiTextarea.vue'

const props = withDefaults(defineProps<{
  modelValue?: string
  language?: string
  readonly?: boolean
  theme?: string
}>(), {
  modelValue: '',
  language: 'plaintext',
  readonly: false,
  theme: 'octopus',
})

const emit = defineEmits<{
  'update:modelValue': [value: string]
  change: [value: string]
}>()

function onUpdate(value: string) {
  emit('update:modelValue', value)
  emit('change', value)
}

const displayValue = computed(() => props.modelValue)
</script>

<template>
  <div class="rounded-md border border-border-strong bg-subtle/30 p-1.5 transition-colors">
    <div class="mb-1 flex items-center justify-between px-1.5 text-[10px] font-bold uppercase tracking-wider text-text-tertiary opacity-60">
      <span>{{ props.language }}</span>
      <span>{{ props.theme }}</span>
    </div>
    <pre class="sr-only">{{ displayValue }}</pre>
    <UiTextarea
      data-testid="ui-code-editor-textarea"
      :model-value="props.modelValue"
      :rows="8"
      class="min-h-[10rem] border-0 bg-transparent font-mono text-[13px] shadow-none focus-visible:ring-0 leading-relaxed"
      :disabled="props.readonly"
      @update:model-value="onUpdate"
    />
  </div>
</template>
