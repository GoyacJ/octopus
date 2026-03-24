<script setup lang="ts">
import { computed, ref } from 'vue'
import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query'
import { useI18n } from 'vue-i18n'
import type { CreateRunInput, RunRecord } from '@octopus/api-client'
import { UiStatusBadge, UiSurfaceCard } from '@octopus/ui'
import { useControlPlaneClient } from '@/lib/control-plane'

const { t } = useI18n()
const client = useControlPlaneClient()
const queryClient = useQueryClient()

const draftInput = ref('')
const draftInteractionType = ref<CreateRunInput['interactionType']>('ask_user')
const selectedRunId = ref<string | null>(null)

const runsQuery = useQuery({
  queryKey: ['runs'],
  queryFn: () => client.listRuns(),
})

const runs = computed(() => runsQuery.data.value ?? [])
const selectedRun = computed<RunRecord | undefined>(
  () => runs.value.find((run) => run.id === selectedRunId.value) ?? runs.value[0],
)
const selectedTimelineRunId = computed(() => selectedRun.value?.id ?? '')

const timelineQuery = useQuery({
  queryKey: computed(() => ['run-timeline', selectedTimelineRunId.value]),
  queryFn: () => client.getRunTimeline(selectedTimelineRunId.value),
  enabled: computed(() => Boolean(selectedTimelineRunId.value)),
})

const createRunMutation = useMutation({
  mutationFn: (input: CreateRunInput) => client.createRun(input),
  onSuccess: async (run) => {
    draftInput.value = ''
    selectedRunId.value = run.id
    await queryClient.invalidateQueries({ queryKey: ['runs'] })
    await queryClient.invalidateQueries({ queryKey: ['run-timeline'] })
  },
})

const interactionOptions = computed(() => [
  { value: 'ask_user' as const, label: t('interaction.ask_user') },
  { value: 'approval' as const, label: t('interaction.approval') },
])

const timelineEvents = computed(() => timelineQuery.data.value ?? [])

function handleCreateRun() {
  const input = draftInput.value.trim()

  if (!input) {
    return
  }

  createRunMutation.mutate({
    workspaceId: 'workspace-1',
    agentId: 'agent-1',
    input,
    interactionType: draftInteractionType.value,
  })
}

function selectRun(runId: string) {
  selectedRunId.value = runId
}

function statusTone(status: RunRecord['status']) {
  switch (status) {
    case 'completed':
      return 'success'
    case 'waiting_input':
    case 'waiting_approval':
    case 'resuming':
      return 'warning'
    default:
      return 'accent'
  }
}
</script>

