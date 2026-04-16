<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'

import {
  UiBadge,
  UiButton,
  UiEmptyState,
  UiField,
  UiInput,
  UiMetricCard,
  UiPanelFrame,
  UiRecordCard,
  UiSelect,
  UiStatusCallout,
} from '@octopus/ui'

import type { AccessMemberSummary, AccessUserUpsertRequest } from '@octopus/schema'

import { useWorkspaceAccessControlStore } from '@/stores/workspace-access-control'
import { useWorkspaceStore } from '@/stores/workspace'
import { formatList } from '@/i18n/copy'

import { getStatusLabel } from './helpers'
import { useAccessControlNotifications } from './useAccessControlNotifications'

interface CreateMemberFormState {
  username: string
  displayName: string
}

const { t } = useI18n()
const router = useRouter()
const workspaceStore = useWorkspaceStore()
const accessControlStore = useWorkspaceAccessControlStore()
const { notifyError, notifySuccess } = useAccessControlNotifications('access-control.members')

const createDialogOpen = ref(false)
const assignDialogOpen = ref(false)
const selectedMemberId = ref('')
const assignTargetUserId = ref('')
const assignPresetCode = ref('')
const createPending = ref(false)
const assignPending = ref(false)
const submitError = ref('')
const feedback = ref<{
  tone: 'success' | 'error'
  message: string
} | null>(null)

const createForm = reactive<CreateMemberFormState>({
  username: '',
  displayName: '',
})

const members = computed(() =>
  [...accessControlStore.members]
    .sort((left, right) => {
      const leftLabel = left.user.displayName || left.user.username
      const rightLabel = right.user.displayName || right.user.username
      return leftLabel.localeCompare(rightLabel)
    }),
)
const memberCount = computed(() =>
  members.value.length > 0
    ? members.value.length
    : (accessControlStore.experienceSummary?.memberCount ?? 0),
)
const hasStarterWorkspace = computed(() => memberCount.value <= 1)
const presetOptions = computed(() => accessControlStore.presetCards.map(preset => ({
  label: preset.name,
  value: preset.code,
})))
const currentUserId = computed(() => accessControlStore.currentUser?.id ?? '')
const selectedMember = computed(() =>
  members.value.find(member => member.user.id === selectedMemberId.value) ?? null,
)
const assignTargetMember = computed(() =>
  members.value.find(member => member.user.id === assignTargetUserId.value) ?? null,
)

const metrics = computed(() => [
  {
    id: 'members',
    label: t('accessControl.members.metrics.members'),
    value: memberCount.value,
    helper: t('accessControl.members.metrics.membersHelper'),
  },
  {
    id: 'org-structure',
    label: t('accessControl.members.metrics.structure'),
    value: accessControlStore.experienceSummary?.hasOrgStructure
      ? t('accessControl.members.metrics.structureEnabled')
      : t('accessControl.members.metrics.structureDisabled'),
    helper: t('accessControl.members.metrics.structureHelper'),
  },
  {
    id: 'landing',
    label: t('accessControl.members.metrics.landing'),
    value: t(`accessControl.sections.${accessControlStore.recommendedAccessSection ?? 'members'}.label`),
    helper: t('accessControl.members.metrics.landingHelper'),
  },
])

watch(members, (nextMembers) => {
  if (!nextMembers.some(member => member.user.id === selectedMemberId.value)) {
    selectedMemberId.value = nextMembers[0]?.user.id ?? ''
  }
}, { immediate: true })

function navigateTo(routeName: 'workspace-access-control-access' | 'workspace-access-control-governance', query?: Record<string, string>) {
  void router.push({
    name: routeName,
    params: {
      workspaceId: workspaceStore.currentWorkspaceId,
    },
    query,
  })
}

function selectMember(memberId: string) {
  selectedMemberId.value = memberId
  feedback.value = null
}

function memberLabel(member: AccessMemberSummary | null) {
  if (!member) {
    return t('common.na')
  }

  return member.user.displayName || member.user.username || t('common.na')
}

function memberStatusLabel(member: AccessMemberSummary) {
  return getStatusLabel(t, member.user.status)
}

function memberRolesLabel(member: AccessMemberSummary) {
  if (!member.effectiveRoleNames.length) {
    return t('accessControl.members.list.noEffectiveRoles')
  }

  return formatList(member.effectiveRoleNames)
}

function memberOrgLabel(member: AccessMemberSummary) {
  return member.hasOrgAssignments
    ? t('accessControl.members.list.orgAssigned')
    : t('accessControl.members.list.orgNotAssigned')
}

