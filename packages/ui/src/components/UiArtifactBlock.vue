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
    :class="cn(
      'flex flex-col gap-2 p-3 rounded-md border border-border-subtle bg-background transition-all hover:border-border-strong',
      props.class
    )"
  >
    <div class="flex items-center justify-between gap-4 text-[10px] font-bold uppercase tracking-wider text-text-tertiary">
      <span>{{ props.typeLabel }}</span>
      <span>{{ props.versionLabel }}</span>
    </div>
    
    <strong class="text-[14px] font-bold text-text-primary leading-tight line-clamp-1">{{ props.title }}</strong>
    
    <p class="text-[12px] leading-relaxed text-text-secondary line-clamp-2 break-words">
      {{ props.excerpt }}
    </p>
    
    <footer class="flex items-center justify-between gap-4 pt-1 mt-auto border-t border-border-subtle/50 pt-2">
      <span class="text-[11px] font-medium text-text-tertiary">{{ props.statusLabel }}</span>
      <div v-if="$slots.actions" class="flex items-center gap-1.5">
        <slot name="actions" />
      </div>
    </footer>
  </article>
</template>
