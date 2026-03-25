<script setup lang="ts">
import { storeToRefs } from 'pinia'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

import { useRuntimeControlStore } from '@/stores/useRuntimeControlStore'

const { t } = useI18n()
const runtimeStore = useRuntimeControlStore()
const {
  currentArtifact,
  currentRun,
  isCreatingCandidate,
  isLoadingKnowledgeSpaces,
  knowledgeSpaces,
  promotingCandidateId,
} = storeToRefs(runtimeStore)

const primarySpace = computed(() => knowledgeSpaces.value?.[0] ?? null)
const canCreateCandidate = computed(
  () => Boolean(primarySpace.value && currentRun.value && currentArtifact.value),
)

const createCandidate = async () => {
  if (!primarySpace.value) {
    return
  }

  try {
    await runtimeStore.createCandidateFromRun(primarySpace.value.space.id)
  } catch {
    // Store-level error state is rendered in the activity panel.
  }
}

const promoteCandidate = async (candidateId: string) => {
  try {
    await runtimeStore.promoteCandidate(candidateId)
  } catch {
    // Store-level error state is rendered in the activity panel.
  }
}
</script>

<template>
  <article class="rounded-[32px] border border-[var(--border-muted)] bg-[var(--surface-panel)] p-7 shadow-sm">
    <div class="flex items-start justify-between gap-4">
      <div>
        <p class="text-xs uppercase tracking-[0.24em] text-[var(--text-muted)]">{{ t('knowledge.eyebrow') }}</p>
        <h2 class="mt-3 text-2xl font-semibold leading-tight">{{ t('knowledge.title') }}</h2>
        <p class="mt-3 max-w-2xl text-sm leading-6 text-[var(--text-muted)]">{{ t('knowledge.subtitle') }}</p>
      </div>
      <span class="rounded-full bg-[var(--surface-elevated)] px-3 py-1 text-xs font-medium text-[var(--text-muted)]">
        {{ primarySpace?.space.id ?? t('knowledge.empty') }}
      </span>
    </div>

    <div class="mt-6 grid gap-4 lg:grid-cols-[0.9fr_1.1fr]">
      <section class="rounded-[28px] border border-[var(--border-muted)] bg-[var(--surface-elevated)] p-5">
        <div class="flex items-center justify-between gap-3">
          <h3 class="text-lg font-semibold">{{ t('knowledge.sections.space') }}</h3>
          <span class="text-sm text-[var(--text-muted)]">
            {{ isLoadingKnowledgeSpaces ? t('knowledge.loading') : knowledgeSpaces.length }}
          </span>
        </div>
        <div v-if="primarySpace" class="mt-4 space-y-3 text-sm">
          <div class="rounded-2xl border border-[var(--border-muted)] bg-[var(--surface-panel)] px-4 py-3">
            <p class="font-medium">{{ primarySpace.space.name }}</p>
            <p class="mt-1 text-[var(--text-muted)]">{{ primarySpace.space.scope }}</p>
          </div>
          <p class="text-[var(--text-muted)]">
            {{ t('knowledge.summary.run') }}: {{ currentRun?.id ?? 'N/A' }}
          </p>
          <p class="text-[var(--text-muted)]">
            {{ t('knowledge.summary.artifact') }}: {{ currentArtifact?.title ?? 'N/A' }}
          </p>
          <button
            data-test="knowledge-create-candidate"
            class="rounded-full bg-[var(--accent-primary)] px-4 py-2 text-sm font-medium text-white transition hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-60"
            :disabled="!canCreateCandidate || isCreatingCandidate"
            type="button"
            @click="createCandidate"
          >
            {{ isCreatingCandidate ? t('knowledge.actions.creating') : t('knowledge.actions.createCandidate') }}
          </button>
        </div>
        <p v-else class="mt-4 text-sm text-[var(--text-muted)]">{{ t('knowledge.emptyDescription') }}</p>
      </section>

      <section class="grid gap-4">
        <div class="rounded-[28px] border border-[var(--border-muted)] bg-[var(--surface-elevated)] p-5">
          <h3 class="text-lg font-semibold">{{ t('knowledge.sections.candidates') }}</h3>
          <ul v-if="primarySpace?.candidates.length" class="mt-4 space-y-3 text-sm">
            <li
              v-for="candidate in primarySpace.candidates"
              :key="candidate.id"
              class="rounded-2xl border border-[var(--border-muted)] bg-[var(--surface-panel)] px-4 py-3"
            >
              <div class="flex items-start justify-between gap-3">
                <div>
                  <p class="font-medium">{{ candidate.title }}</p>
                  <p class="mt-1 text-[var(--text-muted)]">{{ candidate.status }} · {{ candidate.trust_level }}</p>
                </div>
                <button
                  v-if="candidate.status === 'candidate'"
                  :data-test="`knowledge-promote-${candidate.id}`"
                  class="rounded-full border border-[var(--border-muted)] px-3 py-1 text-xs transition hover:bg-[var(--surface-elevated)] disabled:cursor-not-allowed disabled:opacity-60"
                  :disabled="promotingCandidateId === candidate.id"
                  type="button"
                  @click="promoteCandidate(candidate.id)"
                >
                  {{ promotingCandidateId === candidate.id ? t('knowledge.actions.promoting') : t('knowledge.actions.promote') }}
                </button>
              </div>
            </li>
          </ul>
          <p v-else class="mt-4 text-sm text-[var(--text-muted)]">{{ t('knowledge.pending.candidates') }}</p>
        </div>

        <div class="rounded-[28px] border border-[var(--border-muted)] bg-[var(--surface-elevated)] p-5">
          <h3 class="text-lg font-semibold">{{ t('knowledge.sections.assets') }}</h3>
          <ul v-if="primarySpace?.assets.length" class="mt-4 space-y-3 text-sm">
            <li
              v-for="asset in primarySpace.assets"
              :key="asset.id"
              class="rounded-2xl border border-[var(--border-muted)] bg-[var(--surface-panel)] px-4 py-3"
            >
              <p class="font-medium">{{ asset.title }}</p>
              <p class="mt-1 text-[var(--text-muted)]">{{ asset.status }} · {{ asset.id }}</p>
            </li>
          </ul>
          <p v-else class="mt-4 text-sm text-[var(--text-muted)]">{{ t('knowledge.pending.assets') }}</p>
        </div>
      </section>
    </div>
  </article>
</template>
