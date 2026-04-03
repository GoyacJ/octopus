<script setup lang="ts">
const props = defineProps<{
  label: string
  hint?: string
}>()
</script>

<template>
  <label class="flex flex-col gap-1.5 min-w-0">
    <span class="text-[12px] font-semibold text-text-secondary">{{ props.label }}</span>
    <slot />
    <small v-if="props.hint" class="text-[11px] text-text-tertiary leading-relaxed break-words">{{ props.hint }}</small>
  </label>
</template>
