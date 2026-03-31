<script setup lang="ts">
const props = withDefaults(
  defineProps<{
    title: string
    description: string
    priorityLabel: string
    statusLabel: string
    impact: string
    riskNote: string
    statusHeading?: string
    impactHeading?: string
    riskHeading?: string
  }>(),
  {
    statusHeading: 'Status',
    impactHeading: 'Impact',
    riskHeading: 'Risk',
  },
)
</script>

<template>
  <article class="inbox-block">
    <div class="topline">
      <strong>{{ props.title }}</strong>
      <span>{{ props.priorityLabel }}</span>
    </div>
    <p>{{ props.description }}</p>
    <dl>
      <div>
        <dt>{{ props.statusHeading }}</dt>
        <dd>{{ props.statusLabel }}</dd>
      </div>
      <div>
        <dt>{{ props.impactHeading }}</dt>
        <dd>{{ props.impact }}</dd>
      </div>
      <div>
        <dt>{{ props.riskHeading }}</dt>
        <dd>{{ props.riskNote }}</dd>
      </div>
    </dl>
    <div class="actions">
      <slot name="actions" />
    </div>
  </article>
</template>

<style scoped>
.inbox-block {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  min-width: 0;
  padding: 1rem;
  border-radius: var(--radius-l);
  border: 1px solid var(--border-subtle);
  background: var(--bg-surface);
}

.topline {
  display: flex;
  flex-wrap: wrap;
  justify-content: space-between;
  gap: 0.75rem;
}

p,
dd {
  color: var(--text-secondary);
  line-height: 1.5;
  overflow-wrap: anywhere;
}

dl {
  display: grid;
  gap: 0.65rem;
}

dt {
  font-size: 0.76rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--text-tertiary);
}

.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.55rem;
}
</style>
