<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import { useI18n } from 'vue-i18n'

import type { ApprovalDecision, RunDetailResponse, TaskSubmissionRequest } from '@octopus/contracts'

import { runtimeApi, RuntimeApiError } from '@/services/runtimeApi'

const { t } = useI18n()

const formState = reactive({
  projectId: 'project-alpha',
  title: '',
  description: '',
  requestedBy: 'operator-1',
  reviewedBy: 'reviewer-1',
  requiresApproval: true,
})

const runDetail = ref<RunDetailResponse | null>(null)
const errorMessage = ref('')
const isSubmitting = ref(false)
const isResolvingApproval = ref(false)
const isResuming = ref(false)

const currentRun = computed(() => runDetail.value?.run ?? null)
const currentApproval = computed(() => runDetail.value?.approval ?? null)
const currentArtifact = computed(() => runDetail.value?.artifact ?? null)
const currentInboxItem = computed(() => runDetail.value?.inbox_item ?? null)

const canResolveApproval = computed(
  () => currentRun.value?.status === 'waiting_approval' && currentApproval.value?.state === 'pending',
)
const canResume = computed(() => currentRun.value?.status === 'paused')

const clearError = () => {
  errorMessage.value = ''
}

const updateTitle = (event: Event) => {
  formState.title = (event.target as HTMLInputElement | null)?.value ?? ''
}

const updateDescription = (event: Event) => {
  formState.description = (event.target as HTMLTextAreaElement | null)?.value ?? ''
}

const updateRequestedBy = (event: Event) => {
  formState.requestedBy = (event.target as HTMLInputElement | null)?.value ?? ''
}

const updateReviewedBy = (event: Event) => {
  formState.reviewedBy = (event.target as HTMLInputElement | null)?.value ?? ''
}

const handleError = (error: unknown) => {
  if (error instanceof RuntimeApiError) {
    errorMessage.value = error.message
    return
  }

  errorMessage.value = t('demo.genericError')
}

const submitTask = async () => {
  clearError()
  isSubmitting.value = true

  const payload: TaskSubmissionRequest = {
    project_id: formState.projectId,
    title: formState.title,
    description: formState.description || null,
    requested_by: formState.requestedBy,
    requires_approval: formState.requiresApproval,
  }

  try {
    runDetail.value = await runtimeApi.submitTask(payload)
  } catch (error) {
    handleError(error)
  } finally {
    isSubmitting.value = false
  }
}

const resolveApproval = async (decision: ApprovalDecision) => {
  if (!currentApproval.value) {
    return
  }

  clearError()
  isResolvingApproval.value = true

  try {
    runDetail.value = await runtimeApi.resolveApproval(currentApproval.value.id, {
      decision,
      reviewed_by: formState.reviewedBy,
    })
  } catch (error) {
    handleError(error)
  } finally {
    isResolvingApproval.value = false
  }
}

const resumeRun = async () => {
  if (!currentRun.value) {
    return
  }

  clearError()
  isResuming.value = true

  try {
    runDetail.value = await runtimeApi.resumeRun(currentRun.value.id)
  } catch (error) {
    handleError(error)
  } finally {
    isResuming.value = false
  }
}
</script>