function resetCreateForm() {
  createForm.username = ''
  createForm.displayName = ''
  submitError.value = ''
}

function resolvePresetCode(memberId: string) {
  const currentPresetCode = members.value.find(member => member.user.id === memberId)?.primaryPresetCode
  if (currentPresetCode && presetOptions.value.some(option => option.value === currentPresetCode)) {
    return currentPresetCode
  }
  return presetOptions.value[0]?.value ?? ''
}

function openCreateDialog() {
  resetCreateForm()
  createDialogOpen.value = true
}

function closeCreateDialog() {
  createDialogOpen.value = false
  submitError.value = ''
}

function openAssignDialog(memberId = selectedMember.value?.user.id ?? members.value[0]?.user.id ?? '') {
  if (!memberId) {
    return
  }

  assignTargetUserId.value = memberId
  assignPresetCode.value = resolvePresetCode(memberId)
  submitError.value = ''
  assignDialogOpen.value = true
}

function closeAssignDialog() {
  assignDialogOpen.value = false
  submitError.value = ''
}

function openAdvancedDetails(memberId: string) {
  selectedMemberId.value = memberId
}

function validateCreateForm() {
  if (!createForm.username.trim()) {
    return t('accessControl.members.validation.usernameRequired')
  }
  if (!createForm.displayName.trim()) {
    return t('accessControl.members.validation.displayNameRequired')
  }
  return ''
}

function validatePresetAssignment() {
  if (!assignTargetUserId.value) {
    return t('accessControl.assignment.validation.memberRequired')
  }
  if (!assignPresetCode.value) {
    return t('accessControl.assignment.validation.presetRequired')
  }
  return ''
}

async function handleCreateUser() {
  const validationError = validateCreateForm()
  if (validationError) {
    submitError.value = validationError
    return
  }

  createPending.value = true
  submitError.value = ''

  try {
    const payload: AccessUserUpsertRequest = {
      username: createForm.username.trim(),
      displayName: createForm.displayName.trim(),
      status: 'active',
    }
    const record = await accessControlStore.createUser(payload)
    selectedMemberId.value = record.id
    createDialogOpen.value = false
    const successMessage = t('accessControl.members.feedback.created', {
      member: record.displayName || record.username,
    })
    feedback.value = {
      tone: 'success',
      message: successMessage,
    }
    await notifySuccess(successMessage)
    resetCreateForm()
  } catch (cause) {
    const message = cause instanceof Error ? cause.message : t('accessControl.members.feedback.createFailed')
    submitError.value = message
    feedback.value = {
      tone: 'error',
      message: t('accessControl.members.feedback.createFailed'),
    }
    await notifyError(t('accessControl.members.feedback.createFailed'), message)
  } finally {
    createPending.value = false
  }
}

