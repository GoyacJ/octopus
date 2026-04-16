<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useForm } from 'vee-validate'
import { toTypedSchema } from '@vee-validate/zod'
import { z } from 'zod'

import { UiBadge, UiButton, UiCodeEditor, UiEmptyState, UiField, UiInput, UiMetricCard, UiRecordCard, UiStatusCallout } from '@octopus/ui'
import type { AvatarUploadPayload } from '@octopus/schema'

import { enumLabel } from '@/i18n/copy'
import { getMenuDefinition } from '@/navigation/menuRegistry'
import { useUserProfileStore } from '@/stores/user-profile'
import { useWorkspaceAccessControlStore } from '@/stores/workspace-access-control'
import { useWorkspaceStore } from '@/stores/workspace'
import * as tauriClient from '@/tauri/client'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const userProfileStore = useUserProfileStore()
const workspaceAccessControlStore = useWorkspaceAccessControlStore()
const workspaceStore = useWorkspaceStore()

const currentUser = computed(() => userProfileStore.currentUser)
const runtimeConfig = computed(() => userProfileStore.runtimeConfig)
const runtimeSource = computed(() => runtimeConfig.value?.sources.filter(source => source.scope === 'user').at(-1))
const runtimeEffectivePreview = computed(() => JSON.stringify(runtimeConfig.value?.effectiveConfig ?? {}, null, 2))
const accessRoleNames = computed(() => workspaceAccessControlStore.currentRoleNames)
const accessPermissionNames = computed(() => {
  const definitionsByCode = new Map(
    workspaceAccessControlStore.permissionDefinitions.map(permission => [permission.code, permission.name]),
  )
  return (workspaceAccessControlStore.authorization?.effectivePermissionCodes ?? [])
    .map(code => definitionsByCode.get(code) ?? code)
})
const accessMenuLabels = computed(() =>
  workspaceAccessControlStore.currentVisibleMenus.map((menu) => {
    const definition = getMenuDefinition(menu.id)
    return definition ? t(definition.labelKey) : menu.label
  }),
)
const metrics = computed(() => [
  { id: 'roles', label: t('personalCenter.profile.metrics.roleCount'), value: String(workspaceAccessControlStore.currentRoleNames.length) },
  { id: 'permissions', label: t('personalCenter.profile.metrics.permissionCount'), value: String(accessPermissionNames.value.length) },
  { id: 'menus', label: t('personalCenter.profile.metrics.menuCount'), value: String(workspaceAccessControlStore.currentVisibleMenus.length) },
])
const profileSuccessMessage = ref('')
const passwordSuccessMessage = ref('')
const avatarFallback = computed(() => (currentUser.value?.displayName || currentUser.value?.username || '?').slice(0, 1))
const pendingAvatarUpload = ref<AvatarUploadPayload | null>(null)
const pendingAvatarFileName = ref('')
const removeAvatar = ref(false)
const profileAvatarPreview = computed(() => {
  if (pendingAvatarUpload.value) {
    return `data:${pendingAvatarUpload.value.contentType};base64,${pendingAvatarUpload.value.dataBase64}`
  }

  if (removeAvatar.value) {
    return ''
  }

  return currentUser.value?.avatar ?? ''
})
const profileAvatarFileLabel = computed(() => {
  if (pendingAvatarFileName.value) {
    return pendingAvatarFileName.value
  }

  if (removeAvatar.value || !currentUser.value?.avatar) {
    return t('personalCenter.profile.edit.hints.avatarPendingEmpty')
  }

  return t('personalCenter.profile.edit.hints.avatarCurrent')
})

const profileSchema = toTypedSchema(z.object({
  displayName: z.string().trim().min(1, t('personalCenter.profile.validation.displayNameRequired')),
  username: z.string().trim().min(1, t('personalCenter.profile.validation.usernameRequired')),
}))

const {
  defineField: defineProfileField,
  errors: profileErrors,
  handleSubmit: handleProfileSubmit,
  resetForm: resetProfileForm,
} = useForm({
  validationSchema: profileSchema,
  initialValues: {
    displayName: '',
    username: '',
  },
})

const [profileDisplayName] = defineProfileField('displayName')
const [profileUsername] = defineProfileField('username')

