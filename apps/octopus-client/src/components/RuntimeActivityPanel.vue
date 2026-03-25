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
  inboxItems,
  lastEventSequence,
  runs,
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
        <div class="flex flex-wrap justify-end gap-2">
          <span class="rounded-full bg-[var(--surface-elevated)] px-3 py-1 text-xs font-medium text-[var(--text-muted)]">
            {{ currentRun?.status ?? t('activity.emptyState') }}
          </span>
          <span class="rounded-full border border-[var(--border-muted)] px-3 py-1 text-xs font-medium text-[var(--text-muted)]">
            {{
              lastEventSequence === null
                ? t('activity.stream.idle')
                : t('activity.stream.live', { sequence: lastEventSequence })
            }}
          </span>
        </div>
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

    <section class="grid gap-6 xl:grid-cols-2">
      <article class="rounded-[28px] border border-[var(--border-muted)] bg-[var(--surface-panel)] p-6 shadow-sm">
        <div class="flex items-center justify-between gap-3">
          <h3 class="text-lg font-semibold">{{ t('activity.sections.runs') }}</h3>
          <span class="text-xs uppercase tracking-[0.18em] text-[var(--text-muted)]">{{ runs.length }}</span>
        </div>
        <ul v-if="runs.length" class="mt-4 space-y-3">
          <li v-for="run in runs" :key="run.id">
            <button
              class="w-full rounded-2xl border px-4 py-3 text-left transition"
              :class="
                run.id === currentRun?.id
                  ? 'border-[var(--accent-primary)] bg-[var(--accent-soft)]'
                  : 'border-[var(--border-muted)] bg-[var(--surface-elevated)] hover:border-[var(--accent-primary)]/50'
              "
              type="button"
              @click="runtimeStore.selectRun(run.id)"
            >
              <div class="flex items-start justify-between gap-3">
                <div>
                  <p class="font-medium">{{ run.title }}</p>
                  <p class="mt-1 text-sm text-[var(--text-muted)]">{{ run.project_id }} · {{ run.run_type }}</p>
                </div>
                <span class="text-xs font-medium text-[var(--text-muted)]">{{ run.status }}</span>
              </div>
            </button>
          </li>
        </ul>
        <p v-else class="mt-4 text-sm text-[var(--text-muted)]">{{ t('activity.pending.runs') }}</p>
      </article>

      <article class="rounded-[28px] border border-[var(--border-muted)] bg-[var(--surface-panel)] p-6 shadow-sm">
        <div class="flex items-center justify-between gap-3">
          <h3 class="text-lg font-semibold">{{ t('activity.sections.inbox') }}</h3>
          <span class="text-xs uppercase tracking-[0.18em] text-[var(--text-muted)]">{{ inboxItems.length }}</span>
        </div>
        <ul v-if="inboxItems.length" class="mt-4 space-y-3 text-sm">
          <li
            v-for="item in inboxItems"
            :key="item.id"
            class="rounded-2xl border border-[var(--border-muted)] bg-[var(--surface-elevated)] px-4 py-3"
          >
            <div class="flex items-start justify-between gap-3">
              <div>
                <p class="font-medium">{{ item.owner_ref }}</p>
                <p class="mt-1 text-[var(--text-muted)]">{{ item.target_ref }} · {{ item.priority }}</p>
              </div>
              <span class="text-xs font-medium text-[var(--text-muted)]">{{ item.state }}</span>
            </div>
          </li>
        </ul>
        <p v-else class="mt-4 text-sm text-[var(--text-muted)]">{{ t('activity.pending.inbox') }}</p>
      </article>
    </section>

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
