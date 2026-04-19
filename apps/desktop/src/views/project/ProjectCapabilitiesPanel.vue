<script setup lang="ts">
import { UiButton, UiRecordCard } from '@octopus/ui'

defineProps<{
  capabilityCards: Array<{
    id: 'models' | 'tools' | 'agents' | 'teams'
    title: string
    summary: string
  }>
}>()

const emit = defineEmits<{
  editModels: []
  editTools: []
  editAgents: []
  editTeams: []
}>()

function editActionLabel(id: 'models' | 'tools' | 'agents' | 'teams') {
  switch (id) {
    case 'models':
      return 'project-settings-edit-models'
    case 'tools':
      return 'project-settings-edit-tools'
    case 'agents':
      return 'project-settings-edit-agents'
    case 'teams':
      return 'project-settings-edit-teams'
  }
}

function emitEditAction(id: 'models' | 'tools' | 'agents' | 'teams') {
  switch (id) {
    case 'models':
      emit('editModels')
      return
    case 'tools':
      emit('editTools')
      return
    case 'agents':
      emit('editAgents')
      return
    case 'teams':
      emit('editTeams')
  }
}
</script>

<template>
  <UiRecordCard
    :title="$t('projectSettings.sections.capabilities.title')"
    :description="$t('projectSettings.sections.capabilities.description')"
  >
    <div class="space-y-3">
      <div
        v-for="item in capabilityCards"
        :key="item.id"
        :data-testid="`project-settings-capability-${item.id}-card`"
        class="rounded-[var(--radius-l)] border border-border bg-surface px-4 py-4"
      >
        <div class="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
          <div class="min-w-0 space-y-1">
            <div class="text-sm font-semibold text-text-primary">
              {{ item.title }}
            </div>
            <div class="text-sm leading-6 text-text-secondary">
              {{ item.summary }}
            </div>
          </div>

          <div class="flex shrink-0 flex-wrap gap-2">
            <UiButton
              variant="outline"
              size="sm"
              :data-testid="editActionLabel(item.id)"
              @click="emitEditAction(item.id)"
            >
              {{ $t('common.edit') }}
            </UiButton>
          </div>
        </div>
      </div>
    </div>
  </UiRecordCard>
</template>