async function handleAssignPreset() {
  const validationError = validatePresetAssignment()
  if (validationError) {
    submitError.value = validationError
    return
  }

  assignPending.value = true
  submitError.value = ''

  try {
    await accessControlStore.assignUserPreset(assignTargetUserId.value, {
      presetCode: assignPresetCode.value,
    })
    selectedMemberId.value = assignTargetUserId.value
    assignDialogOpen.value = false
    const successMessage = t('accessControl.assignment.feedback.applied', {
      preset: presetOptions.value.find(option => option.value === assignPresetCode.value)?.label ?? assignPresetCode.value,
      member: memberLabel(assignTargetMember.value),
    })
    feedback.value = {
      tone: 'success',
      message: successMessage,
    }
    await notifySuccess(successMessage)
  } catch (cause) {
    const message = cause instanceof Error ? cause.message : t('accessControl.assignment.feedback.failedTitle')
    submitError.value = message
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
  <div class="space-y-4" data-testid="access-members-view">
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

    <UiPanelFrame
      variant="raised"
      padding="md"
      :title="t('accessControl.members.title')"
      :subtitle="t('accessControl.members.description')"
    >
      <template v-if="!hasStarterWorkspace" #actions>
        <UiButton data-testid="access-members-create-button" @click="openCreateDialog">
          {{ t('accessControl.members.actions.createUser') }}
        </UiButton>
        <UiButton
          variant="ghost"
          :disabled="!members.length || !presetOptions.length"
          @click="openAssignDialog()"
        >
          {{ t('accessControl.members.actions.assignPreset') }}
        </UiButton>
      </template>

      <UiEmptyState
        v-if="hasStarterWorkspace"
        :title="t('accessControl.members.empty.title')"
        :description="t('accessControl.members.empty.description')"
      >
        <template #actions>
          <UiButton size="sm" @click="openCreateDialog">
            {{ t('accessControl.members.empty.primaryAction') }}
          </UiButton>
          <UiButton
            size="sm"
            variant="ghost"
            :disabled="!members.length || !presetOptions.length"
            @click="openAssignDialog(members[0]?.user.id)"
          >
            {{ t('accessControl.members.empty.secondaryAction') }}
          </UiButton>
        </template>
      </UiEmptyState>

      <div v-else class="grid gap-4 xl:grid-cols-[minmax(0,1.25fr)_minmax(0,0.85fr)]">
        <UiPanelFrame
          variant="panel"
          padding="sm"
          :title="t('accessControl.members.directory.title')"
          :subtitle="t('accessControl.members.directory.description')"
        >
          <div class="space-y-3">
            <UiRecordCard
              v-for="member in members"
              :key="member.user.id"
              :title="memberLabel(member)"
              :description="member.user.username"
              :active="member.user.id === selectedMemberId"
              interactive
              layout="default"
              @click="selectMember(member.user.id)"
            >
              <template #eyebrow>
                {{ t('accessControl.members.list.memberEyebrow') }}
              </template>
              <template #secondary>
                <UiBadge :label="member.primaryPresetName" />
                <UiBadge :label="memberStatusLabel(member)" subtle />
                <UiBadge
                  v-if="member.user.id === currentUserId"
                  :label="t('accessControl.members.list.currentAccount')"
                  subtle
                />
              </template>
              <template #meta>
                <span class="text-[12px] text-text-tertiary">
                  {{ t('accessControl.members.list.currentAccess', { preset: member.primaryPresetName }) }}
                </span>
                <span class="text-[12px] text-text-tertiary">
                  {{ memberOrgLabel(member) }}
                </span>
                <span class="text-[12px] text-text-tertiary">
                  {{ memberRolesLabel(member) }}
                </span>
              </template>
              <template #actions>
                <UiButton
                  :data-testid="`access-members-assign-preset-${member.user.id}`"
                  size="sm"
                  variant="ghost"
                  @click.stop="openAssignDialog(member.user.id)"
                >
                  {{ t('accessControl.members.actions.assignPreset') }}
                </UiButton>
                <UiButton
                  size="sm"
                  variant="ghost"
                  @click.stop="openAdvancedDetails(member.user.id)"
                >
                  {{ t('accessControl.members.actions.openAdvanced') }}
                </UiButton>
              </template>
            </UiRecordCard>
          </div>
        </UiPanelFrame>

        <UiPanelFrame
          variant="subtle"
          padding="sm"
          :title="selectedMember ? memberLabel(selectedMember) : t('accessControl.members.detail.emptyTitle')"
          :subtitle="selectedMember ? t('accessControl.members.detail.subtitle') : t('accessControl.members.detail.emptyDescription')"
        >
          <UiEmptyState
            v-if="!selectedMember"
            :title="t('accessControl.members.detail.emptyTitle')"
            :description="t('accessControl.members.detail.emptyDescription')"
          />

          <div v-else class="space-y-4">
            <div class="flex flex-wrap gap-2">
              <UiBadge :label="selectedMember.primaryPresetName" />
              <UiBadge :label="memberStatusLabel(selectedMember)" subtle />
              <UiBadge
                v-if="selectedMember.hasOrgAssignments"
                :label="t('accessControl.members.detail.orgAssigned')"
                tone="info"
                subtle
              />
            </div>

            <div class="space-y-2">
              <p class="text-[12px] font-semibold text-text-secondary">
                {{ t('accessControl.members.detail.usernameLabel') }}
              </p>
              <p class="text-[14px] text-text-primary">
                {{ selectedMember.user.username }}
              </p>
            </div>

            <div class="space-y-2">
              <p class="text-[12px] font-semibold text-text-secondary">
                {{ t('accessControl.members.detail.primaryPresetLabel') }}
              </p>
              <p class="text-[14px] text-text-primary">
                {{ selectedMember.primaryPresetName }}
              </p>
            </div>

            <div class="space-y-2">
              <p class="text-[12px] font-semibold text-text-secondary">
                {{ t('accessControl.members.detail.orgParticipationLabel') }}
              </p>
              <p class="text-[14px] text-text-primary">
                {{ memberOrgLabel(selectedMember) }}
              </p>
            </div>

            <div class="space-y-2">
              <p class="text-[12px] font-semibold text-text-secondary">
                {{ t('accessControl.members.detail.effectiveRolesLabel') }}
              </p>
              <div class="flex flex-wrap gap-2">
                <UiBadge
                  v-for="roleName in selectedMember.effectiveRoleNames"
                  :key="roleName"
                  :label="roleName"
                  subtle
                />
                <span
                  v-if="!selectedMember.effectiveRoleNames.length"
                  class="text-[13px] text-text-tertiary"
                >
                  {{ t('accessControl.members.list.noEffectiveRoles') }}
                </span>
              </div>
            </div>

            <div class="flex flex-wrap gap-2 border-t border-border pt-3">
              <UiButton
                size="sm"
                :disabled="!presetOptions.length"
                @click="openAssignDialog(selectedMember.user.id)"
              >
                {{ t('accessControl.members.actions.assignPreset') }}
              </UiButton>
              <UiButton
                v-if="accessControlStore.accessSectionGrants.governance"
                size="sm"
                variant="ghost"
                @click="navigateTo('workspace-access-control-governance', { tab: 'organization' })"
              >
                {{ t('accessControl.members.actions.openGovernance') }}
              </UiButton>
            </div>
          </div>
        </UiPanelFrame>
      </div>
    </UiPanelFrame>

    <div
      v-if="createDialogOpen"
      data-testid="access-members-create-dialog"
      class="fixed inset-0 z-40 flex items-start justify-center bg-[var(--color-overlay)] px-4 py-16"
    >
      <div class="w-full max-w-lg">
        <UiPanelFrame
          variant="raised"
          padding="md"
          :title="t('accessControl.members.createDialog.title')"
          :subtitle="t('accessControl.members.createDialog.description')"
        >
          <form class="space-y-4" @submit.prevent="handleCreateUser">
            <UiField :label="t('accessControl.members.createDialog.usernameLabel')">
              <UiInput
                v-model="createForm.username"
                data-testid="access-members-create-username"
                :placeholder="t('accessControl.members.createDialog.usernamePlaceholder')"
              />
            </UiField>

            <UiField :label="t('accessControl.members.createDialog.displayNameLabel')">
              <UiInput
                v-model="createForm.displayName"
                data-testid="access-members-create-display-name"
                :placeholder="t('accessControl.members.createDialog.displayNamePlaceholder')"
              />
            </UiField>

            <p v-if="submitError" class="text-[13px] text-status-error">
              {{ submitError }}
            </p>

            <div class="flex justify-end gap-2 border-t border-border pt-3">
              <UiButton type="button" variant="ghost" @click="closeCreateDialog">
                {{ t('common.cancel') }}
              </UiButton>
              <UiButton
                type="submit"
                data-testid="access-members-create-submit"
                :loading="createPending"
              >
                {{ t('accessControl.members.createDialog.submit') }}
              </UiButton>
            </div>
          </form>
        </UiPanelFrame>
      </div>
    </div>

    <div
      v-if="assignDialogOpen"
      data-testid="access-members-assign-dialog"
      class="fixed inset-0 z-40 flex items-start justify-center bg-[var(--color-overlay)] px-4 py-16"
    >
      <div class="w-full max-w-lg">
        <UiPanelFrame
          variant="raised"
          padding="md"
          :title="t('accessControl.assignment.dialogTitle')"
          :subtitle="t('accessControl.assignment.dialogDescription', { member: memberLabel(assignTargetMember) })"
        >
          <form class="space-y-4" @submit.prevent="handleAssignPreset">
            <UiField :label="t('accessControl.assignment.memberLabel')">
              <UiInput :model-value="memberLabel(assignTargetMember)" readonly />
            </UiField>

            <UiField :label="t('accessControl.assignment.presetLabel')">
              <UiSelect
                v-model="assignPresetCode"
                data-testid="access-members-assign-preset-select"
                :options="presetOptions"
              />
            </UiField>

            <p v-if="submitError" class="text-[13px] text-status-error">
              {{ submitError }}
            </p>

            <div class="flex justify-end gap-2 border-t border-border pt-3">
              <UiButton type="button" variant="ghost" @click="closeAssignDialog">
                {{ t('common.cancel') }}
              </UiButton>
              <UiButton
                type="submit"
                data-testid="access-members-assign-submit"
                :loading="assignPending"
              >
                {{ t('accessControl.assignment.submit') }}
              </UiButton>
            </div>
          </form>
        </UiPanelFrame>
      </div>
    </div>
  </div>
</template>
