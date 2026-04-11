<script setup lang="ts">
const props = withDefaults(
  defineProps<{
    title: string
    description: string
    priorityLabel: string
    timestampLabel?: string
    statusLabel: string
    impact?: string
    riskNote?: string
    statusHeading?: string
    impactHeading?: string
    riskHeading?: string
  }>(),
  {
    statusHeading: 'Status',
    impactHeading: 'Impact',
    riskHeading: 'Risk',
    impact: '',
    riskNote: '',
  },
)
</script>

<template>
  <article class="flex min-w-0 flex-col gap-3 rounded-[var(--radius-l)] border border-border bg-surface p-4">
    <div class="flex flex-wrap items-start justify-between gap-3">
      <strong class="text-sm font-semibold text-text-primary">{{ props.title }}</strong>
      <div class="flex shrink-0 flex-col items-end gap-1">
        <span v-if="props.timestampLabel" class="text-[11px] font-medium text-text-tertiary">
          {{ props.timestampLabel }}
        </span>
        <span class="rounded-full bg-accent px-2.5 py-1 text-[11px] font-semibold text-text-secondary">
          {{ props.priorityLabel }}
        </span>
      </div>
    </div>

    <p class="text-sm leading-6 text-text-secondary">{{ props.description }}</p>

    <dl class="grid gap-3 sm:grid-cols-2">
      <div class="space-y-1">
        <dt class="text-[11px] font-semibold uppercase tracking-[0.08em] text-text-tertiary">{{ props.statusHeading }}</dt>
        <dd class="text-sm leading-6 text-text-secondary">{{ props.statusLabel }}</dd>
      </div>

      <div v-if="props.impact" class="space-y-1">
        <dt class="text-[11px] font-semibold uppercase tracking-[0.08em] text-text-tertiary">{{ props.impactHeading }}</dt>
        <dd class="text-sm leading-6 text-text-secondary">{{ props.impact }}</dd>
      </div>

      <div v-if="props.riskNote" class="space-y-1 sm:col-span-2">
        <dt class="text-[11px] font-semibold uppercase tracking-[0.08em] text-text-tertiary">{{ props.riskHeading }}</dt>
        <dd class="text-sm leading-6 text-text-secondary">{{ props.riskNote }}</dd>
      </div>
    </dl>

    <div v-if="$slots.actions" class="flex flex-wrap gap-2">
      <slot name="actions" />
    </div>
  </article>
</template>
