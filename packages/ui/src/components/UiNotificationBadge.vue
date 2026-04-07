<script setup lang="ts">
const props = defineProps<{
  count: number
}>()
</script>

<template>
  <span
    v-if="props.count > 0"
    class="inline-flex min-w-5 items-center justify-center rounded-full bg-foreground px-1.5 py-0.5 text-[10px] font-semibold leading-none text-background"
    data-testid="ui-notification-badge"
  >
    {{ props.count > 99 ? '99+' : props.count }}
  </span>
</template>
