<script setup lang="ts">
import { computed } from 'vue'

import { UiButton, UiCheckbox, UiEmptyState, UiRecordCard, UiStatusCallout } from '@octopus/ui'

const props = defineProps<{
  workspaceUsers: Array<{
    id: string
    displayName?: string | null
    username: string
  }>
  selectedMemberUserIds: string[]
  usersError: string
  savingUsers: boolean
}>()

const emit = defineEmits<{
  reset: []
  save: []
  'update:selected-member-user-ids': [value: string[]]
}>()

const selectedMemberUserIdsModel = computed({
  get: () => props.selectedMemberUserIds,
  set: value => emit('update:selected-member-user-ids', value),
})
</script>

<template>
  <UiRecordCard
    :title="$t('projectSettings.users.title')"
    :description="$t('projectSettings.users.description')"
  >
    <template #eyebrow>
      {{ $t('projectSettings.tabs.users') }}
    </template>

    <UiEmptyState
      v-if="!workspaceUsers.length"
      :title="$t('projectSettings.users.emptyTitle')"
      :description="$t('projectSettings.users.emptyDescription')"
    />

    <div v-else class="space-y-3">
      <label
        v-for="user in workspaceUsers"
        :key="user.id"
        :data-testid="`project-member-option-${user.id}`"
        class="flex items-start justify-between gap-4 rounded-[var(--radius-l)] border border-border bg-surface px-4 py-3"
      >
        <div class="min-w-0 space-y-1">
          <div class="text-sm font-semibold text-text-primary">
            {{ user.displayName || user.username }}
          </div>
          <div class="text-xs text-text-secondary">
            @{{ user.username }}
          </div>
        </div>
        <UiCheckbox
          v-model="selectedMemberUserIdsModel"
          :value="user.id"
          :aria-label="user.displayName || user.username"
        />
      </label>

      <UiStatusCallout v-if="usersError" tone="error" :description="usersError" />
    </div>

    <template #actions>
      <UiButton variant="ghost" :disabled="savingUsers" @click="emit('reset')">
        {{ $t('common.reset') }}
      </UiButton>
      <UiButton :disabled="savingUsers || !workspaceUsers.length" @click="emit('save')">
        {{ $t('common.save') }}
      </UiButton>
    </template>
  </UiRecordCard>
</template>
