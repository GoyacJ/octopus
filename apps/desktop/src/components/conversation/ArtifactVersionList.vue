<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

import type { DeliverableVersionSummary } from '@octopus/schema'

import { UiBadge, UiEmptyState, UiPanelFrame } from '@octopus/ui'

import { formatDateTime } from '@/i18n/copy'

const props = withDefaults(defineProps<{
  versions?: DeliverableVersionSummary[]
  selectedVersion?: number | null
  loading?: boolean
}>(), {
  versions: () => [],
  selectedVersion: null,
  loading: false,
})

const emit = defineEmits<{
  select: [version: number]
}>()

const { t } = useI18n()

const sortedVersions = computed(() =>
  props.versions.slice().sort((left, right) => right.version - left.version),
)

function selectVersion(version: number) {
  emit('select', version)
}
</script>

<template>
  <UiPanelFrame
    data-testid="deliverable-version-list"
    variant="subtle"
    padding="md"
    class="space-y-3"
  >
    <div class="flex items-center justify-between gap-3">
      <div class="text-[11px] font-bold uppercase tracking-widest text-text-tertiary">
        {{ t('conversation.detail.deliverables.versionsTitle') }}
      </div>
      <div class="text-xs text-text-tertiary">
        {{ t('conversation.detail.deliverables.versionCount', { count: sortedVersions.length }) }}
      </div>
    </div>

    <div
      v-if="loading && !sortedVersions.length"
      class="text-xs text-text-tertiary"
    >
      {{ t('conversation.detail.deliverables.loadingVersions') }}
    </div>

    <div v-else-if="sortedVersions.length" class="space-y-2">
      <button
        v-for="version in sortedVersions"
        :key="version.version"
        type="button"
        class="flex w-full items-start justify-between gap-3 rounded-[var(--radius-l)] border px-3 py-2 text-left transition-colors"
        :class="version.version === selectedVersion
          ? 'border-primary/40 bg-accent text-primary'
          : 'border-border bg-background text-text-primary hover:bg-subtle'"
        :data-testid="`deliverable-version-${version.version}`"
        @click="selectVersion(version.version)"
      >
        <div class="min-w-0 space-y-1">
          <div class="flex items-center gap-2">
            <span class="text-xs font-semibold">
              {{ t('conversation.detail.deliverables.versionLabel', { version: version.version }) }}
            </span>
            <UiBadge
              v-if="version.parentVersion"
              :label="t('conversation.detail.deliverables.parentVersionLabel', { version: version.parentVersion })"
              subtle
            />
          </div>
          <div class="truncate text-sm font-medium">
            {{ version.title }}
          </div>
          <div class="text-xs text-text-tertiary">
            {{ formatDateTime(version.updatedAt) }}
          </div>
        </div>

        <UiBadge
          v-if="version.version === selectedVersion"
          :label="t('common.selected')"
          subtle
        />
      </button>
    </div>

    <UiEmptyState
      v-else
      :title="t('conversation.detail.deliverables.versionsEmptyTitle')"
      :description="t('conversation.detail.deliverables.versionsEmptyDescription')"
    />
  </UiPanelFrame>
</template>
