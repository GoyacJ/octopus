<script setup lang="ts">
import { cn } from '../lib/utils'

const props = withDefaults(defineProps<{
  class?: string
}>(), {
  class: '',
})
</script>

<template>
  <div
    data-testid="ui-conversation-composer-shell"
    :class="cn(
      'overflow-hidden rounded-[var(--radius-xl)] border border-[color-mix(in_srgb,var(--border)_76%,transparent)] bg-[color-mix(in_srgb,var(--surface)_82%,var(--subtle)_18%)] p-3 shadow-none',
      props.class,
    )"
  >
    <slot />
  </div>
</template>