const passwordSchema = toTypedSchema(z.object({
  currentPassword: z.string().min(1, t('personalCenter.profile.validation.currentPasswordRequired')),
  newPassword: z.string().min(1, t('personalCenter.profile.validation.newPasswordRequired')),
  confirmPassword: z.string().min(1, t('personalCenter.profile.validation.confirmPasswordRequired')),
}).superRefine((value, ctx) => {
  if (value.newPassword !== value.confirmPassword) {
    ctx.addIssue({
      code: z.ZodIssueCode.custom,
      path: ['confirmPassword'],
      message: t('personalCenter.profile.validation.passwordMismatch'),
    })
  }
  if (value.currentPassword && value.newPassword && value.currentPassword === value.newPassword) {
    ctx.addIssue({
      code: z.ZodIssueCode.custom,
      path: ['newPassword'],
      message: t('personalCenter.profile.validation.passwordUnchanged'),
    })
  }
}))

const {
  defineField: definePasswordField,
  errors: passwordErrors,
  handleSubmit: handlePasswordSubmit,
  resetForm: resetPasswordForm,
} = useForm({
  validationSchema: passwordSchema,
  initialValues: {
    currentPassword: '',
    newPassword: '',
    confirmPassword: '',
  },
})

const [currentPassword] = definePasswordField('currentPassword')
const [newPassword] = definePasswordField('newPassword')
const [confirmPassword] = definePasswordField('confirmPassword')

watch(
  () => currentUser.value,
  (user) => {
    if (!user) {
      return
    }
    resetProfileForm({
      values: {
        displayName: user.displayName,
        username: user.username,
      },
    })
    pendingAvatarUpload.value = null
    pendingAvatarFileName.value = ''
    removeAvatar.value = false
  },
  { immediate: true },
)

watch(
  () => currentUser.value?.id ?? '',
  (userId) => {
    if (!userId) {
      return
    }
    void userProfileStore.loadCurrentUserRuntimeConfig()
  },
  { immediate: true },
)

const submitProfile = handleProfileSubmit(async (values) => {
  profileSuccessMessage.value = ''
  try {
    await userProfileStore.updateCurrentUserProfile({
      displayName: values.displayName.trim(),
      username: values.username.trim(),
      avatar: pendingAvatarUpload.value ?? undefined,
      removeAvatar: removeAvatar.value || undefined,
    })
    resetProfileForm({
      values: {
        displayName: currentUser.value?.displayName ?? values.displayName.trim(),
        username: currentUser.value?.username ?? values.username.trim(),
      },
    })
    pendingAvatarUpload.value = null
    pendingAvatarFileName.value = ''
    removeAvatar.value = false
    profileSuccessMessage.value = t('personalCenter.profile.feedback.profileSaved')
  } catch {
    profileSuccessMessage.value = ''
  }
})

const submitPassword = handlePasswordSubmit(async (values) => {
  passwordSuccessMessage.value = ''
  try {
    await userProfileStore.changeCurrentUserPassword({
      currentPassword: values.currentPassword,
      newPassword: values.newPassword,
      confirmPassword: values.confirmPassword,
    })
    resetPasswordForm({
      values: {
        currentPassword: '',
        newPassword: '',
        confirmPassword: '',
      },
    })
    passwordSuccessMessage.value = t('personalCenter.profile.feedback.passwordUpdated')
  } catch {
    passwordSuccessMessage.value = ''
  }
})

function resetProfileValues() {
  if (!currentUser.value) {
    return
  }
  profileSuccessMessage.value = ''
  resetProfileForm({
    values: {
      displayName: currentUser.value.displayName,
      username: currentUser.value.username,
    },
  })
  pendingAvatarUpload.value = null
  pendingAvatarFileName.value = ''
  removeAvatar.value = false
}

async function pickAvatar() {
  const picked = await tauriClient.pickAvatarImage()
  if (!picked) {
    return
  }

  pendingAvatarUpload.value = picked
  pendingAvatarFileName.value = picked.fileName
  removeAvatar.value = false
  profileSuccessMessage.value = ''
}

function clearAvatar() {
  pendingAvatarUpload.value = null
  pendingAvatarFileName.value = ''
  removeAvatar.value = true
  profileSuccessMessage.value = ''
}

function navigateToAccessTab(name: 'workspace-access-control-access' | 'workspace-access-control-governance') {
  const workspaceId = typeof route.params.workspaceId === 'string'
    ? route.params.workspaceId
    : (userProfileStore.workspaceId || workspaceStore.currentWorkspaceId)
  if (!workspaceId) {
    return
  }
  void router.push({
    name,
    params: {
      workspaceId,
    },
  })
}
</script>

