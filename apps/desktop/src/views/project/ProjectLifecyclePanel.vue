<script setup lang="ts">
import { computed } from 'vue'

import type { ProjectDeletionRequest, ProjectRecord } from '@octopus/schema'

import {
  UiBadge,
  UiButton,
  UiStatusCallout,
} from '@octopus/ui'

const props = defineProps<{
  title: string
  description: string
  reviewCallout?: string
  statusLabel: string
  badgeTone: 'success' | 'warning'
  projectStatus: ProjectRecord['status']
  deletionRequestStatusLabel: string
  latestDeletionRequest: ProjectDeletionRequest | null
  deletionRequestsReady: boolean
  canManageProjectSettings: boolean
  canReviewDeletion: boolean
  creatingDeletionRequest: boolean
  reviewingDeletionRequest: 'approve' | 'reject' | null
  deletingProject: boolean
  lifecycleError: string
}>()

const emit = defineEmits<{
  archive: []
  restore: []
  requestDelete: []
  approveDeleteRequest: []
  rejectDeleteRequest: []
  deleteProject: []
}>()

const showRequestDelete = computed(() =>
  props.projectStatus === 'archived'
  && props.deletionRequestsReady
  && (!props.latestDeletionRequest || props.latestDeletionRequest.status === 'rejected'),
)
const showReviewActions = computed(() =>
  props.projectStatus === 'archived'
  && props.deletionRequestsReady
  && props.latestDeletionRequest?.status === 'pending'
  && props.canReviewDeletion,
)
const showDeleteProject = computed(() =>
  props.projectStatus === 'archived'
  && props.deletionRequestsReady
  && props.latestDeletionRequest?.status === 'approved',
)
</script>

<template>
  <section
    data-testid="project-settings-lifecycle-section"
    class="rounded-[var(--radius-xl)] border border-border bg-surface px-5 py-5"
  >
    <div class="flex flex-wrap items-start justify-between gap-3">
      <div class="space-y-1">
        <div class="text-[22px] font-bold tracking-[-0.02em] text-text-primary">
          {{ title }}
        </div>
        <div class="text-sm leading-6 text-text-secondary">
          {{ description }}
        </div>
      </div>

      <UiBadge :label="statusLabel" :tone="badgeTone" />
    </div>

    <UiStatusCallout
      v-if="reviewCallout"
      data-testid="project-settings-lifecycle-review-callout"
      class="mt-4"
      tone="warning"
      :description="reviewCallout"
    />

    <div class="mt-4 grid gap-3 md:grid-cols-[minmax(0,1fr)_auto] md:items-end">
      <div class="rounded-[var(--radius-l)] border border-border bg-surface-muted px-4 py-3">
        <div class="text-xs font-semibold uppercase tracking-[0.18em] text-text-tertiary">
          {{ $t('projects.deletionRequest.title') }}
        </div>
        <div
          data-testid="project-settings-delete-request-status"
          class="mt-1 text-sm leading-6 text-text-primary"
        >
          {{ deletionRequestStatusLabel }}
        </div>
      </div>

      <div class="flex flex-wrap gap-2">
        <UiButton
          v-if="canManageProjectSettings && projectStatus === 'active'"
          data-testid="project-settings-archive-button"
          variant="ghost"
          @click="emit('archive')"
        >
          {{ $t('projects.actions.archive') }}
        </UiButton>

        <UiButton
          v-if="canManageProjectSettings && projectStatus === 'archived'"
          data-testid="project-settings-restore-button"
          variant="ghost"
          @click="emit('restore')"
        >
          {{ $t('projects.actions.restore') }}
        </UiButton>

        <UiButton
          v-if="canManageProjectSettings && showRequestDelete"
          data-testid="project-settings-request-delete-button"
          variant="ghost"
          :disabled="creatingDeletionRequest || reviewingDeletionRequest !== null"
          @click="emit('requestDelete')"
        >
          {{ $t('projects.actions.requestDelete') }}
        </UiButton>

        <UiButton
          v-if="showReviewActions"
          data-testid="project-settings-delete-request-approve-button"
          variant="ghost"
          :disabled="reviewingDeletionRequest !== null"
          @click="emit('approveDeleteRequest')"
        >
          {{ $t('common.approve') }}
        </UiButton>

        <UiButton
          v-if="showReviewActions"
          data-testid="project-settings-delete-request-reject-button"
          variant="ghost"
          :disabled="reviewingDeletionRequest !== null"
          @click="emit('rejectDeleteRequest')"
        >
          {{ $t('common.reject') }}
        </UiButton>

        <UiButton
          v-if="canManageProjectSettings && showDeleteProject"
          data-testid="project-settings-delete-project-button"
          variant="destructive"
          :disabled="deletingProject || reviewingDeletionRequest !== null"
          @click="emit('deleteProject')"
        >
          {{ $t('common.delete') }}
        </UiButton>
      </div>
    </div>

    <UiStatusCallout
      v-if="lifecycleError"
      class="mt-4"
      tone="error"
      :description="lifecycleError"
    />
  </section>
</template>
