<script setup lang="ts">
const props = defineProps<{
  count: number
}>()
</script>

<template>
  <span
    v-if="props.count > 0"
    class="inline-flex min-w-5 items-center justify-center rounded-full border border-border-strong bg-accent px-1.5 py-0.5 text-micro font-semibold text-accent-foreground"
    data-testid="ui-notification-badge"
  >
    {{ props.count > 99 ? '99+' : props.count }}
  </span>
</template>
