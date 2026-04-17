<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'

import {
  UiBadge,
  UiButton,
  UiEmptyState,
  UiField,
  UiMetricCard,
  UiPanelFrame,
  UiRecordCard,
  UiSelect,
  UiStatusCallout,
} from '@octopus/ui'

import { useWorkspaceAccessControlStore } from '@/stores/workspace-access-control'
import { useWorkspaceStore } from '@/stores/workspace'
import { formatList } from '@/i18n/copy'

import {
  getAccessCapabilityBundleName,
  getAccessPresetDescription,
  getAccessPresetName,
  getAccessPresetRecommendedFor,
  getAccessTemplateName,
} from './display-i18n'
import { useAccessControlNotifications } from './useAccessControlNotifications'

const { t } = useI18n()
const router = useRouter()
const workspaceStore = useWorkspaceStore()
const accessControlStore = useWorkspaceAccessControlStore()
const { notifyError, notifySuccess } = useAccessControlNotifications('access-control.access')

const selectedPresetCode = ref('')
const selectedMemberId = ref('')
const assignPending = ref(false)
const assignError = ref('')
const feedback = ref<{
  tone: 'success' | 'error'
  message: string
} | null>(null)

const members = computed(() =>
  [...accessControlStore.members]
    .sort((left, right) => {
      const leftLabel = left.user.displayName || left.user.username
      const rightLabel = right.user.displayName || right.user.username
      return leftLabel.localeCompare(rightLabel)
    }),
)
const selectedPreset = computed(() =>
  accessControlStore.presetCards.find(preset => preset.code === selectedPresetCode.value) ?? null,
)
const selectedMember = computed(() =>
  members.value.find(member => member.user.id === selectedMemberId.value) ?? null,
)
const memberOptions = computed(() => [
  {
    label: t('accessControl.access.assignment.memberPlaceholder'),
    value: '',
  },
  ...members.value.map(member => ({
    label: member.user.displayName || member.user.username,
    value: member.user.id,
  })),
])
const selectedPresetMembers = computed(() => {
  if (!selectedPreset.value) {
    return []
  }
  return accessControlStore.membersByPresetCode.get(selectedPreset.value.code) ?? []
})

const metrics = computed(() => [
  {
    id: 'presets',
    label: t('accessControl.access.metrics.presets'),
    value: accessControlStore.presetCards.length,
    helper: t('accessControl.access.metrics.presetsHelper'),
  },
  {
    id: 'bundles',
    label: t('accessControl.access.metrics.bundles'),
    value: accessControlStore.capabilityBundles.length,
    helper: t('accessControl.access.metrics.bundlesHelper'),
  },
  {
    id: 'roles',
    label: t('accessControl.access.metrics.customRoles'),
    value: accessControlStore.experienceSummary?.hasCustomRoles
      ? t('accessControl.access.metrics.customRolesEnabled')
      : t('accessControl.access.metrics.customRolesDisabled'),
    helper: t('accessControl.access.metrics.customRolesHelper'),
  },
])

watch(() => accessControlStore.presetCards, (presets) => {
  if (!presets.some(preset => preset.code === selectedPresetCode.value)) {
    selectedPresetCode.value = presets[0]?.code ?? ''
  }
}, { immediate: true })

watch(members, (nextMembers) => {
  if (!nextMembers.some(member => member.user.id === selectedMemberId.value)) {
    selectedMemberId.value = nextMembers[0]?.user.id ?? ''
  }
}, { immediate: true })

function openGovernance(tab = 'roles') {
  void router.push({
    name: 'workspace-access-control-governance',
    params: {
      workspaceId: workspaceStore.currentWorkspaceId,
    },
    query: {
      tab,
    },
  })
}

function selectPreset(presetCode: string) {
  selectedPresetCode.value = presetCode
  feedback.value = null
  assignError.value = ''
}

