<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query'
import type {
  InboxItemRecord,
  InteractionResponsePayload,
  ResumeRunInput,
} from '@octopus/api-client'
import { useI18n } from 'vue-i18n'
import { UiStatusBadge, UiSurfaceCard } from '@octopus/ui'
import { useControlPlaneClient } from '@/lib/control-plane'

const { t } = useI18n()
const client = useControlPlaneClient()
const queryClient = useQueryClient()

const selectedItemId = ref<string | null>(null)
const responseText = ref('')
const goalChanged = ref(false)
const selectedValues = ref<string[]>([])
const approvalDecision = ref<'approve' | 'reject'>('approve')

const inboxQuery = useQuery({
  queryKey: ['inbox'],
  queryFn: () => client.listInboxItems(),
})

const pendingItems = computed(() =>
  (inboxQuery.data.value ?? []).filter((item) => item.status === 'pending'),
)
const selectedItem = computed<InboxItemRecord | undefined>(
  () => pendingItems.value.find((item) => item.id === selectedItemId.value) ?? pendingItems.value[0],
)

watch(
  selectedItem,
  () => {
    responseText.value = ''
    goalChanged.value = false
    selectedValues.value = []
    approvalDecision.value = 'approve'
  },
  { immediate: true },
)

const resumeMutation = useMutation({
  mutationFn: ({ runId, input }: { runId: string; input: ResumeRunInput }) =>
    client.resumeRun(runId, input),
  onSuccess: async () => {
    responseText.value = ''
    goalChanged.value = false
    selectedValues.value = []
    approvalDecision.value = 'approve'
    await queryClient.invalidateQueries({ queryKey: ['inbox'] })
    await queryClient.invalidateQueries({ queryKey: ['runs'] })
    await queryClient.invalidateQueries({ queryKey: ['run-timeline'] })
    await queryClient.invalidateQueries({ queryKey: ['audit'] })
  },
})

function selectItem(itemId: string) {
  selectedItemId.value = itemId
}

function toggleSelectedValue(value: string) {
  if (selectedValues.value.includes(value)) {
    selectedValues.value = selectedValues.value.filter((entry) => entry !== value)
    return
  }

  selectedValues.value = [...selectedValues.value, value]
}

function buildResponsePayload(item: InboxItemRecord): InteractionResponsePayload {
  switch (item.responseType) {
    case 'approval':
      return {
        type: 'approval',
        approved: approvalDecision.value === 'approve',
        text: responseText.value.trim() || undefined,
        values: [],
        goalChanged: goalChanged.value,
      }
    case 'single_select':
      return {
        type: 'single_select',
        values: selectedValues.value.slice(0, 1),
        text: responseText.value.trim() || undefined,
        goalChanged: goalChanged.value,
      }
    case 'multi_select':
      return {
        type: 'multi_select',
        values: selectedValues.value,
        text: responseText.value.trim() || undefined,
        goalChanged: goalChanged.value,
      }
    case 'text':
    default:
      return {
        type: 'text',
        values: [],
        text: responseText.value.trim() || undefined,
        goalChanged: goalChanged.value,
      }
  }
}

function handleResume() {
  if (!selectedItem.value) {
    return
  }

  const item = selectedItem.value
  const payload = buildResponsePayload(item)

  if (item.responseType === 'text' && !payload.text) {
    return
  }

  resumeMutation.mutate({
    runId: item.runId,
    input: {
      inboxItemId: item.id,
      resumeToken: item.resumeToken,
      idempotencyKey: `web-${item.id}-${Date.now()}`,
      response: payload,
    },
  })
}

function badgeTone(item: InboxItemRecord) {
  return item.kind === 'approval' ? 'warning' : 'accent'
}
</script>

