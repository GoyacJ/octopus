<script setup lang="ts">
import { cn } from '../lib/utils'

const props = defineProps<{
  title: string
  excerpt: string
  typeLabel: string
  versionLabel: string
  statusLabel: string
  class?: string
}>()
</script>

<template>
  <article
    data-ui-artifact-block="true"
    :class="cn(
      'flex flex-col overflow-hidden rounded-[var(--radius-l)] border border-[color-mix(in_srgb,var(--border)_76%,transparent)] bg-[color-mix(in_srgb,var(--surface)_84%,var(--subtle)_16%)] transition-colors hover:border-border-strong hover:bg-[color-mix(in_srgb,var(--surface)_74%,var(--subtle)_26%)]',
      props.class
    )"
  >
    <div
      data-testid="ui-artifact-block-header"
      class="flex items-center justify-between gap-4 border-b border-border bg-subtle px-4 py-3 text-micro font-bold uppercase tracking-wider text-text-tertiary"
    >
      <span>{{ props.typeLabel }}</span>
      <span>{{ props.versionLabel }}</span>
    </div>

    <div class="space-y-3 px-4 py-4">
      <strong class="text-card-title font-bold text-text-primary line-clamp-1">{{ props.title }}</strong>

      <p class="text-caption text-text-secondary line-clamp-2 break-words">
        {{ props.excerpt }}
      </p>
    </div>

    <footer
      data-testid="ui-artifact-block-footer"
      class="mt-auto flex items-center justify-between gap-4 border-t border-border bg-[color-mix(in_srgb,var(--surface)_76%,var(--subtle)_24%)] px-4 py-3"
    >
      <span class="text-micro font-medium text-text-tertiary">{{ props.statusLabel }}</span>
      <div v-if="$slots.actions" class="flex items-center gap-1.5">
        <slot name="actions" />
      </div>
    </footer>
  </article>
</template>
