<script setup lang="ts">
import { cn } from '../lib/utils'

import UiEmptyState from './UiEmptyState.vue'
import UiInspectorPanel from './UiInspectorPanel.vue'
import UiListDetailShell from './UiListDetailShell.vue'

const props = withDefaults(defineProps<{
  class?: string
  toolbarClass?: string
  listClass?: string
  detailClass?: string
  detailTitle?: string
  detailSubtitle?: string
  emptyDetailTitle?: string
  emptyDetailDescription?: string
  hasSelection?: boolean
}>(), {
  class: '',
  toolbarClass: '',
  listClass: '',
  detailClass: '',
  detailTitle: '',
  detailSubtitle: '',
  emptyDetailTitle: '请选择一项',
  emptyDetailDescription: '从左侧列表中选择一项后即可查看详情。',
  hasSelection: false,
})
</script>

<template>
  <section
    data-testid="ui-list-detail-workspace"
    :class="cn('flex min-h-0 flex-col gap-4', props.class)"
  >
    <div
      v-if="$slots.toolbar"
      data-testid="ui-list-detail-workspace-toolbar"
      :class="cn('min-w-0', props.toolbarClass)"
    >
      <slot name="toolbar" />
    </div>

    <UiListDetailShell :list-class="props.listClass" :detail-class="props.detailClass">
      <template #list>
        <slot name="list" />
      </template>

      <UiInspectorPanel
        :title="props.hasSelection ? props.detailTitle : ''"
        :subtitle="props.hasSelection ? props.detailSubtitle : ''"
        data-testid="ui-list-detail-workspace-detail"
        class="min-h-[480px]"
      >
        <slot v-if="props.hasSelection" name="detail" />
        <UiEmptyState
          v-else
          :title="props.emptyDetailTitle"
          :description="props.emptyDetailDescription"
        />
      </UiInspectorPanel>
    </UiListDetailShell>
  </section>
</template>
