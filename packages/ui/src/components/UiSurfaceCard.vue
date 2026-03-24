<script setup lang="ts">
withDefaults(
  defineProps<{
    eyebrow?: string
    title: string
    body: string
  }>(),
  {
    eyebrow: undefined,
  },
)
</script>

<template>
  <section
    class="rounded-[24px] border border-[color:var(--oc-border-subtle)] bg-[color:var(--oc-bg-surface)] p-5 shadow-[var(--oc-shadow-card)]"
  >
    <p
      v-if="eyebrow"
      class="text-[11px] uppercase tracking-[0.22em] text-[color:var(--oc-text-muted)]"
    >
      {{ eyebrow }}
    </p>
    <h2 class="mt-2 font-[var(--oc-font-display)] text-2xl leading-tight tracking-tight">
      {{ title }}
    </h2>
    <p class="mt-3 max-w-2xl text-sm leading-7 text-[color:var(--oc-text-secondary)]">
      {{ body }}
    </p>
    <div class="mt-4">
      <slot />
    </div>
  </section>
</template>