<template>
  <div class="grid gap-4">
    <UiSurfaceCard
      :eyebrow="t('pages.inboxEyebrow')"
      :title="t('pages.inboxTitle')"
      :body="t('pages.inboxBody')"
    >
      <UiStatusBadge
        :label="t('status.ready')"
        tone="warning"
      />
    </UiSurfaceCard>

    <div class="grid gap-4 xl:grid-cols-[minmax(0,0.8fr),minmax(0,1.2fr)]">
      <section class="rounded-[24px] border border-[color:var(--oc-border-subtle)] bg-[color:var(--oc-bg-panel)] p-5">
        <div class="space-y-2">
          <p class="text-[11px] uppercase tracking-[0.22em] text-[color:var(--oc-text-muted)]">
            {{ t('pages.inboxQueueEyebrow') }}
          </p>
          <h2 class="font-[var(--oc-font-display)] text-2xl tracking-tight">
            {{ t('pages.inboxQueueTitle') }}
          </h2>
          <p class="text-sm leading-7 text-[color:var(--oc-text-secondary)]">
            {{ t('pages.inboxQueueBody') }}
          </p>
        </div>

        <div class="mt-5 grid gap-3">
          <p
            v-if="inboxQuery.isLoading.value"
            class="text-sm text-[color:var(--oc-text-muted)]"
          >
            {{ t('common.loading') }}
          </p>
          <p
            v-else-if="pendingItems.length === 0"
            class="text-sm text-[color:var(--oc-text-muted)]"
          >
            {{ t('pages.inboxEmpty') }}
          </p>
          <button
            v-for="item in pendingItems"
            :key="item.id"
            type="button"
            class="grid gap-3 rounded-[20px] border px-4 py-4 text-left transition"
            :class="
              selectedItem?.id === item.id
                ? 'border-[color:var(--oc-accent)] bg-[color:var(--oc-accent-soft)]'
                : 'border-[color:var(--oc-border-subtle)] bg-[color:var(--oc-bg-surface)] hover:border-[color:var(--oc-accent)]/60'
            "
            @click="selectItem(item.id)"
          >
            <div class="flex items-center justify-between gap-3">
              <p class="font-medium text-[color:var(--oc-text-primary)]">
                {{ item.title }}
              </p>
              <UiStatusBadge
                :label="t(`interaction.${item.kind}`)"
                :tone="badgeTone(item)"
              />
            </div>
            <p class="text-sm leading-6 text-[color:var(--oc-text-secondary)]">
              {{ item.prompt }}
            </p>
          </button>
        </div>
      </section>

      <section class="rounded-[24px] border border-[color:var(--oc-border-subtle)] bg-[color:var(--oc-bg-panel)] p-5">
        <div class="flex flex-wrap items-start justify-between gap-3">
          <div class="space-y-2">
            <p class="text-[11px] uppercase tracking-[0.22em] text-[color:var(--oc-text-muted)]">
              {{ t('pages.inboxDetailEyebrow') }}
            </p>
            <h2 class="font-[var(--oc-font-display)] text-2xl tracking-tight">
              {{ t('pages.inboxDetailTitle') }}
            </h2>
            <p class="text-sm leading-7 text-[color:var(--oc-text-secondary)]">
              {{ selectedItem ? selectedItem.prompt : t('pages.inboxDetailEmpty') }}
            </p>
          </div>

          <UiStatusBadge
            v-if="selectedItem"
            :label="t(`responseType.${selectedItem.responseType}`)"
            tone="accent"
          />
        </div>

        <form
          v-if="selectedItem"
          data-testid="resume-form"
          class="mt-5 grid gap-4"
          @submit.prevent="handleResume"
        >
          <label
            v-if="selectedItem.responseType === 'text' || selectedItem.responseType === 'approval'"
            class="grid gap-2"
          >
            <span class="text-sm font-medium text-[color:var(--oc-text-primary)]">
              {{ t('pages.inboxResponseLabel') }}
            </span>
            <textarea
              v-model="responseText"
              data-testid="inbox-response-text"
              rows="4"
              class="min-h-28 rounded-[18px] border border-[color:var(--oc-border-subtle)] bg-[color:var(--oc-bg-surface)] px-4 py-3 text-sm text-[color:var(--oc-text-primary)] outline-none transition focus:border-[color:var(--oc-accent)]"
              :placeholder="t('pages.inboxResponsePlaceholder')"
            />
          </label>

          <div
            v-if="selectedItem.responseType === 'single_select' || selectedItem.responseType === 'multi_select'"
            class="grid gap-3"
          >
            <p class="text-sm font-medium text-[color:var(--oc-text-primary)]">
              {{ t('pages.inboxOptionsLabel') }}
            </p>
            <label
              v-for="option in selectedItem.options"
              :key="option"
              class="flex items-center gap-3 rounded-[18px] border border-[color:var(--oc-border-subtle)] bg-[color:var(--oc-bg-surface)] px-4 py-3 text-sm text-[color:var(--oc-text-primary)]"
            >
              <input
                type="checkbox"
                class="h-4 w-4 rounded border-[color:var(--oc-border-subtle)]"
                :checked="selectedValues.includes(option)"
                @change="toggleSelectedValue(option)"
              >
              <span>{{ option }}</span>
            </label>
          </div>

          <div
            v-if="selectedItem.responseType === 'approval'"
            class="grid gap-3"
          >
            <p class="text-sm font-medium text-[color:var(--oc-text-primary)]">
              {{ t('pages.inboxApprovalLabel') }}
            </p>
            <div class="grid gap-3 md:grid-cols-2">
              <label class="flex items-center gap-3 rounded-[18px] border border-[color:var(--oc-border-subtle)] bg-[color:var(--oc-bg-surface)] px-4 py-3 text-sm text-[color:var(--oc-text-primary)]">
                <input
                  v-model="approvalDecision"
                  type="radio"
                  value="approve"
                >
                <span>{{ t('pages.inboxApprove') }}</span>
              </label>
              <label class="flex items-center gap-3 rounded-[18px] border border-[color:var(--oc-border-subtle)] bg-[color:var(--oc-bg-surface)] px-4 py-3 text-sm text-[color:var(--oc-text-primary)]">
                <input
                  v-model="approvalDecision"
                  type="radio"
                  value="reject"
                >
                <span>{{ t('pages.inboxReject') }}</span>
              </label>
            </div>
          </div>

          <label class="flex items-center gap-3 rounded-[18px] border border-[color:var(--oc-border-subtle)] bg-[color:var(--oc-bg-surface)] px-4 py-3 text-sm text-[color:var(--oc-text-primary)]">
            <input
              v-model="goalChanged"
              data-testid="goal-changed"
              type="checkbox"
              class="h-4 w-4 rounded border-[color:var(--oc-border-subtle)]"
            >
            <span>{{ t('pages.inboxGoalChangedLabel') }}</span>
          </label>

          <button
            type="submit"
            class="inline-flex items-center justify-center rounded-[18px] bg-[color:var(--oc-accent)] px-4 py-3 text-sm font-semibold text-white transition hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-50"
            :disabled="resumeMutation.isPending.value"
          >
            {{ resumeMutation.isPending.value ? t('common.submitting') : t('pages.inboxSubmit') }}
          </button>
        </form>

        <p
          v-else
          class="mt-5 text-sm text-[color:var(--oc-text-muted)]"
        >
          {{ t('pages.inboxDetailEmpty') }}
        </p>

        <div
          v-if="resumeMutation.data.value"
          class="mt-5 rounded-[20px] border border-[color:var(--oc-border-subtle)] bg-[color:var(--oc-bg-surface)] p-4"
        >
          <div class="flex flex-wrap items-center justify-between gap-3">
            <p class="font-medium text-[color:var(--oc-text-primary)]">
              {{ t('pages.inboxLatestResultTitle') }}
            </p>
            <UiStatusBadge
              :label="t(`runStatus.${resumeMutation.data.value.status}`)"
              tone="success"
            />
          </div>
          <p class="mt-3 text-sm leading-6 text-[color:var(--oc-text-secondary)]">
            {{ resumeMutation.data.value.run.summary }}
          </p>
        </div>
      </section>
    </div>
  </div>
</template>
