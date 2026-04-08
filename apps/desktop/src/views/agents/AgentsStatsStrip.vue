<script setup lang="ts">
import { UiStatTile } from '@octopus/ui'

export interface AgentCenterStatItem {
  label: string
  value: string
  helper: string
  tone?: 'default' | 'success' | 'warning' | 'error' | 'info'
}

const props = defineProps<{
  stats: AgentCenterStatItem[]
}>()
</script>

<template>
  <section class="stats-grid" data-testid="agent-center-stats">
    <UiStatTile
      v-for="stat in props.stats"
      :key="stat.label"
      :label="stat.label"
      :value="stat.value"
      :helper="stat.helper"
      :tone="stat.tone ?? 'default'"
    />
  </section>
</template>

<style scoped>
.stats-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 1rem;
}

@media (max-width: 1100px) {
  .stats-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@media (max-width: 720px) {
  .stats-grid {
    grid-template-columns: 1fr;
  }
}
</style>
