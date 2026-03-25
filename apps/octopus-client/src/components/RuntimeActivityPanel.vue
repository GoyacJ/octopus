<script setup lang="ts">
import { storeToRefs } from 'pinia'
import { useI18n } from 'vue-i18n'

import { useRuntimeControlStore } from '@/stores/useRuntimeControlStore'

const { t } = useI18n()
const runtimeStore = useRuntimeControlStore()
const {
  currentApproval,
  currentArtifact,
  currentInboxItem,
  currentRun,
  currentRunDetail,
  errorMessage,
} = storeToRefs(runtimeStore)
</script>

<template>
  <section class="space-y-6">
    <article class="rounded-[32px] border border-[var(--border-muted)] bg-[var(--surface-panel)] p-7 shadow-sm">
      <div class="flex items-start justify-between gap-4">
        <div>
          <p class="text-xs uppercase tracking-[0.24em] text-[var(--text-muted)]">{{ t('activity.eyebrow') }}</p>
          <h2 class="mt-3 text-2xl font-semibold leading-tight">{{ t('activity.title') }}</h2>
        </div>
        <span class="rounded-full bg-[var(--surface-elevated)] px-3 py-1 text-xs font-medium text-[var(--text-muted)]">
          {{ currentRun?.status ?? t('activity.emptyState') }}
        </span>
      </div>

      <p
        v-if="errorMessage"
        data-test="error-message"
        class="mt-4 rounded-2xl border border-red-300/50 bg-red-100/70 px-4 py-3 text-sm text-red-800"
      >
        {{ errorMessage }}
      </p>

      <dl v-if="currentRun" class="mt-6 grid gap-3 text-sm sm:grid-cols-2">
        <div class="rounded-2xl border border-[var(--border-muted)] bg-[var(--surface-elevated)] px-4 py-3">
          <dt class="text-xs uppercase tracking-[0.18em] text-[var(--text-muted)]">{{ t('activity.summary.runId') }}</dt>
          <dd class="mt-2 font-medium">{{ currentRun.id }}</dd>
        </div>
        <div class="rounded-2xl border border-[var(--border-muted)] bg-[var(--surface-elevated)] px-4 py-3">
          <dt class="text-xs uppercase tracking-[0.18em] text-[var(--text-muted)]">{{ t('activity.summary.runType') }}</dt>
          <dd class="mt-2 font-medium">{{ currentRun.run_type }}</dd>
        </div>
        <div class="rounded-2xl border border-[var(--border-muted)] bg-[var(--surface-elevated)] px-4 py-3">
          <dt class="text-xs uppercase tracking-[0.18em] text-[var(--text-muted)]">{{ t('activity.summary.approvalId') }}</dt>
          <dd class="mt-2 font-medium">{{ currentApproval?.id ?? 'N/A' }}</dd>
        </div>
        <div class="rounded-2xl border border-[var(--border-muted)] bg-[var(--surface-elevated)] px-4 py-3">
          <dt class="text-xs uppercase tracking-[0.18em] text-[var(--text-muted)]">{{ t('activity.summary.reviewedBy') }}</dt>
          <dd class="mt-2 font-medium">{{ currentApproval?.reviewed_by ?? 'N/A' }}</dd>
        </div>
        <div class="rounded-2xl border border-[var(--border-muted)] bg-[var(--surface-elevated)] px-4 py-3 sm:col-span-2">
          <dt class="text-xs uppercase tracking-[0.18em] text-[var(--text-muted)]">{{ t('activity.summary.inboxState') }}</dt>
          <dd class="mt-2 font-medium">{{ currentInboxItem?.state ?? 'N/A' }}</dd>
        </div>
      </dl>
      <p v-else class="mt-4 text-sm text-[var(--text-muted)]">{{ t('activity.emptyDescription') }}</p>
    </article>

    <section class="rounded-[28px] border border-[var(--border-muted)] bg-[var(--surface-panel)] p-6 shadow-sm">
      <h3 class="text-lg font-semibold">{{ t('activity.sections.artifact') }}</h3>
      <div v-if="currentArtifact" class="mt-4 rounded-2xl border border-[var(--border-muted)] bg-[var(--surface-elevated)] px-4 py-3 text-sm">
        <p class="font-medium">{{ currentArtifact.title }}</p>
        <p class="mt-2 text-[var(--text-muted)]">{{ currentArtifact.content_ref }}</p>
      </div>
      <p v-else class="mt-4 text-sm text-[var(--text-muted)]">{{ t('activity.pending.artifact') }}</p>
    </section>

    <section class="rounded-[28px] border border-[var(--border-muted)] bg-[var(--surface-panel)] p-6 shadow-sm">
      <h3 class="text-lg font-semibold">{{ t('activity.sections.trace') }}</h3>
      <ul class="mt-4 space-y-3 text-sm">
        <li
          v-for="event in currentRunDetail?.trace ?? []"
          :key="`${event.name}-${event.occurred_at}`"
          class="rounded-2xl border border-[var(--border-muted)] bg-[var(--surface-elevated)] px-4 py-3"
        >
          <p class="font-medium">{{ event.name }}</p>
          <p class="mt-1 text-[var(--text-muted)]">{{ event.message }}</p>
        </li>
      </ul>
      <p v-if="!(currentRunDetail?.trace.length)" class="mt-4 text-sm text-[var(--text-muted)]">{{ t('activity.pending.trace') }}</p>
    </section>

    <section class="rounded-[28px] border border-[var(--border-muted)] bg-[var(--surface-panel)] p-6 shadow-sm">
      <h3 class="text-lg font-semibold">{{ t('activity.sections.audit') }}</h3>
      <ul class="mt-4 space-y-3 text-sm">
        <li
          v-for="entry in currentRunDetail?.audit ?? []"
          :key="`${entry.action}-${entry.occurred_at}`"
          class="rounded-2xl border border-[var(--border-muted)] bg-[var(--surface-elevated)] px-4 py-3"
        >
          <p class="font-medium">{{ entry.action }}</p>
          <p class="mt-1 text-[var(--text-muted)]">{{ entry.actor }} → {{ entry.target_ref }}</p>
        </li>
      </ul>
      <p v-if="!(currentRunDetail?.audit.length)" class="mt-4 text-sm text-[var(--text-muted)]">{{ t('activity.pending.audit') }}</p>
    </section>
  </section>
</template>