<template>
  <section class="grid gap-6 lg:grid-cols-[0.95fr_1.05fr]">
    <article class="rounded-[32px] border border-[var(--border-muted)] bg-[var(--surface-panel)] p-7 shadow-sm">
      <div class="flex items-start justify-between gap-4">
        <div>
          <p class="text-xs uppercase tracking-[0.24em] text-[var(--text-muted)]">{{ t('demo.eyebrow') }}</p>
          <h2 class="mt-3 text-2xl font-semibold leading-tight">{{ t('demo.title') }}</h2>
          <p class="mt-3 max-w-2xl text-sm leading-6 text-[var(--text-muted)]">{{ t('demo.subtitle') }}</p>
        </div>
        <span class="rounded-full bg-[var(--surface-elevated)] px-3 py-1 text-xs font-medium text-[var(--text-muted)]">
          {{ t('demo.mode') }}
        </span>
      </div>

      <div class="mt-6 grid gap-4">
        <label class="grid gap-2 text-sm">
          <span class="font-medium">{{ t('demo.fields.title') }}</span>
          <input
            data-test="task-title"
            :value="formState.title"
            class="rounded-2xl border border-[var(--border-muted)] bg-[var(--surface-elevated)] px-4 py-3 outline-none transition focus:border-[var(--accent-primary)]"
            type="text"
            @input="updateTitle"
          >
        </label>

        <label class="grid gap-2 text-sm">
          <span class="font-medium">{{ t('demo.fields.description') }}</span>
          <textarea
            data-test="task-description"
            :value="formState.description"
            class="min-h-28 rounded-2xl border border-[var(--border-muted)] bg-[var(--surface-elevated)] px-4 py-3 outline-none transition focus:border-[var(--accent-primary)]"
            @input="updateDescription"
          ></textarea>
        </label>

        <div class="grid gap-4 sm:grid-cols-2">
          <label class="grid gap-2 text-sm">
            <span class="font-medium">{{ t('demo.fields.requestedBy') }}</span>
            <input
              :value="formState.requestedBy"
              class="rounded-2xl border border-[var(--border-muted)] bg-[var(--surface-elevated)] px-4 py-3 outline-none transition focus:border-[var(--accent-primary)]"
              type="text"
              @input="updateRequestedBy"
            >
          </label>
          <label class="grid gap-2 text-sm">
            <span class="font-medium">{{ t('demo.fields.reviewedBy') }}</span>
            <input
              :value="formState.reviewedBy"
              class="rounded-2xl border border-[var(--border-muted)] bg-[var(--surface-elevated)] px-4 py-3 outline-none transition focus:border-[var(--accent-primary)]"
              type="text"
              @input="updateReviewedBy"
            >
          </label>
        </div>

        <label class="flex items-center gap-3 rounded-2xl border border-[var(--border-muted)] bg-[var(--surface-elevated)] px-4 py-3 text-sm">
          <input v-model="formState.requiresApproval" type="checkbox">
          <span>{{ t('demo.fields.requiresApproval') }}</span>
        </label>
      </div>

      <div class="mt-6 flex flex-wrap gap-3">
        <button
          data-test="task-submit"
          class="rounded-full bg-[var(--accent-primary)] px-4 py-2 text-sm font-medium text-white transition hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-60"
          :disabled="isSubmitting || !formState.title.trim()"
          type="button"
          @click="submitTask"
        >
          {{ isSubmitting ? t('demo.actions.submitting') : t('demo.actions.submit') }}
        </button>
        <button
          data-test="approve-run"
          class="rounded-full border border-[var(--border-muted)] px-4 py-2 text-sm transition hover:bg-[var(--surface-elevated)] disabled:cursor-not-allowed disabled:opacity-60"
          :disabled="!canResolveApproval || isResolvingApproval"
          type="button"
          @click="resolveApproval('approved')"
        >
          {{ isResolvingApproval ? t('demo.actions.resolving') : t('demo.actions.approve') }}
        </button>
        <button
          class="rounded-full border border-[var(--border-muted)] px-4 py-2 text-sm transition hover:bg-[var(--surface-elevated)] disabled:cursor-not-allowed disabled:opacity-60"
          :disabled="!canResolveApproval || isResolvingApproval"
          type="button"
          @click="resolveApproval('rejected')"
        >
          {{ t('demo.actions.reject') }}
        </button>
        <button
          data-test="resume-run"
          class="rounded-full border border-[var(--border-muted)] px-4 py-2 text-sm transition hover:bg-[var(--surface-elevated)] disabled:cursor-not-allowed disabled:opacity-60"
          :disabled="!canResume || isResuming"
          type="button"
          @click="resumeRun"
        >
          {{ isResuming ? t('demo.actions.resuming') : t('demo.actions.resume') }}
        </button>
      </div>

      <p v-if="errorMessage" data-test="error-message" class="mt-4 rounded-2xl border border-red-300/50 bg-red-100/70 px-4 py-3 text-sm text-red-800">
        {{ errorMessage }}
      </p>
    </article>

    <aside class="space-y-6">
      <section class="rounded-[28px] border border-[var(--border-muted)] bg-[var(--surface-panel)] p-6 shadow-sm">
        <div class="flex items-center justify-between">
          <h3 class="text-lg font-semibold">{{ t('demo.sections.run') }}</h3>
          <span
            class="rounded-full bg-[var(--surface-elevated)] px-3 py-1 text-xs font-medium text-[var(--text-muted)]"
          >
            {{ currentRun?.status ?? t('demo.emptyState') }}
          </span>
        </div>

        <dl v-if="currentRun" class="mt-4 grid gap-3 text-sm">
          <div class="rounded-2xl border border-[var(--border-muted)] bg-[var(--surface-elevated)] px-4 py-3">
            <dt class="text-xs uppercase tracking-[0.18em] text-[var(--text-muted)]">{{ t('demo.summary.runId') }}</dt>
            <dd class="mt-2 font-medium">{{ currentRun.id }}</dd>
          </div>
          <div class="rounded-2xl border border-[var(--border-muted)] bg-[var(--surface-elevated)] px-4 py-3">
            <dt class="text-xs uppercase tracking-[0.18em] text-[var(--text-muted)]">{{ t('demo.summary.approvalId') }}</dt>
            <dd class="mt-2 font-medium">{{ currentApproval?.id ?? t('demo.na') }}</dd>
          </div>
          <div class="rounded-2xl border border-[var(--border-muted)] bg-[var(--surface-elevated)] px-4 py-3">
            <dt class="text-xs uppercase tracking-[0.18em] text-[var(--text-muted)]">{{ t('demo.summary.reviewedBy') }}</dt>
            <dd class="mt-2 font-medium">{{ currentApproval?.reviewed_by ?? t('demo.na') }}</dd>
          </div>
          <div class="rounded-2xl border border-[var(--border-muted)] bg-[var(--surface-elevated)] px-4 py-3">
            <dt class="text-xs uppercase tracking-[0.18em] text-[var(--text-muted)]">{{ t('demo.summary.inboxState') }}</dt>
            <dd class="mt-2 font-medium">{{ currentInboxItem?.state ?? t('demo.na') }}</dd>
          </div>
        </dl>
        <p v-else class="mt-4 text-sm text-[var(--text-muted)]">{{ t('demo.emptyDescription') }}</p>
      </section>

      <section class="rounded-[28px] border border-[var(--border-muted)] bg-[var(--surface-panel)] p-6 shadow-sm">
        <h3 class="text-lg font-semibold">{{ t('demo.sections.artifact') }}</h3>
        <div v-if="currentArtifact" class="mt-4 rounded-2xl border border-[var(--border-muted)] bg-[var(--surface-elevated)] px-4 py-3 text-sm">
          <p class="font-medium">{{ currentArtifact.title }}</p>
          <p class="mt-2 text-[var(--text-muted)]">{{ currentArtifact.content_ref }}</p>
        </div>
        <p v-else class="mt-4 text-sm text-[var(--text-muted)]">{{ t('demo.artifactPending') }}</p>
      </section>

      <section class="rounded-[28px] border border-[var(--border-muted)] bg-[var(--surface-panel)] p-6 shadow-sm">
        <h3 class="text-lg font-semibold">{{ t('demo.sections.trace') }}</h3>
        <ul class="mt-4 space-y-3 text-sm">
          <li
            v-for="event in runDetail?.trace ?? []"
            :key="`${event.name}-${event.occurred_at}`"
            class="rounded-2xl border border-[var(--border-muted)] bg-[var(--surface-elevated)] px-4 py-3"
          >
            <p class="font-medium">{{ event.name }}</p>
            <p class="mt-1 text-[var(--text-muted)]">{{ event.message }}</p>
          </li>
        </ul>
        <p v-if="!(runDetail?.trace.length)" class="mt-4 text-sm text-[var(--text-muted)]">{{ t('demo.tracePending') }}</p>
      </section>

      <section class="rounded-[28px] border border-[var(--border-muted)] bg-[var(--surface-panel)] p-6 shadow-sm">
        <h3 class="text-lg font-semibold">{{ t('demo.sections.audit') }}</h3>
        <ul class="mt-4 space-y-3 text-sm">
          <li
            v-for="entry in runDetail?.audit ?? []"
            :key="`${entry.action}-${entry.occurred_at}`"
            class="rounded-2xl border border-[var(--border-muted)] bg-[var(--surface-elevated)] px-4 py-3"
          >
            <p class="font-medium">{{ entry.action }}</p>
            <p class="mt-1 text-[var(--text-muted)]">{{ entry.actor }} → {{ entry.target_ref }}</p>
          </li>
        </ul>
        <p v-if="!(runDetail?.audit.length)" class="mt-4 text-sm text-[var(--text-muted)]">{{ t('demo.auditPending') }}</p>
      </section>
    </aside>
  </section>
</template>
