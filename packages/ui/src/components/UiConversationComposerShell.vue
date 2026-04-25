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
      'overflow-hidden rounded-[var(--radius-xl)] border border-primary/20 bg-sidebar/40 backdrop-blur-xl shadow-xl highlight-border',
      props.class,
    )"
  >
    <slot />
  </div>
</template>
