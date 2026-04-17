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
  <article class="flex min-w-0 flex-col overflow-hidden rounded-[var(--radius-l)] border border-[color-mix(in_srgb,var(--border)_76%,transparent)] bg-[color-mix(in_srgb,var(--surface)_84%,var(--subtle)_16%)]">
    <div class="flex flex-wrap items-start justify-between gap-3 border-b border-border bg-subtle px-4 py-3">
      <strong class="text-sm font-semibold text-text-primary">{{ props.title }}</strong>
      <div class="flex shrink-0 flex-col items-end gap-1">
        <span v-if="props.timestampLabel" class="text-[11px] font-medium text-text-tertiary">
          {{ props.timestampLabel }}
        </span>
        <span
          data-testid="ui-inbox-block-priority"
          class="rounded-full border border-border-strong bg-accent px-2.5 py-1 text-[11px] font-semibold text-text-primary"
        >
          {{ props.priorityLabel }}
        </span>
      </div>
    </div>

    <div class="space-y-3 px-4 py-4">
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
    </div>

    <div v-if="$slots.actions" class="flex flex-wrap gap-2 border-t border-border bg-[color-mix(in_srgb,var(--surface)_76%,var(--subtle)_24%)] px-4 py-3">
      <slot name="actions" />
    </div>
  </article>
</template>