<template>
  <div class="grid gap-4">
    <UiSurfaceCard
      :eyebrow="t('pages.runsEyebrow')"
      :title="t('pages.runsTitle')"
      :body="t('pages.runsBody')"
    >
      <UiStatusBadge
        :label="t('status.ready')"
        tone="accent"
      />
    </UiSurfaceCard>

    <div class="grid gap-4 xl:grid-cols-[minmax(0,1.2fr),minmax(0,0.8fr)]">
      <section class="rounded-[24px] border border-[color:var(--oc-border-subtle)] bg-[color:var(--oc-bg-panel)] p-5">
        <div class="flex items-start justify-between gap-3">
          <div class="space-y-2">
            <p class="text-[11px] uppercase tracking-[0.22em] text-[color:var(--oc-text-muted)]">
              {{ t('pages.runsCreateEyebrow') }}
            </p>
            <h2 class="font-[var(--oc-font-display)] text-2xl tracking-tight">
              {{ t('pages.runsCreateTitle') }}
            </h2>
            <p class="text-sm leading-7 text-[color:var(--oc-text-secondary)]">
              {{ t('pages.runsCreateBody') }}
            </p>
          </div>

          <UiStatusBadge
            :label="t('pages.runsDefaultAgentLabel')"
            tone="accent"
          />
        </div>

        <form
          data-testid="create-run-form"
          class="mt-5 grid gap-4"
          @submit.prevent="handleCreateRun"
        >
          <label class="grid gap-2">
            <span class="text-sm font-medium text-[color:var(--oc-text-primary)]">
              {{ t('pages.runsInputLabel') }}
            </span>
            <textarea
              v-model="draftInput"
              data-testid="run-input"
              rows="4"
              class="min-h-32 rounded-[18px] border border-[color:var(--oc-border-subtle)] bg-[color:var(--oc-bg-surface)] px-4 py-3 text-sm text-[color:var(--oc-text-primary)] outline-none transition focus:border-[color:var(--oc-accent)]"
              :placeholder="t('pages.runsInputPlaceholder')"
            />
          </label>

          <label class="grid gap-2">
            <span class="text-sm font-medium text-[color:var(--oc-text-primary)]">
              {{ t('pages.runsInteractionTypeLabel') }}
            </span>
            <select
              v-model="draftInteractionType"
              data-testid="interaction-type"
              class="rounded-[18px] border border-[color:var(--oc-border-subtle)] bg-[color:var(--oc-bg-surface)] px-4 py-3 text-sm text-[color:var(--oc-text-primary)] outline-none transition focus:border-[color:var(--oc-accent)]"
            >
              <option
                v-for="option in interactionOptions"
                :key="option.value"
                :value="option.value"
              >
                {{ option.label }}
              </option>
            </select>
          </label>

          <button
            type="submit"
            class="inline-flex items-center justify-center rounded-[18px] bg-[color:var(--oc-accent)] px-4 py-3 text-sm font-semibold text-white transition hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-50"
            :disabled="createRunMutation.isPending.value || !draftInput.trim()"
          >
            {{ createRunMutation.isPending.value ? t('common.submitting') : t('pages.runsSubmit') }}
          </button>
        </form>
      </section>

      <section class="rounded-[24px] border border-[color:var(--oc-border-subtle)] bg-[color:var(--oc-bg-panel)] p-5">
        <div class="space-y-2">
          <p class="text-[11px] uppercase tracking-[0.22em] text-[color:var(--oc-text-muted)]">
            {{ t('pages.runsListEyebrow') }}
          </p>
          <h2 class="font-[var(--oc-font-display)] text-2xl tracking-tight">
            {{ t('pages.runsListTitle') }}
          </h2>
          <p class="text-sm leading-7 text-[color:var(--oc-text-secondary)]">
            {{ t('pages.runsListBody') }}
          </p>
        </div>

        <div class="mt-5 grid gap-3">
          <p
            v-if="runsQuery.isLoading.value"
            class="text-sm text-[color:var(--oc-text-muted)]"
          >
            {{ t('common.loading') }}
          </p>
          <p
            v-else-if="runs.length === 0"
            class="text-sm text-[color:var(--oc-text-muted)]"
          >
            {{ t('pages.runsEmpty') }}
          </p>
          <button
            v-for="run in runs"
            :key="run.id"
            type="button"
            class="grid gap-3 rounded-[20px] border px-4 py-4 text-left transition"
            :class="
              selectedRun?.id === run.id
                ? 'border-[color:var(--oc-accent)] bg-[color:var(--oc-accent-soft)]'
                : 'border-[color:var(--oc-border-subtle)] bg-[color:var(--oc-bg-surface)] hover:border-[color:var(--oc-accent)]/60'
            "
            @click="selectRun(run.id)"
          >
            <div class="flex items-center justify-between gap-3">
              <p class="font-medium text-[color:var(--oc-text-primary)]">
                {{ run.summary }}
              </p>
              <UiStatusBadge
                :label="t(`runStatus.${run.status}`)"
                :tone="statusTone(run.status)"
              />
            </div>
            <p class="text-sm leading-6 text-[color:var(--oc-text-secondary)]">
              {{ run.input }}
            </p>
          </button>
        </div>
      </section>
    </div>

    <section class="rounded-[24px] border border-[color:var(--oc-border-subtle)] bg-[color:var(--oc-bg-panel)] p-5">
      <div class="flex flex-wrap items-start justify-between gap-3">
        <div class="space-y-2">
          <p class="text-[11px] uppercase tracking-[0.22em] text-[color:var(--oc-text-muted)]">
            {{ t('pages.runsTimelineEyebrow') }}
          </p>
          <h2 class="font-[var(--oc-font-display)] text-2xl tracking-tight">
            {{ t('pages.runsTimelineTitle') }}
          </h2>
          <p class="text-sm leading-7 text-[color:var(--oc-text-secondary)]">
            {{ selectedRun ? selectedRun.summary : t('pages.runsTimelineEmpty') }}
          </p>
        </div>

        <UiStatusBadge
          v-if="selectedRun"
          :label="t(`interaction.${selectedRun.interactionType}`)"
          tone="accent"
        />
      </div>

      <div class="mt-5 grid gap-3">
        <p
          v-if="!selectedRun"
          class="text-sm text-[color:var(--oc-text-muted)]"
        >
          {{ t('pages.runsTimelineEmpty') }}
        </p>
        <p
          v-else-if="timelineQuery.isLoading.value"
          class="text-sm text-[color:var(--oc-text-muted)]"
        >
          {{ t('common.loading') }}
        </p>
        <div
          v-if="selectedRun"
          class="rounded-[20px] border border-[color:var(--oc-border-subtle)] bg-[color:var(--oc-bg-surface)] p-4"
        >
          <div class="flex flex-wrap items-center justify-between gap-3">
            <p class="text-sm font-medium text-[color:var(--oc-text-primary)]">
              {{ t('pages.runsSelectedRunLabel') }}: {{ selectedRun.id }}
            </p>
            <UiStatusBadge
              :label="t(`runStatus.${selectedRun.status}`)"
              :tone="statusTone(selectedRun.status)"
            />
          </div>
          <p class="mt-3 text-sm leading-6 text-[color:var(--oc-text-secondary)]">
            {{ selectedRun.input }}
          </p>
        </div>
        <ol
          v-if="timelineEvents.length > 0"
          class="grid gap-3"
        >
          <li
            v-for="event in timelineEvents"
            :key="event.id"
            class="rounded-[20px] border border-[color:var(--oc-border-subtle)] bg-[color:var(--oc-bg-surface)] p-4"
          >
            <div class="flex flex-wrap items-center justify-between gap-3">
              <p class="font-medium text-[color:var(--oc-text-primary)]">
                {{ event.type }}
              </p>
              <p class="text-xs uppercase tracking-[0.18em] text-[color:var(--oc-text-muted)]">
                {{ event.occurredAt }}
              </p>
            </div>
            <p class="mt-3 text-sm leading-6 text-[color:var(--oc-text-secondary)]">
              {{ event.summary }}
            </p>
          </li>
        </ol>
        <p
          v-else-if="selectedRun"
          class="text-sm text-[color:var(--oc-text-muted)]"
        >
          {{ t('pages.runsTimelineEmpty') }}
        </p>
      </div>
    </section>
  </div>
</template>
