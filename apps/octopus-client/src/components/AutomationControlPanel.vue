<script setup lang="ts">
import { storeToRefs } from 'pinia'
import { computed, reactive } from 'vue'
import { useI18n } from 'vue-i18n'

import type { AutomationCreateRequest, TriggerDeliveryRequest, TriggerSource } from '@octopus/contracts'

import { useRuntimeControlStore } from '@/stores/useRuntimeControlStore'

const { t } = useI18n()
const runtimeStore = useRuntimeControlStore()
const { activeTriggerId, automations, isCreatingAutomation } = storeToRefs(runtimeStore)

const formState = reactive({
  workspaceId: 'workspace-alpha',
  projectId: 'project-alpha',
  name: '',
  triggerSource: 'manual_event' as TriggerSource,
  requestedBy: 'operator-1',
  requiresApproval: true,
  deliveryDedupeKey: '',
})

const primaryAutomation = computed(() => automations.value[0] ?? null)
const isDeliveringPrimary = computed(
  () => primaryAutomation.value && activeTriggerId.value === primaryAutomation.value.trigger.id,
)

const createAutomation = async () => {
  const payload: AutomationCreateRequest = {
    workspace_id: formState.workspaceId,
    project_id: formState.projectId,
    name: formState.name,
    trigger_source: formState.triggerSource,
    requested_by: formState.requestedBy,
    requires_approval: formState.requiresApproval,
  }

  try {
    await runtimeStore.createAutomation(payload)
    formState.deliveryDedupeKey = ''
  } catch {
    // The store owns the visible error state.
  }
}

const deliverPrimaryTrigger = async () => {
  if (!primaryAutomation.value) {
    return
  }

  const payload: TriggerDeliveryRequest = {
    trigger_id: primaryAutomation.value.trigger.id,
    dedupe_key: formState.deliveryDedupeKey,
    requested_by: formState.requestedBy,
    title: null,
    description: `Trigger delivery ${formState.deliveryDedupeKey}`,
  }

  try {
    await runtimeStore.deliverTrigger(payload)
  } catch {
    // The store owns the visible error state.
  }
}
</script>

<template>
  <article class="rounded-[32px] border border-[var(--border-muted)] bg-[var(--surface-panel)] p-7 shadow-sm">
    <div class="flex items-start justify-between gap-4">
      <div>
        <p class="text-xs uppercase tracking-[0.24em] text-[var(--text-muted)]">{{ t('automation.eyebrow') }}</p>
        <h2 class="mt-3 text-2xl font-semibold leading-tight">{{ t('automation.title') }}</h2>
        <p class="mt-3 max-w-2xl text-sm leading-6 text-[var(--text-muted)]">{{ t('automation.subtitle') }}</p>
      </div>
      <span class="rounded-full bg-[var(--surface-elevated)] px-3 py-1 text-xs font-medium text-[var(--text-muted)]">
        {{ primaryAutomation?.automation.state ?? 'draft' }}
      </span>
    </div>

    <div class="mt-6 grid gap-4">
      <label class="grid gap-2 text-sm">
        <span class="font-medium">{{ t('automation.fields.name') }}</span>
        <input
          data-test="automation-name"
          v-model="formState.name"
          class="rounded-2xl border border-[var(--border-muted)] bg-[var(--surface-elevated)] px-4 py-3 outline-none transition focus:border-[var(--accent-primary)]"
          type="text"
        >
      </label>

      <label class="grid gap-2 text-sm">
        <span class="font-medium">{{ t('automation.fields.source') }}</span>
        <select
          v-model="formState.triggerSource"
          class="rounded-2xl border border-[var(--border-muted)] bg-[var(--surface-elevated)] px-4 py-3 outline-none transition focus:border-[var(--accent-primary)]"
        >
          <option value="cron">cron</option>
          <option value="webhook">webhook</option>
          <option value="manual_event">manual_event</option>
          <option value="mcp_event">mcp_event</option>
        </select>
      </label>

      <button
        data-test="automation-create"
        class="rounded-full bg-[var(--accent-primary)] px-4 py-2 text-sm font-medium text-white transition hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-60"
        :disabled="isCreatingAutomation || !formState.name.trim()"
        type="button"
        @click="createAutomation"
      >
        {{ isCreatingAutomation ? t('automation.actions.creating') : t('automation.actions.create') }}
      </button>
    </div>

    <div class="mt-6 rounded-[28px] border border-[var(--border-muted)] bg-[var(--surface-elevated)] p-5">
      <div class="flex items-center justify-between gap-3">
        <div>
          <p class="text-sm font-medium">{{ primaryAutomation?.automation.name ?? t('automation.empty') }}</p>
          <p v-if="primaryAutomation" class="mt-1 text-sm text-[var(--text-muted)]">
            {{ t('automation.summary.trigger') }}: {{ primaryAutomation.trigger.source_type }}
          </p>
        </div>
        <p v-if="primaryAutomation" class="text-xs uppercase tracking-[0.18em] text-[var(--text-muted)]">
          {{ t('automation.summary.state') }}: {{ primaryAutomation.automation.state }}
        </p>
      </div>

      <ul v-if="automations.length" class="mt-4 space-y-3 text-sm">
        <li
          v-for="entry in automations"
          :key="entry.automation.id"
          class="rounded-2xl border border-[var(--border-muted)] bg-[var(--surface-panel)] px-4 py-3"
        >
          <div class="flex items-center justify-between gap-3">
            <div>
              <p class="font-medium">{{ entry.automation.name }}</p>
              <p class="mt-1 text-[var(--text-muted)]">{{ entry.trigger.source_type }}</p>
            </div>
            <p class="text-xs uppercase tracking-[0.18em] text-[var(--text-muted)]">
              {{ entry.latest_delivery?.state ?? 'idle' }}
            </p>
          </div>
        </li>
      </ul>
    </div>

    <div class="mt-6 grid gap-4">
      <label class="grid gap-2 text-sm">
        <span class="font-medium">{{ t('automation.fields.dedupeKey') }}</span>
        <input
          data-test="delivery-dedupe-key"
          v-model="formState.deliveryDedupeKey"
          class="rounded-2xl border border-[var(--border-muted)] bg-[var(--surface-elevated)] px-4 py-3 outline-none transition focus:border-[var(--accent-primary)]"
          type="text"
        >
      </label>

      <button
        data-test="trigger-deliver"
        class="rounded-full border border-[var(--border-muted)] px-4 py-2 text-sm transition hover:bg-[var(--surface-elevated)] disabled:cursor-not-allowed disabled:opacity-60"
        :disabled="!primaryAutomation || !formState.deliveryDedupeKey.trim() || !!isDeliveringPrimary"
        type="button"
        @click="deliverPrimaryTrigger"
      >
        {{ isDeliveringPrimary ? t('automation.actions.delivering') : t('automation.actions.deliver') }}
      </button>
    </div>
  </article>
</template>
