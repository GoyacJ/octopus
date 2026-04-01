<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'

import { UiEmptyState, UiSectionHeading, UiSurface } from '@octopus/ui'

import { useWorkbenchStore } from '@/stores/workbench'

const { t } = useI18n()
const workbench = useWorkbenchStore()
const activeTab = ref<'builtin' | 'skill' | 'mcp'>('builtin')

const activeGroup = computed(() =>
  workbench.toolCatalogGroups.find((group) => group.id === activeTab.value),
)
</script>

<template>
  <section class="section-stack">
    <UiSectionHeading
      :eyebrow="t('tools.header.eyebrow')"
      :title="t('tools.header.title')"
      :subtitle="t('tools.header.subtitle')"
    />

    <UiSurface :title="t('tools.tabs.title')" :subtitle="t('tools.tabs.subtitle')">
      <div class="tool-tabs">
        <button type="button" class="tool-tab" :class="{ active: activeTab === 'builtin' }" @click="activeTab = 'builtin'">
          {{ t('tools.tabs.builtin') }}
        </button>
        <button type="button" class="tool-tab" :class="{ active: activeTab === 'skill' }" @click="activeTab = 'skill'">
          {{ t('tools.tabs.skill') }}
        </button>
        <button type="button" class="tool-tab" :class="{ active: activeTab === 'mcp' }" @click="activeTab = 'mcp'">
          {{ t('tools.tabs.mcp') }}
        </button>
      </div>

      <div v-if="activeGroup?.items.length" class="tool-list">
        <div v-for="item in activeGroup.items" :key="item.id" class="tool-row">
          <div class="tool-copy">
            <strong>{{ item.name }}</strong>
            <small>{{ item.description }}</small>
          </div>
          <div class="tool-meta">
            <span>{{ item.availability }}</span>
            <span>{{ item.permissions.join(' / ') }}</span>
          </div>
        </div>
      </div>
      <UiEmptyState v-else :title="t('tools.tabs.emptyTitle')" :description="t('tools.tabs.emptyDescription')" />
    </UiSurface>
  </section>
</template>

<style scoped>
.tool-tabs,
.tool-list {
  display: flex;
}

.tool-tabs {
  gap: 0.5rem;
  margin-bottom: 1rem;
  flex-wrap: wrap;
}

.tool-tab {
  padding: 0.55rem 0.85rem;
  border-radius: 999px;
  border: 1px solid color-mix(in srgb, var(--border-subtle) 88%, transparent);
  background: color-mix(in srgb, var(--bg-subtle) 72%, transparent);
  color: var(--text-secondary);
}

.tool-tab.active {
  border-color: color-mix(in srgb, var(--brand-primary) 38%, var(--border-subtle));
  background: color-mix(in srgb, var(--brand-primary) 8%, var(--bg-surface));
  color: var(--text-primary);
}

.tool-list {
  flex-direction: column;
  gap: 0.75rem;
}

.tool-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.9rem;
  padding: 0.9rem 0.95rem;
  border-radius: 1rem;
  border: 1px solid color-mix(in srgb, var(--border-subtle) 88%, transparent);
  background: color-mix(in srgb, var(--bg-subtle) 72%, transparent);
}

.tool-copy,
.tool-meta {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.tool-copy small,
.tool-meta {
  color: var(--text-muted);
}

.tool-meta {
  align-items: flex-end;
  font-size: 0.78rem;
}
</style>
