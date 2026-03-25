<script setup lang="ts">
import { storeToRefs } from 'pinia'
import { computed, reactive } from 'vue'
import { useI18n } from 'vue-i18n'

import type { ApprovalDecision, TaskSubmissionRequest } from '@octopus/contracts'

import { useRuntimeControlStore } from '@/stores/useRuntimeControlStore'

const { t } = useI18n()
const runtimeStore = useRuntimeControlStore()
const {
  currentApproval,
  currentRun,
  isResolvingApproval,
  isResumingRun,
  isSubmittingTask,
} = storeToRefs(runtimeStore)

const formState = reactive({
  workspaceId: 'workspace-alpha',
  projectId: 'project-alpha',
  title: '',
  description: '',
  requestedBy: 'operator-1',
  reviewedBy: 'reviewer-1',
  requiresApproval: true,
})

const canResolveApproval = computed(
  () => currentRun.value?.status === 'waiting_approval' && currentApproval.value?.state === 'pending',
)
const canResume = computed(() => currentRun.value?.status === 'paused')

const submitTask = async () => {
  const payload: TaskSubmissionRequest = {
    workspace_id: formState.workspaceId,
    project_id: formState.projectId,
    title: formState.title,
    description: formState.description || null,
    requested_by: formState.requestedBy,
    requires_approval: formState.requiresApproval,
  }

  try {
    await runtimeStore.submitTask(payload)
  } catch {
    // The store owns error state for the surrounding shell.
  }
}

const resolveApproval = async (decision: ApprovalDecision) => {
  try {
    await runtimeStore.resolveApproval(decision, formState.reviewedBy)
  } catch {
    // The store owns error state for the surrounding shell.
  }
}

const resumeRun = async () => {
  try {
    await runtimeStore.resumeRun()
  } catch {
    // The store owns error state for the surrounding shell.
  }
}
</script>

<template>
  <article class="rounded-[32px] border border-[var(--border-muted)] bg-[var(--surface-panel)] p-7 shadow-sm">
    <div class="flex items-start justify-between gap-4">
      <div>
        <p class="text-xs uppercase tracking-[0.24em] text-[var(--text-muted)]">{{ t('task.eyebrow') }}</p>
        <h2 class="mt-3 text-2xl font-semibold leading-tight">{{ t('task.title') }}</h2>
        <p class="mt-3 max-w-2xl text-sm leading-6 text-[var(--text-muted)]">{{ t('task.subtitle') }}</p>
      </div>
      <dl class="grid gap-2 text-right text-xs uppercase tracking-[0.18em] text-[var(--text-muted)]">
        <div>
          <dt>{{ t('task.context.workspace') }}</dt>
          <dd class="mt-1 text-sm font-medium text-[var(--text-primary)]">{{ formState.workspaceId }}</dd>
        </div>
        <div>
          <dt>{{ t('task.context.project') }}</dt>
          <dd class="mt-1 text-sm font-medium text-[var(--text-primary)]">{{ formState.projectId }}</dd>
        </div>
      </dl>
    </div>

    <div class="mt-6 grid gap-4">
      <label class="grid gap-2 text-sm">
        <span class="font-medium">{{ t('task.fields.title') }}</span>
        <input
          data-test="task-title"
          v-model="formState.title"
          class="rounded-2xl border border-[var(--border-muted)] bg-[var(--surface-elevated)] px-4 py-3 outline-none transition focus:border-[var(--accent-primary)]"
          type="text"
        >
      </label>

      <label class="grid gap-2 text-sm">
        <span class="font-medium">{{ t('task.fields.description') }}</span>
        <textarea
          data-test="task-description"
          v-model="formState.description"
          class="min-h-28 rounded-2xl border border-[var(--border-muted)] bg-[var(--surface-elevated)] px-4 py-3 outline-none transition focus:border-[var(--accent-primary)]"
        ></textarea>
      </label>

      <div class="grid gap-4 sm:grid-cols-2">
        <label class="grid gap-2 text-sm">
          <span class="font-medium">{{ t('task.fields.requestedBy') }}</span>
          <input
            v-model="formState.requestedBy"
            class="rounded-2xl border border-[var(--border-muted)] bg-[var(--surface-elevated)] px-4 py-3 outline-none transition focus:border-[var(--accent-primary)]"
            type="text"
          >
        </label>
        <label class="grid gap-2 text-sm">
          <span class="font-medium">{{ t('task.fields.reviewedBy') }}</span>
          <input
            v-model="formState.reviewedBy"
            class="rounded-2xl border border-[var(--border-muted)] bg-[var(--surface-elevated)] px-4 py-3 outline-none transition focus:border-[var(--accent-primary)]"
            type="text"
          >
        </label>
      </div>

      <label class="flex items-center gap-3 rounded-2xl border border-[var(--border-muted)] bg-[var(--surface-elevated)] px-4 py-3 text-sm">
        <input v-model="formState.requiresApproval" type="checkbox">
        <span>{{ t('task.fields.requiresApproval') }}</span>
      </label>
    </div>

    <div class="mt-6 flex flex-wrap gap-3">
      <button
        data-test="task-submit"
        class="rounded-full bg-[var(--accent-primary)] px-4 py-2 text-sm font-medium text-white transition hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-60"
        :disabled="isSubmittingTask || !formState.title.trim()"
        type="button"
        @click="submitTask"
      >
        {{ isSubmittingTask ? t('task.actions.submitting') : t('task.actions.submit') }}
      </button>
      <button
        data-test="approve-run"
        class="rounded-full border border-[var(--border-muted)] px-4 py-2 text-sm transition hover:bg-[var(--surface-elevated)] disabled:cursor-not-allowed disabled:opacity-60"
        :disabled="!canResolveApproval || isResolvingApproval"
        type="button"
        @click="resolveApproval('approved')"
      >
        {{ isResolvingApproval ? t('task.actions.resolving') : t('task.actions.approve') }}
      </button>
      <button
        class="rounded-full border border-[var(--border-muted)] px-4 py-2 text-sm transition hover:bg-[var(--surface-elevated)] disabled:cursor-not-allowed disabled:opacity-60"
        :disabled="!canResolveApproval || isResolvingApproval"
        type="button"
        @click="resolveApproval('rejected')"
      >
        {{ t('task.actions.reject') }}
      </button>
      <button
        data-test="resume-run"
        class="rounded-full border border-[var(--border-muted)] px-4 py-2 text-sm transition hover:bg-[var(--surface-elevated)] disabled:cursor-not-allowed disabled:opacity-60"
        :disabled="!canResume || isResumingRun"
        type="button"
        @click="resumeRun"
      >
        {{ isResumingRun ? t('task.actions.resuming') : t('task.actions.resume') }}
      </button>
    </div>
  </article>
</template>