function presetMemberCountLabel(presetCode: string) {
  return t('accessControl.access.memberCount', {
    count: accessControlStore.membersByPresetCode.get(presetCode)?.length ?? 0,
  })
}

function presetName(preset: NonNullable<typeof selectedPreset.value>) {
  return getAccessPresetName(preset.code, preset.name)
}

function presetDescription(preset: NonNullable<typeof selectedPreset.value>) {
  return getAccessPresetDescription(preset)
}

function presetRecommendedFor(preset: NonNullable<typeof selectedPreset.value>) {
  return getAccessPresetRecommendedFor(preset)
}

function bundleName(bundle: { code: string, name: string }) {
  return getAccessCapabilityBundleName(bundle.code, bundle.name)
}

function templateName(template: { code: string, name: string }) {
  return getAccessTemplateName(template.code, template.name)
}

async function handleAssignPreset() {
  if (!selectedMemberId.value) {
    assignError.value = t('accessControl.assignment.validation.memberRequired')
    return
  }
  if (!selectedPreset.value) {
    assignError.value = t('accessControl.assignment.validation.presetRequired')
    return
  }

  assignPending.value = true
  assignError.value = ''

  try {
    await accessControlStore.assignUserPreset(selectedMemberId.value, {
      presetCode: selectedPreset.value.code,
    })
    const successMessage = t('accessControl.assignment.feedback.applied', {
      preset: presetName(selectedPreset.value),
      member: selectedMember.value?.user.displayName || selectedMember.value?.user.username || selectedMemberId.value,
    })
    feedback.value = {
      tone: 'success',
      message: successMessage,
    }
    await notifySuccess(successMessage)
  } catch (cause) {
    const message = cause instanceof Error ? cause.message : t('accessControl.assignment.feedback.failedTitle')
    assignError.value = message
    feedback.value = {
      tone: 'error',
      message: t('accessControl.assignment.feedback.failedTitle'),
    }
    await notifyError(t('accessControl.assignment.feedback.failedTitle'), message)
  } finally {
    assignPending.value = false
  }
}
</script>

