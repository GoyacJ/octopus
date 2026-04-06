<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { Brain, FileText, FolderTree, Sparkles, Waypoints, Wrench } from 'lucide-vue-next'

import { UiButton, UiEmptyState, UiTimelineList } from '@octopus/ui'

import type { ConversationDetailFocus } from '@octopus/schema'

import { formatDateTime } from '@/i18n/copy'
import { useRuntimeStore } from '@/stores/runtime'
import { useShellStore } from '@/stores/shell'

const { t } = useI18n()
const shell = useShellStore()
const runtime = useRuntimeStore()

const sectionItems = computed(() => [
  { id: 'summary', label: t('conversation.detail.sections.summary'), icon: Sparkles },
  { id: 'memories', label: t('conversation.detail.sections.memories'), icon: Brain },
  { id: 'artifacts', label: t('conversation.detail.sections.artifacts'), icon: FileText },
  { id: 'resources', label: t('conversation.detail.sections.resources'), icon: FolderTree },
  { id: 'tools', label: t('conversation.detail.sections.tools'), icon: Wrench },
  { id: 'timeline', label: t('conversation.detail.sections.timeline'), icon: Waypoints },
] as const)

const timelineItems = computed(() =>
  runtime.activeTrace.map(trace => ({
    id: trace.id,
    title: trace.title,
    description: trace.detail,
    helper: trace.actor,
    timestamp: formatDateTime(trace.timestamp),
  })),
)

function setDetail(detail: string) {
  shell.setDetailFocus(detail as ConversationDetailFocus)
  shell.setRightSidebarCollapsed(false)
}
</script>

<template>
  <aside
    class="h-full border-l border-border-subtle bg-sidebar dark:border-white/[0.05]"
    :class="shell.rightSidebarCollapsed ? 'w-[48px]' : 'w-[360px]'"
  >
    <div v-if="shell.rightSidebarCollapsed" class="flex h-full flex-col items-center gap-3 py-6">
      <UiButton
        v-for="item in sectionItems"
        :key="item.id"
        variant="ghost"
        size="icon"
        class="h-9 w-9 rounded-lg"
        :title="item.label"
        @click="setDetail(item.id)"
      >
        <component :is="item.icon" :size="18" />
      </UiButton>
    </div>

    <div v-else class="flex h-full flex-col overflow-hidden">
      <div class="border-b border-border-subtle px-4 py-3 dark:border-white/[0.05]">
        <div class="text-[11px] font-bold uppercase tracking-widest text-text-tertiary">{{ t('conversation.detail.title') }}</div>
      </div>

      <nav class="flex flex-wrap gap-1 border-b border-border-subtle p-2 dark:border-white/[0.05]">
        <UiButton
          v-for="item in sectionItems"
          :key="item.id"
          variant="ghost"
          size="sm"
          class="h-auto rounded px-2.5 py-1.5 text-[11px]"
          :class="shell.detailFocus === item.id ? 'bg-background text-text-primary shadow-xs' : 'text-text-tertiary hover:bg-accent hover:text-text-secondary'"
          @click="setDetail(item.id)"
        >
          {{ item.label }}
        </UiButton>
      </nav>

      <div class="flex-1 overflow-y-auto p-4">
        <div v-if="shell.detailFocus === 'summary'" class="space-y-4">
          <div class="rounded-xl border border-border-subtle p-4 dark:border-white/[0.05]">
            <div class="text-xs text-text-secondary">{{ t('conversation.detail.summary.title') }}</div>
            <div class="mt-2 text-sm text-text-primary">{{ runtime.activeSession?.summary.title ?? t('common.na') }}</div>
          </div>
          <div class="rounded-xl border border-border-subtle p-4 dark:border-white/[0.05]">
            <div class="text-xs text-text-secondary">{{ t('trace.stats.status') }}</div>
            <div class="mt-2 text-sm text-text-primary">{{ runtime.activeRunStatusLabel }}</div>
          </div>
        </div>

        <div v-else-if="shell.detailFocus === 'timeline'" class="space-y-4">
          <UiTimelineList v-if="timelineItems.length" :items="timelineItems" />
          <UiEmptyState v-else :title="t('conversation.detail.timeline.emptyTitle')" :description="t('conversation.detail.timeline.emptyDescription')" />
        </div>

        <UiEmptyState
          v-else
          :title="t('conversation.detail.emptyTitle')"
          :description="t('conversation.detail.emptyDescription')"
        />
      </div>
    </div>
  </aside>
</template>
