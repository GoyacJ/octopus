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
    class="flex flex-col sm:flex-row sm:justify-between sm:items-start gap-4 min-w-0 p-4 rounded-xl border transition-all duration-fast ease-apple"
    :class="[
      props.active 
        ? 'border-primary/30 bg-surface shadow-sm' 
        : 'border-border bg-subtle/50',
      props.interactive && !props.active ? 'hover:border-border-strong hover:bg-subtle hover:-translate-y-[1px]' : '',
      props.interactive ? 'cursor-pointer' : ''
    ]"
  >
    <div class="flex flex-col flex-1 gap-1.5 min-w-0">
      <p v-if="props.eyebrow" class="m-0 text-primary text-[0.72rem] font-bold tracking-[0.12em] uppercase">{{ props.eyebrow }}</p>
      <strong class="text-[0.98rem] leading-[1.35] font-semibold min-w-0 overflow-hidden text-ellipsis line-clamp-2 text-text-primary">{{ props.title }}</strong>
      <p v-if="props.subtitle" class="m-0 text-text-secondary leading-[1.5] min-w-0 overflow-hidden text-ellipsis line-clamp-2 break-words">{{ props.subtitle }}</p>
      <slot />
    </div>
    <div class="flex flex-col items-start sm:items-end gap-2.5 min-w-0">
      <div v-if="$slots.meta" class="flex flex-wrap justify-start sm:justify-end min-w-0 gap-2">
        <slot name="meta" />
      </div>
      <div v-if="$slots.actions" class="flex flex-wrap justify-start sm:justify-end min-w-0 gap-2">
        <slot name="actions" />
      </div>
    </div>
  </article>
</template>