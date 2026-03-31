<script setup lang="ts">
const props = defineProps<{
  label: string
  hint?: string
}>()
</script>

<template>
  <label class="ui-field">
    <span class="label">{{ props.label }}</span>
    <slot />
    <small v-if="props.hint" class="hint">{{ props.hint }}</small>
  </label>
</template>

<style scoped>
.ui-field {
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
  min-width: 0;
}

.label {
  font-size: 0.83rem;
  font-weight: 700;
}

.hint {
  color: var(--text-tertiary);
  line-height: 1.5;
  min-width: 0;
  overflow-wrap: anywhere;
}
</style>
