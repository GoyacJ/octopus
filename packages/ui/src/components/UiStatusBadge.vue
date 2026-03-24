<script setup lang="ts">
withDefaults(
  defineProps<{
    label: string
    tone?: 'accent' | 'success' | 'warning'
  }>(),
  {
    tone: 'accent',
  },
)
</script>

<template>
  <span
    class="inline-flex items-center rounded-full px-3 py-1 text-[11px] font-semibold uppercase tracking-[0.18em]"
    :class="{
      'bg-[color:var(--oc-accent-soft)] text-[color:var(--oc-accent)]': tone === 'accent',
      'bg-emerald-500/14 text-[color:var(--oc-success)]': tone === 'success',
      'bg-amber-500/14 text-[color:var(--oc-warning)]': tone === 'warning',
    }"
  >
    {{ label }}
  </span>
</template>
