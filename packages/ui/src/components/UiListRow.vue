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
  <article class="ui-list-row" :class="{ active: props.active, interactive: props.interactive }">
    <div class="copy">
      <p v-if="props.eyebrow" class="eyebrow">{{ props.eyebrow }}</p>
      <strong>{{ props.title }}</strong>
      <p v-if="props.subtitle" class="subtitle">{{ props.subtitle }}</p>
      <slot />
    </div>
    <div class="aside">
      <div class="meta">
        <slot name="meta" />
      </div>
      <div class="actions">
        <slot name="actions" />
      </div>
    </div>
  </article>
</template>

<style scoped>
.ui-list-row {
  display: flex;
  justify-content: space-between;
  gap: 1rem;
  min-width: 0;
  padding: 0.95rem;
  border-radius: var(--radius-l);
  border: 1px solid var(--border-subtle);
  background: color-mix(in srgb, var(--bg-subtle) 76%, transparent);
}

.ui-list-row.active {
  border-color: color-mix(in srgb, var(--brand-primary) 28%, var(--border-subtle));
  background:
    radial-gradient(circle at top right, color-mix(in srgb, var(--brand-primary) 10%, transparent), transparent 45%),
    var(--bg-surface);
}

.ui-list-row.interactive {
  transition: transform var(--duration-fast) var(--ease-apple), border-color var(--duration-fast) var(--ease-apple);
}

.ui-list-row.interactive:hover {
  transform: translateY(-1px);
  border-color: color-mix(in srgb, var(--brand-primary) 28%, var(--border-subtle));
}

.copy {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 0.35rem;
  min-width: 0;
}

.aside {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 0.65rem;
  min-width: 0;
}

.eyebrow {
  color: var(--brand-primary);
  font-size: 0.72rem;
  font-weight: 700;
  letter-spacing: 0.12em;
  text-transform: uppercase;
}

strong {
  font-size: 0.98rem;
  line-height: 1.35;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  display: -webkit-box;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
}

.subtitle {
  color: var(--text-secondary);
  line-height: 1.5;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  display: -webkit-box;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
  overflow-wrap: anywhere;
}

.meta,
.actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  min-width: 0;
  gap: 0.45rem;
}

@media (max-width: 720px) {
  .ui-list-row {
    flex-direction: column;
  }

  .aside {
    align-items: flex-start;
  }

  .meta,
  .actions {
    justify-content: flex-start;
  }
}
</style>
