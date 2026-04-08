<script setup lang="ts">
const props = withDefaults(
  defineProps<{
    title: string
    subtitle?: string
    eyebrow?: string
    active?: boolean
    interactive?: boolean
  }>(),
  {
    subtitle: '',
    eyebrow: '',
    active: false,
    interactive: false,
  },
)
</script>

<template>
  <article 
    class="flex flex-col sm:flex-row sm:justify-between sm:items-start gap-3 min-w-0 p-3 rounded-md transition-colors border border-transparent"
    :class="[
      props.active 
        ? 'bg-accent/50 border-border/40 dark:border-white/[0.08]' 
        : 'border-b border-b-border/15 dark:border-b-white/[0.02] rounded-none sm:rounded-md sm:border-b-transparent',
      props.interactive && !props.active ? 'hover:bg-accent' : '',
      props.interactive ? 'cursor-pointer' : ''
    ]"
  >
    <div class="flex flex-col flex-1 gap-1 min-w-0">
      <p v-if="props.eyebrow" class="m-0 text-text-tertiary text-[10px] font-bold tracking-[0.1em] uppercase">{{ props.eyebrow }}</p>
      <strong class="text-[14px] leading-snug font-semibold min-w-0 overflow-hidden text-ellipsis line-clamp-1 text-text-primary">{{ props.title }}</strong>
      <p v-if="props.subtitle" class="m-0 text-[13px] text-text-secondary leading-relaxed min-w-0 overflow-hidden text-ellipsis line-clamp-2">{{ props.subtitle }}</p>
      <div v-if="$slots.default" class="pt-1">
        <slot />
      </div>
    </div>
    <div class="flex flex-col items-start sm:items-end gap-2 min-w-0 shrink-0 pt-1 sm:pt-0">
      <div v-if="$slots.meta" class="flex flex-wrap justify-start sm:justify-end min-w-0 gap-2 text-[12px]">
        <slot name="meta" />
      </div>
      <div v-if="$slots.actions" class="flex flex-wrap justify-start sm:justify-end min-w-0 gap-1.5">
        <slot name="actions" />
      </div>
    </div>
  </article>
</template>