<template>
  <div class="space-y-4" data-testid="access-permissions-view">
    <div class="grid gap-4 lg:grid-cols-3">
      <UiMetricCard
        v-for="metric in metrics"
        :key="metric.id"
        :label="metric.label"
        :value="metric.value"
        :helper="metric.helper"
      />
    </div>

    <UiStatusCallout
      v-if="feedback"
      :tone="feedback.tone"
      :title="feedback.message"
    />

    <div class="grid gap-4 xl:grid-cols-[minmax(0,1.5fr)_minmax(0,0.7fr)]">
      <UiPanelFrame
        variant="raised"
        padding="md"
        :title="t('accessControl.access.title')"
        :subtitle="t('accessControl.access.description')"
      >
        <UiEmptyState
          v-if="!accessControlStore.presetCards.length"
          :title="t('accessControl.access.empty.title')"
          :description="t('accessControl.access.empty.description')"
        />

        <div v-else class="grid gap-4 lg:grid-cols-2">
          <UiRecordCard
            v-for="preset in accessControlStore.presetCards"
            :key="preset.code"
            :title="getAccessPresetName(preset.code, preset.name)"
            :description="getAccessPresetDescription(preset)"
            :test-id="`access-preset-card-${preset.code}`"
            :active="preset.code === selectedPresetCode"
            interactive
            @click="selectPreset(preset.code)"
          >
            <template #eyebrow>
              {{ t('accessControl.access.presetEyebrow') }}
            </template>
            <template #secondary>
              <UiBadge :label="getAccessPresetRecommendedFor(preset)" />
              <UiBadge :label="t('accessControl.access.templateCount', { count: preset.templates.length })" subtle />
              <UiBadge :label="presetMemberCountLabel(preset.code)" subtle />
            </template>
            <div class="space-y-2">
              <div class="space-y-1">
                <p class="text-[12px] font-medium text-text-secondary">
                  {{ t('accessControl.access.includedBundles') }}
                </p>
                <div class="flex flex-wrap gap-2">
                  <UiBadge
                    v-for="bundle in preset.capabilityBundles"
                    :key="bundle.code"
                    :label="bundleName(bundle)"
                    subtle
                  />
                </div>
              </div>
              <div class="space-y-1">
                <p class="text-[12px] font-medium text-text-secondary">
                  {{ t('accessControl.access.managedTemplates') }}
                </p>
                <div class="flex flex-wrap gap-2">
                  <UiBadge
                    v-for="template in preset.templates"
                    :key="template.code"
                    :label="templateName(template)"
                    subtle
                  />
                </div>
              </div>
            </div>
          </UiRecordCard>
        </div>
      </UiPanelFrame>

      <UiPanelFrame
        variant="subtle"
        padding="sm"
        :title="selectedPreset ? presetName(selectedPreset) : t('accessControl.access.assignment.emptyTitle')"
        :subtitle="selectedPreset ? presetDescription(selectedPreset) : t('accessControl.access.assignment.emptyDescription')"
      >
        <UiEmptyState
          v-if="!selectedPreset"
          :title="t('accessControl.access.assignment.emptyTitle')"
          :description="t('accessControl.access.assignment.emptyDescription')"
        />

        <div v-else class="space-y-4">
          <div class="space-y-2">
            <p class="text-[12px] font-semibold text-text-secondary">
              {{ t('accessControl.access.assignment.audienceLabel') }}
            </p>
            <p class="text-[14px] text-text-primary">
              {{ presetRecommendedFor(selectedPreset) }}
            </p>
          </div>

          <div class="space-y-2">
            <p class="text-[12px] font-semibold text-text-secondary">
              {{ t('accessControl.access.includedBundles') }}
            </p>
            <div class="flex flex-wrap gap-2">
              <UiBadge
                v-for="bundle in selectedPreset.capabilityBundles"
                :key="bundle.code"
                :label="bundleName(bundle)"
                subtle
              />
            </div>
          </div>

          <div class="space-y-2">
            <p class="text-[12px] font-semibold text-text-secondary">
              {{ t('accessControl.access.managedTemplates') }}
            </p>
            <div class="flex flex-wrap gap-2">
              <UiBadge
                v-for="template in selectedPreset.templates"
                :key="template.code"
                :label="templateName(template)"
                subtle
              />
            </div>
          </div>

          <div class="space-y-2">
            <p class="text-[12px] font-semibold text-text-secondary">
              {{ t('accessControl.access.assignment.appliedMembersLabel') }}
            </p>
            <p class="text-[13px] leading-relaxed text-text-secondary">
              {{
                selectedPresetMembers.length
                  ? formatList(selectedPresetMembers.map(member => member.user.displayName || member.user.username))
                  : t('accessControl.access.assignment.noAppliedMembers')
              }}
            </p>
          </div>

          <div class="space-y-3 border-t border-border pt-3">
            <UiField :label="t('accessControl.assignment.memberLabel')">
              <UiSelect
                v-model="selectedMemberId"
                data-testid="access-assign-member-select"
                :options="memberOptions"
              />
            </UiField>

            <p v-if="assignError" class="text-[13px] text-status-error">
              {{ assignError }}
            </p>

            <div class="flex flex-wrap gap-2">
              <UiButton
                data-testid="access-assign-submit"
                :loading="assignPending"
                @click="handleAssignPreset"
              >
                {{ t('accessControl.assignment.submit') }}
              </UiButton>
              <UiButton
                v-if="accessControlStore.accessSectionGrants.governance"
                variant="ghost"
                @click="openGovernance()"
              >
                {{ t('accessControl.access.assignment.openRoles') }}
              </UiButton>
            </div>
          </div>
        </div>
      </UiPanelFrame>
    </div>
  </div>
</template>
