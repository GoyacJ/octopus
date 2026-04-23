<script setup lang="ts">
import UiSurface from './UiSurface.vue'

const props = withDefaults(defineProps<{
  title?: string
  subtitle?: string
  class?: string
}>(), {
  title: '',
  subtitle: '',
  class: '',
})
</script>

<template>
  <UiSurface
    data-testid="ui-inspector-panel"
    variant="panel"
    padding="none"
    :class="props.class"
  >
    <div
      v-if="props.title || props.subtitle || $slots.actions"
      data-testid="ui-inspector-panel-header"
      class="flex flex-wrap items-start justify-between gap-4 border-b border-border px-5 py-4"
    >
      <div v-if="props.title || props.subtitle" class="min-w-0 flex-1 space-y-1">
        <h2 v-if="props.title" class="text-card-title font-semibold leading-tight text-text-primary">
          {{ props.title }}
        </h2>
        <p v-if="props.subtitle" class="text-label leading-6 text-text-secondary">
          {{ props.subtitle }}
        </p>
      </div>

      <div v-if="$slots.actions" class="flex shrink-0 flex-wrap items-center gap-2">
        <slot name="actions" />
      </div>
    </div>

    <div data-testid="ui-inspector-panel-body" class="flex min-h-0 flex-col gap-4 px-5 py-5">
      <slot />
    </div>
  </UiSurface>
</template>
