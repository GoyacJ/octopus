<script setup lang="ts">
const props = defineProps<{
  label: string
  hint?: string
  error?: string
}>()
</script>

<template>
  <label class="flex flex-col gap-2">
    <span class="text-sm font-semibold">{{ props.label }}</span>
    <slot />
    <small v-if="props.error" class="text-sm text-destructive">{{ props.error }}</small>
    <small v-else-if="props.hint" class="text-sm text-muted-foreground">{{ props.hint }}</small>
  </label>
</template>
