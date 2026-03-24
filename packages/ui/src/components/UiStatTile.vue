<script setup lang="ts">
defineProps<{
  label: string
  value: string
  caption: string
}>()
</script>

<template>
  <div class="rounded-[22px] border border-[color:var(--oc-border-subtle)] bg-[color:var(--oc-bg-panel)] p-4">
    <p class="text-xs uppercase tracking-[0.2em] text-[color:var(--oc-text-muted)]">
      {{ label }}
    </p>
    <p class="mt-4 font-[var(--oc-font-display)] text-4xl leading-none tracking-tight">
      {{ value }}
    </p>
    <p class="mt-3 text-sm text-[color:var(--oc-text-secondary)]">
      {{ caption }}
    </p>
  </div>
</template>