<template>
  <div data-testid="personal-center-profile-view" class="space-y-8">
    <div v-if="currentUser" class="grid gap-4 md:grid-cols-3">
      <UiMetricCard v-for="metric in metrics" :key="metric.id" :label="metric.label" :value="metric.value" />
    </div>

    <UiRecordCard
      v-if="currentUser"
      :title="currentUser.displayName"
      :description="currentUser.username"
    >
      <template #leading>
        <div class="flex h-12 w-12 items-center justify-center overflow-hidden rounded-full border border-border/60 bg-accent text-sm font-semibold uppercase text-text-secondary">
          <img v-if="currentUser.avatar" :src="currentUser.avatar" alt="" class="h-full w-full object-cover" data-testid="profile-avatar-image">
          <span v-else data-testid="profile-avatar-fallback">{{ avatarFallback }}</span>
        </div>
      </template>
      <template #badges>
        <UiBadge :label="enumLabel('recordStatus', currentUser.status)" subtle />
        <UiBadge :label="enumLabel('passwordState', currentUser.passwordState)" subtle />
        <UiBadge v-for="roleName in workspaceAccessControlStore.currentRoleNames" :key="roleName" :label="roleName" subtle />
      </template>
      <template #meta>
        <span class="text-xs text-text-tertiary">{{ userProfileStore.workspaceId || workspaceStore.currentWorkspaceId }}</span>
      </template>
    </UiRecordCard>

    <div v-if="userProfileStore.alerts.length" class="space-y-3">
      <UiRecordCard
        v-for="alert in userProfileStore.alerts"
        :key="alert.id"
        :title="alert.title"
        :description="alert.description"
      >
        <template #badges>
          <UiBadge :label="enumLabel('riskLevel', alert.severity)" subtle />
        </template>
      </UiRecordCard>
    </div>

    <UiRecordCard
      v-if="currentUser"
      :title="t('personalCenter.profile.access.title')"
      :description="t('personalCenter.profile.access.description')"
      test-id="profile-access-card"
    >
      <div class="grid gap-4 md:grid-cols-3">
        <section data-testid="profile-access-roles" class="space-y-3">
          <div class="flex items-start justify-between gap-3">
            <div class="space-y-1">
              <p class="text-[11px] font-semibold uppercase tracking-[0.24em] text-text-tertiary">
                {{ t('personalCenter.profile.access.roles') }}
              </p>
              <p class="text-sm text-text-secondary">
                {{ t('personalCenter.profile.access.rolesDescription') }}
              </p>
            </div>
            <UiButton
              type="button"
              variant="ghost"
              size="sm"
              data-testid="profile-access-roles-link"
              @click="navigateToAccessTab('workspace-access-control-access')"
            >
              {{ t('personalCenter.profile.access.openRoles') }}
            </UiButton>
          </div>
          <div class="flex flex-wrap gap-2">
            <UiBadge v-for="roleName in accessRoleNames" :key="roleName" :label="roleName" />
            <UiBadge
              v-if="!accessRoleNames.length"
              :label="t('personalCenter.profile.access.emptyRoles')"
              subtle
            />
          </div>
        </section>

        <section data-testid="profile-access-permissions" class="space-y-3">
          <div class="flex items-start justify-between gap-3">
            <div class="space-y-1">
              <p class="text-[11px] font-semibold uppercase tracking-[0.24em] text-text-tertiary">
                {{ t('personalCenter.profile.access.permissions') }}
              </p>
              <p class="text-sm text-text-secondary">
                {{ t('personalCenter.profile.access.permissionsDescription') }}
              </p>
            </div>
            <UiButton
              type="button"
              variant="ghost"
              size="sm"
              data-testid="profile-access-permissions-link"
              @click="navigateToAccessTab('workspace-access-control-governance')"
            >
              {{ t('personalCenter.profile.access.openPermissions') }}
            </UiButton>
          </div>
          <div class="flex flex-wrap gap-2">
            <UiBadge v-for="permissionName in accessPermissionNames" :key="permissionName" :label="permissionName" />
            <UiBadge
              v-if="!accessPermissionNames.length"
              :label="t('personalCenter.profile.access.emptyPermissions')"
              subtle
            />
          </div>
        </section>

        <section data-testid="profile-access-menus" class="space-y-3">
          <div class="flex items-start justify-between gap-3">
            <div class="space-y-1">
              <p class="text-[11px] font-semibold uppercase tracking-[0.24em] text-text-tertiary">
                {{ t('personalCenter.profile.access.menus') }}
              </p>
              <p class="text-sm text-text-secondary">
                {{ t('personalCenter.profile.access.menusDescription') }}
              </p>
            </div>
            <UiButton
              type="button"
              variant="ghost"
              size="sm"
              data-testid="profile-access-menus-link"
              @click="navigateToAccessTab('workspace-access-control-governance')"
            >
              {{ t('personalCenter.profile.access.openMenus') }}
            </UiButton>
          </div>
          <div class="flex flex-wrap gap-2">
            <UiBadge v-for="menuLabel in accessMenuLabels" :key="menuLabel" :label="menuLabel" />
            <UiBadge
              v-if="!accessMenuLabels.length"
              :label="t('personalCenter.profile.access.emptyMenus')"
              subtle
            />
          </div>
        </section>
      </div>
    </UiRecordCard>

    <UiRecordCard
      v-if="currentUser"
      :title="t('personalCenter.profile.edit.title')"
      :description="t('personalCenter.profile.edit.description')"
      test-id="user-profile-edit-card"
    >
      <form class="space-y-4" data-testid="profile-edit-form" @submit.prevent="submitProfile">
        <div class="grid gap-4 md:grid-cols-2">
          <div>
            <UiField :label="t('personalCenter.profile.edit.fields.displayName')" :hint="profileErrors.displayName">
              <UiInput v-model="profileDisplayName" data-testid="profile-display-name-input" autocomplete="nickname" />
            </UiField>
          </div>

          <div>
            <UiField :label="t('personalCenter.profile.edit.fields.username')" :hint="profileErrors.username">
              <UiInput v-model="profileUsername" data-testid="profile-username-input" autocomplete="username" />
            </UiField>
          </div>
        </div>

        <div>
          <UiField :label="t('personalCenter.profile.edit.fields.avatar')" :hint="t('personalCenter.profile.edit.hints.avatar')">
            <div class="flex items-center gap-3">
              <div class="flex h-12 w-12 shrink-0 items-center justify-center overflow-hidden rounded-full border border-border/60 bg-accent text-xs font-semibold uppercase text-text-secondary">
                <img v-if="profileAvatarPreview" :src="profileAvatarPreview" alt="" class="h-full w-full object-cover">
                <span v-else data-testid="profile-avatar-fallback">{{ avatarFallback }}</span>
              </div>
              <div class="grid min-w-0 flex-1 gap-2 sm:grid-cols-[minmax(0,1fr)_auto_auto]">
                <div
                  data-testid="profile-avatar-file-label"
                  class="min-w-0 truncate rounded-md border border-border bg-surface px-3 py-2 text-sm text-text-secondary"
                >
                  {{ profileAvatarFileLabel }}
                </div>
                <UiButton
                  type="button"
                  variant="ghost"
                  data-testid="profile-avatar-pick-button"
                  @click="pickAvatar"
                >
                  {{ t('personalCenter.profile.edit.actions.pickAvatar') }}
                </UiButton>
                <UiButton
                  type="button"
                  variant="ghost"
                  data-testid="profile-avatar-clear-button"
                  :disabled="!currentUser.avatar && !pendingAvatarUpload"
                  @click="clearAvatar"
                >
                  {{ t('personalCenter.profile.edit.actions.clearAvatar') }}
                </UiButton>
              </div>
            </div>
          </UiField>
        </div>

        <UiStatusCallout
          v-if="userProfileStore.profileError"
          tone="error"
          :description="userProfileStore.profileError"
        />
        <UiStatusCallout
          v-if="profileSuccessMessage"
          tone="success"
          :description="profileSuccessMessage"
        />

        <div class="flex items-center justify-end gap-2">
          <UiButton
            type="button"
            variant="ghost"
            data-testid="profile-reset-button"
            :disabled="userProfileStore.profileSaving"
            @click="resetProfileValues"
          >
            {{ t('personalCenter.profile.edit.actions.reset') }}
          </UiButton>
          <UiButton
            data-testid="profile-save-button"
            type="submit"
            :loading="userProfileStore.profileSaving"
          >
            {{ t('personalCenter.profile.edit.actions.save') }}
          </UiButton>
        </div>
      </form>
    </UiRecordCard>

    <UiRecordCard
      v-if="currentUser"
      :title="t('personalCenter.profile.password.title')"
      :description="t('personalCenter.profile.password.description')"
      test-id="user-profile-password-card"
    >
      <form class="space-y-4" data-testid="profile-password-form" @submit.prevent="submitPassword">
        <div class="grid gap-4 md:grid-cols-3">
          <div>
            <UiField :label="t('personalCenter.profile.password.fields.currentPassword')" :hint="passwordErrors.currentPassword">
              <UiInput
                v-model="currentPassword"
                data-testid="profile-current-password-input"
                type="password"
                autocomplete="current-password"
              />
            </UiField>
          </div>

          <div>
            <UiField :label="t('personalCenter.profile.password.fields.newPassword')" :hint="passwordErrors.newPassword || t('personalCenter.profile.password.hints.newPassword')">
              <UiInput
                v-model="newPassword"
                data-testid="profile-new-password-input"
                type="password"
                autocomplete="new-password"
              />
            </UiField>
          </div>

          <div>
            <UiField :label="t('personalCenter.profile.password.fields.confirmPassword')" :hint="passwordErrors.confirmPassword">
              <UiInput
                v-model="confirmPassword"
                data-testid="profile-confirm-password-input"
                type="password"
                autocomplete="new-password"
              />
            </UiField>
          </div>
        </div>

        <UiStatusCallout
          v-if="userProfileStore.passwordError"
          tone="error"
          :description="userProfileStore.passwordError"
        />
        <UiStatusCallout
          v-if="passwordSuccessMessage"
          tone="success"
          :description="passwordSuccessMessage"
        />

        <div class="flex items-center justify-end">
          <UiButton
            data-testid="profile-password-submit-button"
            type="submit"
            :loading="userProfileStore.passwordSaving"
          >
            {{ t('personalCenter.profile.password.actions.submit') }}
          </UiButton>
        </div>
      </form>
    </UiRecordCard>

    <UiRecordCard
      v-if="currentUser"
      :title="t('personalCenter.profile.runtime.title')"
      :description="t('personalCenter.profile.runtime.description')"
      test-id="user-runtime-editor"
    >
      <template #eyebrow>
        user
      </template>
      <template #badges>
        <UiBadge
          :label="userProfileStore.runtimeValidation?.valid ? t('settings.runtime.validation.valid') : t('settings.runtime.validation.idle')"
          :tone="userProfileStore.runtimeValidation?.valid ? 'success' : 'default'"
        />
        <UiBadge
          :label="runtimeSource?.loaded ? t('settings.runtime.sourceStatuses.loaded') : t('settings.runtime.sourceStatuses.missing')"
          :tone="runtimeSource?.loaded ? 'success' : 'warning'"
        />
      </template>

      <div class="space-y-3">
        <UiCodeEditor
          language="json"
          theme="octopus"
          :model-value="userProfileStore.runtimeDraft"
          @update:model-value="userProfileStore.setCurrentUserRuntimeDraft($event)"
        />

        <UiStatusCallout
          v-if="userProfileStore.runtimeValidation?.errors.length"
          tone="error"
          :description="userProfileStore.runtimeValidation.errors.join(' ')"
        />
      </div>

      <template #meta>
        <span class="text-[11px] uppercase tracking-[0.24em] text-text-tertiary">
          {{ t('settings.runtime.sourcePath') }}
        </span>
        <span class="min-w-0 truncate font-mono text-[12px] text-text-secondary">
          {{ runtimeSource?.displayPath ?? t('common.na') }}
        </span>
      </template>
      <template #actions>
        <UiButton
          variant="ghost"
          size="sm"
          :disabled="userProfileStore.runtimeValidating || userProfileStore.runtimeSaving"
          @click="userProfileStore.validateCurrentUserRuntimeConfig()"
        >
          {{ t('settings.runtime.actions.validate') }}
        </UiButton>
        <UiButton
          size="sm"
          :disabled="userProfileStore.runtimeSaving"
          @click="userProfileStore.saveCurrentUserRuntimeConfig()"
        >
          {{ t('settings.runtime.actions.save') }}
        </UiButton>
      </template>
    </UiRecordCard>

    <UiRecordCard
      v-if="currentUser"
      :title="t('personalCenter.profile.runtime.effectiveTitle')"
      :description="t('personalCenter.profile.runtime.effectiveDescription')"
      test-id="user-runtime-effective-preview"
    >
      <UiCodeEditor
        language="json"
        theme="octopus"
        readonly
        :model-value="runtimeEffectivePreview"
      />
    </UiRecordCard>

    <UiEmptyState
      v-if="!currentUser"
      :title="t('personalCenter.profile.emptyTitle')"
      :description="t('personalCenter.profile.emptyDescription')"
    />
  </div>
</template>
