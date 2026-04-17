<script setup lang="ts">
import { cn } from '../lib/utils'

const props = withDefaults(defineProps<{
  class?: string
  listClass?: string
  detailClass?: string
}>(), {
  class: '',
  listClass: '',
  detailClass: '',
})
</script>

<template>
  <section
    data-testid="ui-list-detail-shell"
    :class="cn(
      'grid min-h-0 gap-px overflow-hidden rounded-[var(--radius-xl)] border border-[color-mix(in_srgb,var(--border)_76%,transparent)] bg-border xl:grid-cols-[360px_minmax(0,1fr)]',
      props.class,
    )"
  >
    <aside
      data-testid="ui-list-detail-shell-list"
      :class="cn('min-w-0 bg-surface', props.listClass)"
    >
      <slot name="list" />
    </aside>
    <section
      data-testid="ui-list-detail-shell-detail"
      :class="cn('min-w-0 bg-[color-mix(in_srgb,var(--surface)_72%,var(--subtle)_28%)]', props.detailClass)"
    >
      <slot />
    </section>
  </section>
</template>
